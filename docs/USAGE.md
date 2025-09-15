# Usage Guide

## Features

The `agentic-search` mcp server provides advanced search capabilities with the following features:

- **Vector Search**: Semantic search using Qdrant vector database with embedding services
- **Keyword Search**: Full-text search using TiDB with intelligent keyword extraction
- **Combined Search**: Use both vector and keyword search simultaneously for comprehensive results
- **Flexible Configuration**: Choose your search mode via command-line subcommands
- **Multiple Transport Types**: Support for both SSE and Streamable HTTP MCP transports
- **Customizable Keyword Extraction**: Configure keyword extraction prompts via environment variables

## Usage

### Command Line Options

The server supports three search modes through subcommands:

#### Global Options

These options apply to all search modes:

- `-s, --socket-addr`: Socket address to bind to (default: 127.0.0.1:8009)
- `-t, --transport`: Transport type (sse, stream-http) (default: stream-http)

#### 1. Qdrant Vector Search Only

```bash
./cardea-agentic-search-mcp-server qdrant \
    --qdrant-collection my_collection \
    --qdrant-payload-field "full_text" \
    --embedding-service-base-url http://localhost:8081/v1 \
    --limit 20 \
    --score-threshold 0.7
```

**Options:**

- `--qdrant-collection`: Collection name in Qdrant (required if QDRANT_COLLECTION env var not set)
- `--qdrant-payload-field`: The name of the field in the payload that contains the source of the document (required if QDRANT_PAYLOAD_FIELD env var not set)
- `--embedding-service-base-url`: Embedding service base URL (required if EMBEDDING_SERVICE_BASE_URL env var not set)
- `--limit`: Maximum number of results (default: 10)
- `--score-threshold`: Score threshold for results (default: 0.5)

