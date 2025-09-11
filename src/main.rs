mod search;
mod types;

use anyhow::{anyhow, bail};
use clap::{Parser, Subcommand, ValueEnum};
use mysql::*;
use regex::Regex;
use rmcp::transport::{
    sse_server::SseServer,
    streamable_http_server::{StreamableHttpService, session::local::LocalSessionManager},
};
use rustls::crypto::{CryptoProvider, ring::default_provider};
use search::AgenticSearchServer;
use std::{env, path::PathBuf};
use tracing::{error, info};
use tracing_subscriber::{self, layer::SubscriberExt, util::SubscriberInitExt};

const DEFAULT_SOCKET_ADDR: &str = "127.0.0.1:8009";
const DEFAULT_QDRANT_BASE_URL: &str = "http://127.0.0.1:6333";

#[derive(Parser, Debug)]
#[command(author, version, about = "Cardea Agentic Search MCP server")]
struct Args {
    /// Socket address to bind to
    #[arg(short, long, default_value = DEFAULT_SOCKET_ADDR)]
    socket_addr: String,
    /// Transport type to use
    #[arg(short, long, value_enum, default_value = "stream-http")]
    transport: TransportType,
    /// Search mode to enable
    #[command(subcommand)]
    search_mode: SearchMode,
}

#[derive(Subcommand, Debug)]
enum SearchMode {
    /// Enable vector search only
    Qdrant {
        /// Name of the collection to search in Qdrant (can be overridden by QDRANT_COLLECTION env var)
        #[arg(long, required = false)]
        qdrant_collection: Option<String>,
        /// The name of the field in the payload that contains the source of the document (can be overridden by QDRANT_PAYLOAD_FIELD env var)
        #[arg(long, required = false)]
        qdrant_payload_field: Option<String>,
        /// Maximum number of results to return
        #[arg(long, default_value = "10")]
        limit: u64,
        /// Score threshold for the results
        #[arg(long, default_value = "0.5")]
        score_threshold: f32,
        /// The base URL of the embedding server, e.g., "https://api.openai.com/v1" (can be overridden by EMBEDDING_SERVICE_BASE_URL env var)
        #[arg(long, required = false)]
        embedding_service_base_url: Option<String>,
    },
    /// Enable keyword search only
    Tidb {
        /// Path to the SSL CA certificate. On macOS, this is typically
        /// `/etc/ssl/cert.pem`. On Debian/Ubuntu/Arch Linux, it's typically
        /// `/etc/ssl/certs/ca-certificates.crt`. (can be overridden by TIDB_SSL_CA env var)
        #[arg(long, required = false)]
        tidb_ssl_ca: Option<PathBuf>,
        /// Table name to search in TiDB (can be overridden by TIDB_TABLE_NAME env var)
        #[arg(long, required = false)]
        tidb_table_name: Option<String>,
        /// Field name for full-text search content (can be overridden by TIDB_SEARCH_FIELD env var)
        #[arg(long, required = false)]
        tidb_search_field: Option<String>,
        /// Field name to return from TiDB query results (can be overridden by TIDB_RETURN_FIELD env var)
        #[arg(long, required = false)]
        tidb_return_field: Option<String>,
        /// Maximum number of results to return
        #[arg(long, default_value = "10")]
        limit: u64,
        /// Score threshold for the results
        #[arg(long, default_value = "0.5")]
        score_threshold: f32,
        /// The base URL of the chat server, e.g., "https://api.openai.com/v1" (can be overridden by CHAT_SERVICE_BASE_URL env var)
        #[arg(long, required = false)]
        chat_service_base_url: Option<String>,
    },
    /// Enable both vector and keyword search
    Search {
        /// Name of the collection to search in Qdrant (can be overridden by QDRANT_COLLECTION env var)
        #[arg(long, required = false)]
        qdrant_collection: Option<String>,
        /// The name of the field in the payload that contains the source of the document (can be overridden by QDRANT_PAYLOAD_FIELD env var)
        #[arg(long, required = false)]
        qdrant_payload_field: Option<String>,
        /// Path to the SSL CA certificate. On macOS, this is typically
        /// `/etc/ssl/cert.pem`. On Debian/Ubuntu/Arch Linux, it's typically
        /// `/etc/ssl/certs/ca-certificates.crt`. (can be overridden by TIDB_SSL_CA env var)
        #[arg(long, required = false)]
        tidb_ssl_ca: Option<PathBuf>,
        /// Table name to search in TiDB (can be overridden by TIDB_TABLE_NAME env var)
        #[arg(long, required = false)]
        tidb_table_name: Option<String>,
        /// Field name for full-text search content (can be overridden by TIDB_SEARCH_FIELD env var)
        #[arg(long, required = false)]
        tidb_search_field: Option<String>,
        /// Field name to return from TiDB query results (can be overridden by TIDB_RETURN_FIELD env var)
        #[arg(long, required = false)]
        tidb_return_field: Option<String>,
        /// Maximum number of results to return
        #[arg(long, default_value = "10")]
        limit: u64,
        /// Score threshold for the results
        #[arg(long, default_value = "0.5")]
        score_threshold: f32,
        /// The base URL of the chat server, e.g., "https://api.openai.com/v1" (can be overridden by CHAT_SERVICE_BASE_URL env var)
        #[arg(long, required = false)]
        chat_service_base_url: Option<String>,
        /// The base URL of the embedding server, e.g., "https://api.openai.com/v1" (can be overridden by EMBEDDING_SERVICE_BASE_URL env var)
        #[arg(long, required = false)]
        embedding_service_base_url: Option<String>,
    },
}

