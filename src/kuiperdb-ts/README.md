# KuiperDB TypeScript client

The client speaks the vector-first KuiperDB server API. The application supplies prepared vectors.

```ts
import { createClient } from '@kuiperdb/client';

const db = createClient({ baseURL: 'http://localhost:17001' });
await db.createVectorSpace({
  name: 'semantic', dimensions: 3, distance_metric: 'cosine', normalization: 'unit',
});
await db.putRecord({
  id: 'record-1',
  payload: { text: 'application-owned content' },
  vectors: [{ space: 'semantic', values: [1, 0, 0] }],
});
const nearest = await db.search({ space: 'semantic', vector: [1, 0, 0], limit: 10 });
```