**Note:** Qdrant base URL is configured via the `QDRANT_BASE_URL` environment variable (default: http://127.0.0.1:6333)

#### 2. TiDB Keyword Search Only

```bash
./cardea-agentic-search-mcp-server tidb \
    --tidb-ssl-ca /path/to/ca.pem \
    --tidb-table-name my_table \
    --tidb-search-field "content" \
    --tidb-return-field "*" \
    --chat-service-base-url http://localhost:8080/v1 \
    --limit 15
```

**Options:**

- `--tidb-ssl-ca`: TiDB SSL CA certificate path (required if TIDB_SSL_CA env var not set)
  - On macOS: typically `/etc/ssl/cert.pem`
  - On Debian/Ubuntu/Arch Linux: typically `/etc/ssl/certs/ca-certificates.crt`
- `--tidb-table-name`: Table name in TiDB (required if TIDB_TABLE_NAME env var not set)
- `--tidb-search-field`: Field name for full-text search content (optional, default: "content", overridden by TIDB_SEARCH_FIELD env var)
- `--tidb-return-field`: Field names to return from TiDB query results, comma-separated (optional, default: "*", overridden by TIDB_RETURN_FIELD env var)
- `--chat-service-base-url`: Chat service base URL (required if CHAT_SERVICE_BASE_URL env var not set)
- `--limit`: Maximum number of results (default: 10)
- `--score-threshold`: Score threshold for results (default: 0.5)

#### 3. Combined Search (Both Vector and Keyword)

```bash
./cardea-agentic-search-mcp-server search \
    --qdrant-collection my_collection \
    --qdrant-payload-field "full_text" \
    --tidb-ssl-ca /path/to/ca.pem \
    --tidb-table-name my_table \
    --tidb-search-field "content" \
    --tidb-return-field "*" \
    --chat-service-base-url http://localhost:8080/v1 \
    --embedding-service-base-url http://localhost:8081/v1 \
    --limit 25
```

**Options:**

- `--qdrant-collection`: Collection name in Qdrant (required if QDRANT_COLLECTION env var not set)
- `--qdrant-payload-field`: The name of the field in the payload that contains the source of the document (required if QDRANT_PAYLOAD_FIELD env var not set)
- `--tidb-ssl-ca`: TiDB SSL CA certificate path (required if TIDB_SSL_CA env var not set)
  - On macOS: typically `/etc/ssl/cert.pem`
  - On Debian/Ubuntu/Arch Linux: typically `/etc/ssl/certs/ca-certificates.crt`
- `--tidb-table-name`: Table name in TiDB (required if TIDB_TABLE_NAME env var not set)
- `--tidb-search-field`: Field name for full-text search content (optional, default: "content", overridden by TIDB_SEARCH_FIELD env var)
- `--tidb-return-field`: Field names to return from TiDB query results, comma-separated (optional, default: "*", overridden by TIDB_RETURN_FIELD env var)
- `--chat-service-base-url`: Chat service base URL (required if CHAT_SERVICE_BASE_URL env var not set)
- `--embedding-service-base-url`: Embedding service base URL (required if EMBEDDING_SERVICE_BASE_URL env var not set)
- `--limit`: Maximum number of results (default: 10)
- `--score-threshold`: Score threshold for results (default: 0.5)

**Note:** Qdrant base URL is configured via the `QDRANT_BASE_URL` environment variable (default: http://127.0.0.1:6333)

### Environment Variables

#### For Qdrant Vector Search

- `QDRANT_BASE_URL`: Qdrant database URL (default: http://127.0.0.1:6333)
- `QDRANT_API_KEY`: API key for Qdrant (optional)
- `QDRANT_COLLECTION`: Name of the collection to search in Qdrant (required for vector search modes, overrides command line)
- `QDRANT_PAYLOAD_FIELD`: The name of the field in the payload that contains the source of the document (required for vector search modes, overrides command line)

#### For TiDB Keyword Search

- `TIDB_CONNECTION`: TiDB connection string in format `mysql://<USERNAME>:<PASSWORD>@<HOST>:<PORT>/<DATABASE>` (required)
- `TIDB_SSL_CA`: Path to the SSL CA certificate (required for TiDB modes, overrides command line)
- `TIDB_TABLE_NAME`: Table name to search in TiDB (required for TiDB modes, overrides command line)
- `TIDB_SEARCH_FIELD`: Field name for full-text search content (optional, default: "content")
- `TIDB_RETURN_FIELD`: Field names to return from TiDB query results, comma-separated (optional, default: "*")
- `PROMPT_KEYWORD_EXTRACTOR`: Custom prompt for keyword extraction (optional, uses built-in default if not set)

#### For External Services

- `CHAT_SERVICE_BASE_URL`: Base URL for chat service (required for keyword search modes, overrides command line)
- `CHAT_SERVICE_API_KEY`: API key for chat service (optional)
- `CHAT_SERVICE_MODEL`: Model name for chat service (optional, e.g., "gpt-4", "claude-3")
- `EMBEDDING_SERVICE_BASE_URL`: Base URL for embedding service (required for vector search modes, overrides command line)
- `EMBEDDING_SERVICE_API_KEY`: API key for embedding service (optional)
- `EMBEDDING_SERVICE_MODEL`: Model name for embedding service (optional, e.g., "text-embedding-ada-002")

## Examples

### Qdrant Vector Search Example

```bash
export QDRANT_BASE_URL=http://localhost:6333
export QDRANT_API_KEY=your_qdrant_api_key
export QDRANT_COLLECTION=documents
export QDRANT_PAYLOAD_FIELD="full_text"
export EMBEDDING_SERVICE_BASE_URL=http://localhost:8081/v1
export EMBEDDING_SERVICE_API_KEY=your_embedding_api_key
export EMBEDDING_SERVICE_MODEL=text-embedding-ada-002

# Using environment variables (no need for --qdrant-collection, --qdrant-payload-field, and --embedding-service-base-url)
./cardea-agentic-search qdrant \
    --limit 10 \
    --score-threshold 0.6

# Or using command line arguments (will override environment variables if set)
./cardea-agentic-search qdrant \
    --qdrant-collection documents \
    --qdrant-payload-field "full_text" \
    --embedding-service-base-url http://localhost:8081/v1 \
    --limit 10 \
    --score-threshold 0.6
```

### TiDB Keyword Search Example

```bash
export TIDB_CONNECTION="mysql://root:mypassword@localhost:4000/search_db"
export TIDB_SSL_CA=/etc/ssl/certs/ca.pem
export TIDB_TABLE_NAME=documents
export CHAT_SERVICE_BASE_URL=http://localhost:8080/v1
export CHAT_SERVICE_API_KEY=your_chat_api_key
export CHAT_SERVICE_MODEL=gpt-4

# Using environment variables (no need for --tidb-ssl-ca, --tidb-table-name, and --chat-service-base-url)
./cardea-agentic-search tidb \
    --tidb-search-field "content" \
    --tidb-return-field "id,title,content" \
    --limit 20 \
    --score-threshold 0.4

# Or using command line arguments (will override environment variables if set)
./cardea-agentic-search tidb \
    --tidb-ssl-ca /etc/ssl/certs/ca.pem \
    --tidb-table-name documents \
    --tidb-search-field "content" \
    --tidb-return-field "id,title,content" \
    --chat-service-base-url http://localhost:8080/v1 \
    --limit 20 \
    --score-threshold 0.4
```

### Combined Search Example

```bash
export TIDB_CONNECTION="mysql://root:mypassword@localhost:4000/search_db"
export QDRANT_BASE_URL=http://localhost:6333
export QDRANT_API_KEY=your_qdrant_api_key
export QDRANT_COLLECTION=documents
export QDRANT_PAYLOAD_FIELD="full_text"
export TIDB_SSL_CA=/etc/ssl/certs/ca.pem
export TIDB_TABLE_NAME=documents
export CHAT_SERVICE_BASE_URL=http://localhost:8080/v1
export CHAT_SERVICE_API_KEY=your_chat_api_key
export CHAT_SERVICE_MODEL=gpt-4
export EMBEDDING_SERVICE_BASE_URL=http://localhost:8081/v1
export EMBEDDING_SERVICE_API_KEY=your_embedding_api_key
export EMBEDDING_SERVICE_MODEL=text-embedding-ada-002

# Using environment variables (no need for --qdrant-collection, --qdrant-payload-field, --tidb-ssl-ca, --tidb-table-name, --embedding-service-base-url, --chat-service-base-url)
./cardea-agentic-search search \
    --tidb-search-field "content" \
    --tidb-return-field "id,title,content" \
    --limit 15 \
    --score-threshold 0.5

# Or using command line arguments (will override environment variables if set)
./cardea-agentic-search search \
    --qdrant-collection documents \
    --qdrant-payload-field "full_text" \
    --tidb-ssl-ca /etc/ssl/certs/ca.pem \
    --tidb-table-name documents \
    --tidb-search-field "content" \
    --tidb-return-field "id,title,content" \
    --chat-service-base-url http://localhost:8080/v1 \
    --embedding-service-base-url http://localhost:8081/v1 \
    --limit 15 \
    --score-threshold 0.5
```

## How It Works

### Vector Search Process

1. **Query Processing**: The user query is sent to the embedding service to generate a vector representation
2. **Vector Search**: The generated vector is used to search the Qdrant collection for similar documents
3. **Result Formatting**: Results are formatted and returned with scores and metadata

### Keyword Search Process

1. **Keyword Extraction**: The user query is sent to the chat service to extract relevant keywords using a customizable prompt
2. **Full-text Search**: The extracted keywords are used to perform full-text search in TiDB
3. **Result Formatting**: Results are formatted and returned with document content

#### Keyword Extraction Customization

The keyword extraction process uses an intelligent prompt that can be customized via the `PROMPT_KEYWORD_EXTRACTOR` environment variable. The default prompt is a multilingual keyword extractor that:

```text
You are a multilingual keyword extractor. Your task is to extract the most relevant and concise keywords or key phrases from the given user query.

Follow these requirements strictly:
- Detect the language of the query automatically.
- Return 3 to 7 keywords or keyphrases that best represent the query's core intent.
- Keep the extracted keywords in the **original language** (do not translate).
- Include **multi-word expressions** if they convey meaningful concepts.
- **Avoid all types of stop words, question words, filler words, or overly generic terms**, such as:
  - English: what, how, why, is, the, of, and, etc.
  - Chinese: 什么、怎么、如何、是、的、了、吗、啊 等。
- Do **not** include punctuation or meaningless words.
- Only return the final keywords, separated by a **single space**.

Examples:
- Input: "What is the impact of artificial intelligence on education?"
  Output: artificial intelligence education impact
- Input: "什么是人工智能对教育的影响？"
  Output: 人工智能 教育 影响
```

## TiDB Return Fields Configuration

The `--tidb-return-field` parameter (or `TIDB_RETURN_FIELD` environment variable) supports flexible field selection for TiDB queries:

### Single Field

```bash
--tidb-return-field id
# Returns only the 'id' field
```

### Multiple Fields (Comma-separated)

```bash
--tidb-return-field id,title,content
# Returns 'id', 'title', and 'content' fields
```

### All Fields (Default)

```bash
--tidb-return-field "*"
# Returns all fields (*)
```

### With Spaces

```bash
--tidb-return-field "id, title, content"
# Spaces are automatically trimmed
```

### Environment Variable Examples

```bash
export TIDB_RETURN_FIELD=id,title,content
export TIDB_RETURN_FIELD="id, title, content"
export TIDB_RETURN_FIELD=*
```

### Notes

- Field names are case-sensitive and must match your TiDB table schema
- Multiple fields are returned in the order specified
- The `*` wildcard returns all available fields
- Invalid field names will result in SQL errors
