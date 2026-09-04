# @kuiperdb/react

React context bindings for `@kuiperdb/client`.

```tsx
import { KuiperDbProvider, useKuiperDb } from '@kuiperdb/react';

function Results() {
  const { client } = useKuiperDb();
  // client.search({ space: 'semantic', vector: [1, 0, 0] })
  return null;
}

export function App() {
  return <KuiperDbProvider baseURL="http://localhost:17001"><Results /></KuiperDbProvider>;
}
```

See the [TypeScript client](../kuiperdb-ts/README.md) for the API.
