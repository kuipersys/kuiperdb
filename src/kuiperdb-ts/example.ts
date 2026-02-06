/**
 * KuiperDb TypeScript Client - Example Usage
 */

import { createClient } from './src/index';

async function main() {
  // Create client
  const client = createClient({
    baseURL: 'http://localhost:8080',
  });

  // Check health
  console.log('Checking server health...');
  const healthy = await client.healthCheck();
  console.log(`Server healthy: ${healthy}`);

  // List databases
  console.log('\nListing databases...');
  const databases = await client.getDatabases();
  console.log('Databases:', databases.map(db => db.name));

  const dbName = 'dnd_campaign';
  const tableName = 'characters';

  // List tables
  console.log(`\nListing tables in ${dbName}...`);
  const tables = await client.getTables(dbName);
  console.log('Tables:', tables.map(t => t.name));

  // Get documents
  console.log(`\nGetting documents from ${tableName}...`);
  const docs = await client.getDocuments(dbName, tableName);
  console.log(`Found ${docs.length} documents`);

  if (docs.length > 0) {
    const doc = docs[0];
    console.log(`\nFirst document: ${doc.id}`);
    console.log(`  Content preview: ${doc.content.substring(0, 100)}...`);
    console.log(`  Token count: ${doc.token_count}`);
    console.log(`  Vectorized: ${doc.is_vectorized}`);

    // Get relations
    console.log(`\nGetting relations for document ${doc.id}...`);
    const relations = await client.getDocumentRelations(dbName, doc.id);
    console.log(`Found ${relations.length} relations:`);
    relations.forEach(rel => {
      console.log(`  - ${rel.relation_type}: ${rel.source_id} → ${rel.target_id}`);
    });

    // Get graph stats
    console.log(`\nGetting graph statistics...`);
    const stats = await client.getGraphStats(dbName);
    console.log(`  Nodes: ${stats.node_count}`);
    console.log(`  Edges: ${stats.edge_count}`);
    console.log(`  Has cycles: ${stats.has_cycles}`);

    // Traverse graph
    if (stats.node_count > 0) {
      console.log(`\nTraversing graph from ${doc.id}...`);
      const traversal = await client.graphTraverse(dbName, {
        start_id: doc.id,
        max_depth: 2,
      });
      console.log(`  Reachable documents: ${traversal.document_ids.length}`);
      console.log(`  Relations in subgraph: ${traversal.relations.length}`);
    }
  }

  // Example: Store a new document
  console.log('\nStoring a new document...');
  const newDoc = await client.storeDocument(dbName, 'test_table', {
    content: '# Test Document\n\nThis is a test document created by the TypeScript client.',
    metadata: { 
      created_by: 'example.ts',
      timestamp: new Date().toISOString(),
    },
    tags: ['test', 'example'],
  });
  console.log(`Created document: ${newDoc.id}`);

  // Example: Create a relation
  if (docs.length >= 2) {
    console.log('\nCreating a relation...');
    const rel = await client.createRelation(dbName, {
      source_id: docs[0].id,
      target_id: docs[1].id,
      relation_type: 'EXAMPLE_RELATION',
      metadata: { note: 'Created by example script' },
    });
    console.log(`Created relation: ${rel.id}`);
  }

  console.log('\n✓ Example completed!');
}

main().catch(console.error);