#[derive(Debug, Clone, ValueEnum)]
enum TransportType {
    Sse,
    StreamHttp,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env file in development mode only
    #[cfg(debug_assertions)]
    dotenv::dotenv().ok();

    // Prevent .env files in production builds
    #[cfg(not(debug_assertions))]
    {
        if std::path::Path::new(".env").exists() {
            panic!("Production environment should not contain .env file!");
        }
    }

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer().with_line_number(true))
        .init();

    let args = Args::parse();

    // Determine search mode and configure connection
    let search_config = match args.search_mode {
        SearchMode::Qdrant {
            qdrant_collection,
            qdrant_payload_field,
            limit,
            score_threshold,
            embedding_service_base_url,
        } => {
            info!("Enabling vector search mode");

            // Determine collection with priority: Environment Variable > Command Line > Error
            let qdrant_collection = match env::var("QDRANT_COLLECTION") {
                Ok(env_value) => {
                    info!("Using QDRANT_COLLECTION from environment: {}", env_value);
                    env_value
                }
                Err(_) => match qdrant_collection {
                    Some(arg_value) => {
                        info!(
                            "Using qdrant_collection from command line argument: {}",
                            arg_value
                        );
                        arg_value
                    }
                    None => {
                        bail!(
                            "QDRANT_COLLECTION environment variable or --qdrant-collection argument is required"
                        );
                    }
                },
            };

            // Determine payload field with priority: Environment Variable > Command Line > Error
            let qdrant_payload_field = match env::var("QDRANT_PAYLOAD_FIELD") {
                Ok(env_value) => {
                    info!("Using QDRANT_PAYLOAD_FIELD from environment: {}", env_value);
                    env_value
                }
                Err(_) => match qdrant_payload_field {
                    Some(arg_value) => {
                        info!(
                            "Using qdrant_payload_field from command line argument: {}",
                            arg_value
                        );
                        arg_value
                    }
                    None => {
                        bail!(
                            "QDRANT_PAYLOAD_FIELD environment variable or --qdrant-payload-field argument is required"
                        );
                    }
                },
            };

            // parse base url
            let qdrant_base_url =
                std::env::var("QDRANT_BASE_URL").unwrap_or(DEFAULT_QDRANT_BASE_URL.to_string());

            // parse api key
            let qdrant_api_key = env::var("QDRANT_API_KEY").ok();

            // parse embedding service base url with priority: Environment Variable > Command Line > Error
            let embedding_service_base_url = match env::var("EMBEDDING_SERVICE_BASE_URL") {
                Ok(env_value) => {
                    info!(
                        "Using EMBEDDING_SERVICE_BASE_URL from environment: {}",
                        env_value
                    );
                    env_value
                }
                Err(_) => match embedding_service_base_url {
                    Some(arg_value) => {
                        info!(
                            "Using embedding_service_base_url from command line argument: {}",
                            arg_value
                        );
                        arg_value
                    }
                    None => {
                        bail!(
                            "EMBEDDING_SERVICE_BASE_URL environment variable or --embedding-service-base-url argument is required"
                        );
                    }
                },
            };

            // parse embedding service api key
            let embedding_service_api_key = env::var("EMBEDDING_SERVICE_API_KEY").ok();

            // parse embedding service model
            let embedding_service_model = env::var("EMBEDDING_SERVICE_MODEL").ok();

            AgenticSearchConfig {
                qdrant_config: Some(QdrantConfig {
                    api_key: qdrant_api_key,
                    base_url: qdrant_base_url,
                    collection: qdrant_collection,
                    payload_source: qdrant_payload_field,
                }),
                tidb_config: None,
                limit,
                score_threshold,
                chat_service: None,
                embedding_service: Some(ServiceConfig {
                    url: embedding_service_base_url,
                    api_key: embedding_service_api_key,
                    model: embedding_service_model,
                }),
            }
        }
        SearchMode::Tidb {
            tidb_ssl_ca,
            tidb_table_name,
            tidb_search_field,
            tidb_return_field,
            limit,
            score_threshold,
            chat_service_base_url,
        } => {
            info!("Enabling keyword search mode");

            // Determine SSL CA path with priority: Environment Variable > Command Line > Error
            let tidb_ssl_ca = match env::var("TIDB_SSL_CA") {
                Ok(env_value) => {
                    info!("Using TIDB_SSL_CA from environment: {}", env_value);
                    PathBuf::from(env_value)
                }
                Err(_) => match tidb_ssl_ca {
                    Some(arg_value) => {
                        info!(
                            "Using tidb_ssl_ca from command line argument: {}",
                            arg_value.display()
                        );
                        arg_value
                    }
                    None => {
                        bail!(
                            "TIDB_SSL_CA environment variable or --tidb-ssl-ca argument is required"
                        );
                    }
                },
            };

            // Determine table name with priority: Environment Variable > Command Line > Error
            let tidb_table_name = match env::var("TIDB_TABLE_NAME") {
                Ok(env_value) => {
                    info!("Using TIDB_TABLE_NAME from environment: {}", env_value);
                    env_value
                }
                Err(_) => match tidb_table_name {
                    Some(arg_value) => {
                        info!(
                            "Using tidb_table_name from command line argument: {}",
                            arg_value
                        );
                        arg_value
                    }
                    None => {
                        bail!(
                            "TIDB_TABLE_NAME environment variable or --tidb-table-name argument is required"
                        );
                    }
                },
            };

            // Determine content field with priority: Environment Variable > Command Line > Default
            let tidb_search_field = match env::var("TIDB_SEARCH_FIELD") {
                Ok(env_value) => {
                    info!("Using TIDB_SEARCH_FIELD from environment: {}", env_value);
                    env_value
                }
                Err(_) => match tidb_search_field {
                    Some(arg_value) => {
                        info!(
                            "Using TIDB_SEARCH_FIELD from command line argument: {}",
                            arg_value
                        );
                        arg_value
                    }
                    None => {
                        info!("Using TIDB_SEARCH_FIELD default value: content");
                        "content".to_string()
                    }
                },
            };

            // Determine return field with priority: Environment Variable > Command Line > Default
            let tidb_return_field = match env::var("TIDB_RETURN_FIELD") {
                Ok(env_value) => {
                    info!("Using TIDB_RETURN_FIELD from environment: {}", env_value);
                    env_value
                }
                Err(_) => match tidb_return_field {
                    Some(arg_value) => {
                        info!(
                            "Using TIDB_RETURN_FIELD from command line argument: {}",
                            arg_value
                        );
                        arg_value
                    }
                    None => {
                        info!("Using TIDB_RETURN_FIELD default value: *");
                        "*".to_string()
                    }
                },
            };

            // parse connection string
            let (username, password, host, port, database) = match env::var("TIDB_CONNECTION") {
                Ok(ref conn) => {
                    parse_tidb_conn_str(conn.as_str()).ok_or_else(|| anyhow!(
                        "Invalid connection string! The pattern should be `mysql://<USERNAME>:<PASSWORD>@<HOST>:<PORT>/<DATABASE>`"
                    ))?
                }
                Err(e) => {
                    let error_message = format!("Failed to get TIDB_CONNECTION: {e}");
                    error!(error_message);
                    bail!(error_message);
                }
            };

            // convert port to u16
            let port = port.parse::<u16>().map_err(|e| {
                let error_message = format!("Failed to parse TIDB_PORT: {e}");
                error!(error_message);
                anyhow!(error_message)
            })?;

            // parse chat service base url with priority: Environment Variable > Command Line > Error
            let chat_service_base_url = match env::var("CHAT_SERVICE_BASE_URL") {
                Ok(env_value) => {
                    info!(
                        "Using CHAT_SERVICE_BASE_URL from environment: {}",
                        env_value
                    );
                    env_value
                }
                Err(_) => match chat_service_base_url {
                    Some(arg_value) => {
                        info!(
                            "Using chat_service_base_url from command line argument: {}",
                            arg_value
                        );
                        arg_value
                    }
                    None => {
                        bail!(
                            "CHAT_SERVICE_BASE_URL environment variable or --chat-service-base-url argument is required"
                        );
                    }
                },
            };

            // parse chat service api key
            let chat_service_api_key = env::var("CHAT_SERVICE_API_KEY").ok();

            // parse chat service model
            let chat_service_model = env::var("CHAT_SERVICE_MODEL").ok();

            CryptoProvider::install_default(default_provider()).map_err(|e| {
                let err_msg = format!("Failed to install default crypto provider: {e:?}");
                error!("{}", err_msg);
                anyhow!(err_msg)
            })?;

            // create connection options
            info!("Creating connection options for TiDB Cloud...");
            let opts = OptsBuilder::new()
                .ip_or_hostname(Some(host))
                .tcp_port(port)
                .user(Some(username))
                .pass(Some(password))
                .db_name(Some(database.clone()))
                .ssl_opts(Some(
                    SslOpts::default().with_root_cert_path(Some(tidb_ssl_ca)),
                ))
                .init(vec!["SET NAMES utf8mb4".to_string()]);

            // create connection pool
            info!("Creating connection pool...");
            let pool = Pool::new(opts).map_err(|e| {
                let error_message = format!("Failed to create connection pool: {e}");
                error!(error_message);
                anyhow!(error_message)
            })?;

            AgenticSearchConfig {
                qdrant_config: None,
                tidb_config: Some(TiDBConfig {
                    database,
                    table_name: tidb_table_name,
                    pool,
                    search_field: tidb_search_field,
                    return_field: tidb_return_field,
                }),
                limit,
                score_threshold,
                chat_service: Some(ServiceConfig {
                    url: chat_service_base_url,
                    api_key: chat_service_api_key,
                    model: chat_service_model,
                }),
                embedding_service: None,
            }
        }
        SearchMode::Search {
            qdrant_collection,
            qdrant_payload_field,
            tidb_ssl_ca,
            tidb_table_name,
            tidb_search_field,
            tidb_return_field,
            limit,
            score_threshold,
            chat_service_base_url,
            embedding_service_base_url,
        } => {
            info!("Enabling both vector and keyword search modes");

            // Determine collection with priority: Environment Variable > Command Line > Error
            let qdrant_collection = match env::var("QDRANT_COLLECTION") {
                Ok(env_value) => {
                    info!("Using QDRANT_COLLECTION from environment: {}", env_value);
                    env_value
                }
                Err(_) => match qdrant_collection {
                    Some(arg_value) => {
                        info!(
                            "Using qdrant_collection from command line argument: {}",
                            arg_value
                        );
                        arg_value
                    }
                    None => {
                        bail!(
                            "QDRANT_COLLECTION environment variable or --qdrant-collection argument is required"
                        );
                    }
                },
            };

            // Determine payload field with priority: Environment Variable > Command Line > Error
            let qdrant_payload_field = match env::var("QDRANT_PAYLOAD_FIELD") {
                Ok(env_value) => {
                    info!("Using QDRANT_PAYLOAD_FIELD from environment: {}", env_value);
                    env_value
                }
                Err(_) => match qdrant_payload_field {
                    Some(arg_value) => {
                        info!(
                            "Using qdrant_payload_field from command line argument: {}",
                            arg_value
                        );
                        arg_value
                    }
                    None => {
                        bail!(
                            "QDRANT_PAYLOAD_FIELD environment variable or --qdrant-payload-field argument is required"
                        );
                    }
                },
            };

            // Determine SSL CA path with priority: Environment Variable > Command Line > Error
            let tidb_ssl_ca = match env::var("TIDB_SSL_CA") {
                Ok(env_value) => {
                    info!("Using TIDB_SSL_CA from environment: {}", env_value);
                    PathBuf::from(env_value)
                }
                Err(_) => match tidb_ssl_ca {
                    Some(arg_value) => {
                        info!(
                            "Using tidb_ssl_ca from command line argument: {}",
                            arg_value.display()
                        );
                        arg_value
                    }
                    None => {
                        bail!(
                            "TIDB_SSL_CA environment variable or --tidb-ssl-ca argument is required"
                        );
                    }
                },
            };

            // Determine table name with priority: Environment Variable > Command Line > Error
            let tidb_table_name = match env::var("TIDB_TABLE_NAME") {
                Ok(env_value) => {
                    info!("Using TIDB_TABLE_NAME from environment: {}", env_value);
                    env_value
                }
                Err(_) => match tidb_table_name {
                    Some(arg_value) => {
                        info!(
                            "Using tidb_table_name from command line argument: {}",
                            arg_value
                        );
                        arg_value
                    }
                    None => {
                        bail!(
                            "TIDB_TABLE_NAME environment variable or --tidb-table-name argument is required"
                        );
                    }
                },
            };

            // Determine content field with priority: Environment Variable > Command Line > Default
            let tidb_search_field = match env::var("TIDB_SEARCH_FIELD") {
                Ok(env_value) => {
                    info!("Using TIDB_SEARCH_FIELD from environment: {}", env_value);
                    env_value
                }
                Err(_) => match tidb_search_field {
                    Some(arg_value) => {
                        info!(
                            "Using TIDB_SEARCH_FIELD from command line argument: {}",
                            arg_value
                        );
                        arg_value
                    }
                    None => {
                        info!("Using TIDB_SEARCH_FIELD default value: content");
                        "content".to_string()
                    }
                },
            };

            // Determine return field with priority: Environment Variable > Command Line > Default
            let tidb_return_field = match env::var("TIDB_RETURN_FIELD") {
                Ok(env_value) => {
                    info!("Using TIDB_RETURN_FIELD from environment: {}", env_value);
                    env_value
                }
                Err(_) => match tidb_return_field {
                    Some(arg_value) => {
                        info!(
                            "Using TIDB_RETURN_FIELD from command line argument: {}",
                            arg_value
                        );
                        arg_value
                    }
                    None => {
                        info!("Using TIDB_RETURN_FIELD default value: *");
                        "*".to_string()
                    }
                },
            };

            // parse base url
            let qdrant_base_url =
                std::env::var("QDRANT_BASE_URL").unwrap_or(DEFAULT_QDRANT_BASE_URL.to_string());

            // parse qdrant api key
            let qdrant_api_key = env::var("QDRANT_API_KEY").ok();

            // parse connection string
            let (tidb_username, tidb_password, tidb_host, tidb_port, tidb_database) = match env::var("TIDB_CONNECTION") {
                Ok(ref conn) => {
                    parse_tidb_conn_str(conn.as_str()).ok_or_else(|| anyhow!(
                        "Invalid connection string! The pattern should be `mysql://<USERNAME>:<PASSWORD>@<HOST>:<PORT>/<DATABASE>`"
                    ))?
                }
                Err(e) => {
                    let error_message = format!("Failed to get TIDB_CONNECTION: {e}");
                    error!(error_message);
                    bail!(error_message);
                }
            };

            // convert port to u16
            let tidb_port = tidb_port.parse::<u16>().map_err(|e| {
                let error_message = format!("Failed to parse TIDB_PORT: {e}");
                error!(error_message);
                anyhow!(error_message)
            })?;

            // parse chat service base url with priority: Environment Variable > Command Line > Error
            let chat_service_base_url = match env::var("CHAT_SERVICE_BASE_URL") {
                Ok(env_value) => {
                    info!(
                        "Using CHAT_SERVICE_BASE_URL from environment: {}",
                        env_value
                    );
                    env_value
                }
                Err(_) => match chat_service_base_url {
                    Some(arg_value) => {
                        info!(
                            "Using chat_service_base_url from command line argument: {}",
                            arg_value
                        );
                        arg_value
                    }
                    None => {
                        bail!(
                            "CHAT_SERVICE_BASE_URL environment variable or --chat-service-base-url argument is required"
                        );
                    }
                },
            };

            // parse chat service api key
            let chat_service_api_key = env::var("CHAT_SERVICE_API_KEY").ok();

            // parse chat service model
            let chat_service_model = env::var("CHAT_SERVICE_MODEL").ok();

            // parse embedding service base url with priority: Environment Variable > Command Line > Error
            let embedding_service_base_url = match env::var("EMBEDDING_SERVICE_BASE_URL") {
                Ok(env_value) => {
                    info!(
                        "Using EMBEDDING_SERVICE_BASE_URL from environment: {}",
                        env_value
                    );
                    env_value
                }
                Err(_) => match embedding_service_base_url {
                    Some(arg_value) => {
                        info!(
                            "Using embedding_service_base_url from command line argument: {}",
                            arg_value
                        );
                        arg_value
                    }
                    None => {
                        bail!(
                            "EMBEDDING_SERVICE_BASE_URL environment variable or --embedding-service-base-url argument is required"
                        );
                    }
                },
            };

            // parse embedding service api key
            let embedding_service_api_key = env::var("EMBEDDING_SERVICE_API_KEY").ok();

            // parse embedding service model
            let embedding_service_model = env::var("EMBEDDING_SERVICE_MODEL").ok();

            CryptoProvider::install_default(default_provider()).map_err(|e| {
                let err_msg = format!("Failed to install default crypto provider: {e:?}");
                error!("{}", err_msg);
                anyhow!(err_msg)
            })?;

            // create connection options
            info!("Creating connection options for TiDB Cloud...");
            let opts = OptsBuilder::new()
                .ip_or_hostname(Some(tidb_host))
                .tcp_port(tidb_port)
                .user(Some(tidb_username))
                .pass(Some(tidb_password))
                .db_name(Some(tidb_database.clone()))
                .ssl_opts(Some(
                    SslOpts::default().with_root_cert_path(Some(tidb_ssl_ca)),
                ));

            // create connection pool
            info!("Creating connection pool...");
            let pool = Pool::new(opts).map_err(|e| {
                let error_message = format!("Failed to create connection pool: {e}");
                error!(error_message);
                anyhow!(error_message)
            })?;

            AgenticSearchConfig {
                qdrant_config: Some(QdrantConfig {
                    api_key: qdrant_api_key,
                    base_url: qdrant_base_url,
                    collection: qdrant_collection,
                    payload_source: qdrant_payload_field,
                }),
                tidb_config: Some(TiDBConfig {
                    database: tidb_database,
                    table_name: tidb_table_name,
                    pool,
                    search_field: tidb_search_field,
                    return_field: tidb_return_field,
                }),
                limit,
                score_threshold,
                chat_service: Some(ServiceConfig {
                    url: chat_service_base_url,
                    api_key: chat_service_api_key,
                    model: chat_service_model,
                }),
                embedding_service: Some(ServiceConfig {
                    url: embedding_service_base_url,
                    api_key: embedding_service_api_key,
                    model: embedding_service_model,
                }),
            }
        }
    };

    info!(
        "Starting Cardea Agentic Search MCP server on {}",
        args.socket_addr
    );

    match args.transport {
        TransportType::StreamHttp => {
            let service = StreamableHttpService::new(
                move || Ok(AgenticSearchServer::new(search_config.clone())),
                LocalSessionManager::default().into(),
                Default::default(),
            );

            let router = axum::Router::new().nest_service("/mcp", service);
            let tcp_listener = tokio::net::TcpListener::bind(args.socket_addr).await?;
            let _ = axum::serve(tcp_listener, router)
                .with_graceful_shutdown(async { tokio::signal::ctrl_c().await.unwrap() })
                .await;
        }
        TransportType::Sse => {
            let ct = SseServer::serve(args.socket_addr.parse()?)
                .await?
                .with_service(move || AgenticSearchServer::new(search_config.clone()));

            tokio::signal::ctrl_c().await?;
            ct.cancel();
        }
    }

    Ok(())
}

