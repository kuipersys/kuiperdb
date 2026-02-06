# KuiperDb UI Provider

The KuiperDb UI now uses the `@kuiperdb/client` TypeScript client package via a React Context Provider pattern.

## Architecture

```
@kuiperdb/client (TypeScript Client)
    ↓
KuiperDbProvider (React Context)
    ↓
useKuiperDb() hook
    ↓
UI Components
```

## Usage

### 1. Provider Setup

The provider is already configured in `src/main.tsx`:

```tsx
import { KuiperDbProvider } from './providers';

<KuiperDbProvider baseURL="/">
  <App />
</KuiperDbProvider>
```

### 2. Using the Client in Components

Use the `useKuiperDb()` hook to access the KuiperDb client:

```tsx
import { useKuiperDb } from '../providers';

function MyComponent() {
  const { client } = useKuiperDb();
  
  // Use client methods
  const databases = await client.getDatabases();
  const documents = await client.getDocuments(db, table);
  const results = await client.search(db, table, { query: 'search term' });
  
  // ... etc
}
```

### 3. Available Client Methods

The `KuiperDbClient` provides all API operations:

**Database Operations:**
- `getDatabases()` - List all databases
- `deleteDatabase(dbName)` - Delete a database

**Table Operations:**
- `getTables(dbName)` - List tables in a database
- `deleteTable(dbName, tableName)` - Delete a table

**Document Operations:**
- `getDocuments(dbName, tableName)` - List all documents
- `getDocument(dbName, tableName, docId)` - Get specific document
- `storeDocument(dbName, tableName, document)` - Create/update document
- `deleteDocument(dbName, tableName, docId)` - Delete document
- `getChunks(dbName, tableName, docId)` - Get document chunks
- `rechunkDocument(dbName, tableName, docId, strategy?)` - Rechunk document

**Search Operations:**
- `search(dbName, tableName, request)` - Semantic vector search

**Relation Operations:**
- `createRelation(dbName, relation)` - Create document relation
- `getRelation(dbName, relationId)` - Get specific relation
- `deleteRelation(dbName, relationId)` - Delete relation
- `getDocumentRelations(dbName, docId)` - Get all relations for document

**Graph Operations:**
- `graphTraverse(dbName, request)` - Traverse document graph
- `getGraphStats(dbName)` - Get graph statistics

**Health:**
- `healthCheck()` - Check server health

### 4. With React Query

Example using with `@tanstack/react-query`:

```tsx
import { useQuery } from '@tanstack/react-query';
import { useKuiperDb } from '../providers';

function DatabaseList() {
  const { client } = useKuiperDb();
  
  const { data: databases, isLoading } = useQuery({
    queryKey: ['databases'],
    queryFn: () => client.getDatabases(),
  });
  
  if (isLoading) return <div>Loading...</div>;
  
  return (
    <div>
      {databases?.map(db => (
        <div key={db.name}>{db.name}</div>
      ))}
    </div>
  );
}
```

## Backwards Compatibility

The old `src/api/client.ts` has been updated to re-export from `@kuiperdb/client` for backwards compatibility. Existing code will continue to work, but new code should use the provider pattern.

## Benefits

1. **Centralized Configuration** - Single place to configure base URL, timeout, headers
2. **Type Safety** - Full TypeScript support with exported types
3. **Reusability** - Shared client package between UI and other TypeScript projects
4. **Testing** - Easy to mock the provider for testing components
5. **Performance** - Single client instance shared across all components
