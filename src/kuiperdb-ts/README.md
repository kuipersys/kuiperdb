# KuiperDb TypeScript Client

A TypeScript/JavaScript client library for KuiperDb - a vector database with document relations and graph capabilities.

## Installation

```bash
npm install KuiperDb
# or
yarn add KuiperDb
```

## Quick Start

```typescript
import { createClient } from 'KuiperDb';

const client = createClient({
  baseURL: 'http://localhost:8080',
});

// Store a document
const doc = await client.storeDocument('my_db', 'articles', {
  content: 'This is my article...',
  metadata: { author: 'John' },
  tags: ['typescript'],
});

// Search
const results = await client.search('my_db', 'articles', {
  query: 'typescript article',
  limit: 10,
});

// Create relations
await client.createRelation('my_db', {
  source_id: doc1.id,
  target_id: doc2.id,
  relation_type: 'REFERENCES',
});

// Traverse graph
const graph = await client.graphTraverse('my_db', {
  start_id: doc.id,
  max_depth: 2,
});
```

See full documentation at [https://github.com/tsharp/KuiperDb](https://github.com/tsharp/KuiperDb)