#[derive(Debug, Clone)]
pub struct AgenticSearchConfig {
    pub qdrant_config: Option<QdrantConfig>,
    pub tidb_config: Option<TiDBConfig>,
    pub limit: u64,
    pub score_threshold: f32,
    pub chat_service: Option<ServiceConfig>,
    pub embedding_service: Option<ServiceConfig>,
}

#[derive(Debug, Clone)]
pub struct QdrantConfig {
    pub api_key: Option<String>,
    pub base_url: String,
    pub collection: String,
    pub payload_source: String,
}

#[derive(Debug, Clone)]
pub struct TiDBConfig {
    pub database: String,
    pub table_name: String,
    pub pool: Pool,
    pub search_field: String,
    pub return_field: String,
}

#[derive(Debug, Clone)]
pub struct ServiceConfig {
    pub url: String,
    pub api_key: Option<String>,
    pub model: Option<String>,
}

fn parse_tidb_conn_str(conn_str: &str) -> Option<(String, String, String, String, String)> {
    let re = Regex::new(r"^mysql://([^:]+):([^@]+)@([^:/]+):(\d+)/(.+)$").unwrap();
    if let Some(caps) = re.captures(conn_str) {
        let username = caps.get(1)?.as_str().to_string();
        let password = caps.get(2)?.as_str().to_string();
        let host = caps.get(3)?.as_str().to_string();
        let port = caps.get(4)?.as_str().to_string();
        let database = caps.get(5)?.as_str().to_string();
        Some((username, password, host, port, database))
    } else {
        None
    }
}
