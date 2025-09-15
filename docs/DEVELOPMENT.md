# Development Guide


## Architecture

The server is designed with a modular architecture that supports different search backends:

- **Vector Search**: Uses Qdrant for semantic/vector-based search with embedding services
- **Keyword Search**: Uses TiDB for full-text search with intelligent keyword extraction via chat services
- **Combined Search**: Merges results from both vector and keyword search for comprehensive results

## Development

### Building

**For development:**

```bash
cargo build
```

**For production:**

```bash
# First, ensure no sensitive .env files exist (keep .env.example)
ls .env* 2>/dev/null | grep -v ".env.example" && echo "⚠️ Remove .env files before production build" || echo "✅ No .env files found"

# Build release version
cargo build --release
```

**Quick development setup:**

```bash
# 1. Copy environment template
cp .env.example .env

# 2. Edit .env with your actual configuration values
# (all variables are optional - see configuration section below)

# 3. Build and run
cargo build
./target/debug/cardea-agentic-search --help
```

### Configuration

#### Environment Variables

The server uses environment variables for sensitive configuration. Copy `.env.example` to `.env` and configure your values:

```bash
cp .env.example .env
```

**Required Environment Variables by Search Mode:**

**For Vector Search (Qdrant mode):**

- `QDRANT_BASE_URL`: Qdrant server URL (default: <http://127.0.0.1:6333>)
- `EMBEDDING_SERVICE_BASE_URL`: Embedding service base URL (required for vector search modes, overrides command line)
- `EMBEDDING_SERVICE_API_KEY`: API key for embedding service (optional - only needed if service requires authentication)
- `QDRANT_API_KEY`: Qdrant API key (optional - only needed for authenticated Qdrant instances)
- `QDRANT_COLLECTION`: Name of the collection to search in Qdrant (required for vector search modes, overrides command line)
- `QDRANT_PAYLOAD_FIELD`: The name of the field in the payload that contains the source of the document (required for vector search modes, overrides command line)

**For Keyword Search (TiDB mode):**

- `TIDB_CONNECTION`: TiDB connection string (format: `mysql://username:password@host:port/database`)
- `CHAT_SERVICE_BASE_URL`: Chat service base URL (required for keyword search modes, overrides command line)
- `CHAT_SERVICE_API_KEY`: API key for chat service (optional - only needed if service requires authentication)
- `TIDB_SEARCH_FIELD`: Field name for full-text search content (optional, default: "content")

**For Combined Search mode:**

- `QDRANT_BASE_URL`: Qdrant server URL
- `QDRANT_API_KEY`: Qdrant API key (optional)
- `QDRANT_COLLECTION`: Name of the collection to search in Qdrant (required for vector search modes, overrides command line)
- `QDRANT_PAYLOAD_FIELD`: The name of the field in the payload that contains the source of the document (required for vector search modes, overrides command line)
- `TIDB_CONNECTION`: TiDB connection string
- `TIDB_SEARCH_FIELD`: Field name for full-text search content (optional, default: "content")
- `EMBEDDING_SERVICE_BASE_URL`: Embedding service base URL (required for vector search modes, overrides command line)
- `EMBEDDING_SERVICE_API_KEY`: API key for embedding service (optional)
- `CHAT_SERVICE_BASE_URL`: Chat service base URL (required for keyword search modes, overrides command line)
- `CHAT_SERVICE_API_KEY`: API key for chat service (optional)

**Configuration Priority (highest to lowest):**

1. **Environment variables** (highest priority)
2. **Command line arguments**
3. **Default values** (lowest priority)

**Optional Environment Variables:**

- `PROMPT_KEYWORD_EXTRACTOR`: Custom prompt for keyword extraction (has built-in default)
- `RUST_LOG`: Logging level (default: info)

**Example .env file:**

```bash
QDRANT_BASE_URL=http://127.0.0.1:6333
# QDRANT_API_KEY=your_qdrant_api_key  # Optional - only needed for authenticated Qdrant
TIDB_CONNECTION=mysql://user:pass@host:4000/database
# TIDB_SEARCH_FIELD=content  # Optional - field name for full-text search (default: "content")
# TIDB_RETURN_FIELD=*  # Optional - field name to return from TiDB query results (default: "*")
# CHAT_SERVICE_BASE_URL=https://api.openai.com/v1  # Optional - chat service base URL (can be overridden by command line)
# EMBEDDING_SERVICE_BASE_URL=https://api.openai.com/v1  # Optional - embedding service base URL (can be overridden by command line)
# EMBEDDING_SERVICE_API_KEY=your_embedding_key  # Optional - only needed if service requires auth
# CHAT_SERVICE_API_KEY=your_chat_key  # Optional - only needed if service requires auth
RUST_LOG=info
# PROMPT_KEYWORD_EXTRACTOR=Custom prompt for keyword extraction  # Optional - has built-in default
```

**Configuration Examples:**

```bash
# Example 1: Using environment variable (highest priority)
export TIDB_SEARCH_FIELD="article_text"
export TIDB_RETURN_FIELD="title,content,source"
./cardea-agentic-search tidb --tidb-search-field "description" --tidb-return-field "summary" [other options...]
# Result: Uses "article_text" and "title,content,source" from environment variables

# Example 2: Using command line argument
./cardea-agentic-search tidb --tidb-search-field "full_text" --tidb-return-field "title,content" [other options...]
# Result: Uses "full_text" and "title,content" from command line

# Example 3: Using default value
./cardea-agentic-search tidb [other options...]
# Result: Uses "content" (search field default) and "*" (return field default)
```

#### Security Best Practices

**⚠️ Important Security Notes:**

- **Never commit `.env` files** to version control
- **Remove `.env` files** before building for production
- Use **system environment variables** or **container configurations** for production deployments
- The `.env.example` file is safe to commit and should be kept as a template

#### Command Line Arguments

The server uses a flexible configuration system that allows you to:

1. Choose your search mode at runtime
2. Configure different backends independently
3. Set appropriate limits and thresholds for each search type
4. Use environment variables for sensitive configuration (like API keys and database credentials)
5. Configure external services for embedding and keyword extraction

### Dependencies

- **Qdrant**: Vector database for semantic search
- **TiDB**: MySQL-compatible database for full-text search
- **Chat Service**: External service for intelligent keyword extraction
- **Embedding Service**: External service for vector generation
