# Using KuiperDb with LM Studio for Embeddings

This guide shows you how to use KuiperDb with LM Studio to generate real vector embeddings for semantic search.

## Setup

### 1. Install LM Studio

Download and install LM Studio from [https://lmstudio.ai/](https://lmstudio.ai/)

### 2. Download an Embedding Model

In LM Studio:
1. Click the **Search** icon (🔍)
2. Search for "nomic-embed" or "bge-"
3. Download one of these recommended models:
   - **nomic-ai/nomic-embed-text-v1.5-GGUF** (768 dimensions) - Recommended
   - **BAAI/bge-small-en-v1.5-GGUF** (384 dimensions) - Smaller/faster
   - **BAAI/bge-base-en-v1.5-GGUF** (768 dimensions)

### 3. Start the Local Server

1. Go to the **Local Server** tab (↔️) in LM Studio
2. Load your embedding model
3. Click **Start Server**
4. Note the URL (usually `http://localhost:1234`)

### 4. Update the Example Code

If your LM Studio is running on a different port or you're using a different embedding dimension:

```rust
// In examples/with_embeddings.rs
let embedding_url = "http://localhost:1234"; // Just the base URL, change port if needed
let embedding_dimensions = 768; // Match your model's dimension
```

**Note:** Use only the base URL (`http://localhost:1234`), not the full endpoint path. The embedder automatically adds `/v1/embeddings`.

Common embedding dimensions:
- **nomic-embed-text**: 768
- **bge-small**: 384
- **bge-base**: 768
- **bge-large**: 1024

### 5. Run the Example

```bash
cargo run --example with_embeddings
```

## Expected Output

```
🚀 KuiperDb with Real Embeddings Example

✅ DocumentStore initialized

🔗 Connecting to embedding server at http://localhost:1234/v1/embeddings...
   (Make sure LM Studio is running with an embedding model loaded)

🧪 Testing embedder... ✅ Success! (got 768 dimensional vector)

📝 Adding documents with embeddings...

  Adding: Rust Programming... ✅
  Adding: Python Overview... ✅
  Adding: Machine Learning... ✅
  Adding: Web Development... ✅
  Adding: Database Systems... ✅

🔍 Performing semantic searches...

Query: 'fast compiled programming language'
  Results:
    1. Rust Programming (score: 0.9234)
       Vector: 0.8456 | FTS: 0.0778
    ...
```

## How It Works

1. **Embedding Generation**: Each document's content is sent to LM Studio's API
2. **Vector Storage**: The embedding vectors (768-dimensional arrays) are stored in SQLite
3. **Semantic Search**: Query text is embedded, then compared against document vectors using cosine similarity
4. **Hybrid Search**: Combines vector similarity with fulltext search for best results

## Troubleshooting

### "Failed to connect" error

- ✅ Make sure LM Studio is running
- ✅ Check the Local Server tab is active and shows "Server Running"
- ✅ Verify the port number matches (check LM Studio's server tab)
- ✅ Ensure an embedding model is loaded (not a chat model)

### "Dimension mismatch" error

The `embedding_dimensions` in the code must match your model's output dimension:
- Check your model's documentation
- Common dimensions: 384, 768, 1024

### Slow embedding generation

- Use a smaller model (bge-small-en is faster than bge-large-en)
- Reduce the number of documents
- Use GPU acceleration if available in LM Studio

## Alternative Embedding Servers

You can also use these OpenAI-compatible embedding servers:

- **Ollama**: `http://localhost:11434` (uses `/api/embeddings`)
- **text-embeddings-inference**: `http://localhost:8080` (or custom port)
- **FastEmbed**: Custom deployment base URL
- **OpenAI API**: `https://api.openai.com` (requires API key)

Just update the `embedding_url` (base URL only) and `embedding_dimensions` accordingly.

## Performance Tips

1. **Batch Processing**: For large datasets, consider batching documents
2. **Caching**: KuiperDb automatically caches embeddings to avoid re-computation
3. **Model Selection**: Smaller models (384d) are faster but less accurate than larger ones (768d, 1024d)
4. **Index Threshold**: For >1000 documents, enable HNSW indexing for faster vector search

## Next Steps

- Try different embedding models to see quality differences
- Experiment with hybrid vs vector-only search
- Add your own documents and queries
- Build a semantic search application!
