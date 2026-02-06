# @kuiperdb/react

React components and hooks for KuiperDb - a vector database with document relations.

## Installation

```bash
npm install @kuiperdb/react @kuiperdb/client
```

## Usage

### Provider Setup

Wrap your app with the `KuiperDbProvider`:

```tsx
import { KuiperDbProvider } from '@kuiperdb/react';

function App() {
  return (
    <KuiperDbProvider baseURL="http://localhost:8080">
      <YourComponents />
    </KuiperDbProvider>
  );
}
```

### Using the Client Hook

Access the KuiperDb client in any component:

```tsx
import { useKuiperDb } from '@kuiperdb/react';

function DatabaseList() {
  const { client } = useKuiperDb();
  
  useEffect(() => {
    const fetchDatabases = async () => {
      const databases = await client.getDatabases();
      console.log(databases);
    };
    fetchDatabases();
  }, [client]);
  
  return <div>...</div>;
}
```

### With React Query

```tsx
import { useQuery } from '@tanstack/react-query';
import { useKuiperDb } from '@kuiperdb/react';

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

## API

### `KuiperDbProvider`

**Props:**
- `children: ReactNode` - Child components
- `baseURL?: string` - KuiperDb server URL (default: '/')
- `timeout?: number` - Request timeout in milliseconds (default: 30000)

### `useKuiperDb()`

Returns an object with:
- `client: KuiperDbClient` - The KuiperDb client instance

See [@kuiperdb/client](../kuiperdb-ts/README.md) for full client API documentation.

## License

MIT
