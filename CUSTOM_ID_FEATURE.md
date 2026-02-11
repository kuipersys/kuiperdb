# Custom Document ID Feature

## Summary
The issue was that the Rust client library wasn't sending custom IDs to the server, even though the backend supported them.

## Changes Made

### Updated: src/kuiperdb-rs/src/client.rs

1. **AddDocumentRequest struct** - Added optional fields:
   - \id: Option<String>\ - Custom document ID
   - \	ags: Option<Vec<String>>\ - Document tags  
   - \ectorize: Option<bool>\ - Embedding control

2. **New method: add_document_with_options()** - Full control over document creation:
   - Custom ID support
   - Tags support
   - Vectorize control

3. **Existing method: add_document()** - Still works, auto-generates UUID

## Usage Examples

### Auto-generated ID (existing behavior)
\\\ust
let doc_id = client.add_document(
    "My content".to_string(),
    None
).await?;
// doc_id will be a UUID like "550e8400-e29b-41d4-a716-446655440000"
\\\

### Custom ID (new feature)
\\\ust
let doc_id = client.add_document_with_options(
    Some("my-custom-id-123".to_string()),  // Custom ID
    "My content".to_string(),
    None,  // metadata
    Some(vec!["tag1".to_string()]),  // tags
    Some(true)  // vectorize
).await?;
// doc_id will be "my-custom-id-123"
\\\

### Via HTTP API directly
\\\json
POST /db/{db_name}/{table_name}
{
  "id": "my-custom-id-123",
  "content": "Document content",
  "metadata": {},
  "tags": ["tag1"],
  "vectorize": true
}
\\\

## Backend Flow
The backend (in \kuiperdb-core/src/store.rs\) already had the correct logic:
\\\ust
let doc_id = request.id.unwrap_or_else(|| Uuid::new_v4().to_string());
\\\

This ensures:
- If client provides an ID → use it
- If client doesn't provide an ID → generate UUID
- ID is always stored as a String type
