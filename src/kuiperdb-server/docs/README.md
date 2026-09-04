# KuiperDB HTTP API

The server is a thin adapter around one embedded `Database`. It accepts externally prepared vectors and performs no parsing, chunking, model inference, or embedding caching.

| Method | Path | Purpose |
|---|---|---|
| `GET` | `/health` | Liveness |
| `POST` / `GET` | `/spaces` | Create/list vector spaces |
| `DELETE` | `/spaces/{name}` | Delete a space and its vectors |
| `POST` | `/records` | Atomically upsert a record and supplied vectors |
| `POST` | `/records/bulk` | Atomic batch upsert |
| `GET` / `DELETE` | `/records/{id}` | Read/delete a record |
| `GET` | `/records?limit=100&offset=0` | List records |
| `POST` | `/search` | Nearest-record query from a vector |
| `POST` | `/relations` | Create a generic relation |
| `GET` | `/records/{id}/relations` | List incoming/outgoing relations |
| `DELETE` | `/relations/{id}` | Delete a relation |
| `GET` | `/spaces/{name}/index` | Inspect index generation/freshness |
| `PUT` | `/spaces/{name}/index` | Change configuration and invalidate loaded indexes |
| `POST` | `/spaces/{name}/index/rebuild` | Explicitly rebuild derived ANN state |

Example:

```bash
curl -X POST http://localhost:17001/spaces -H "content-type: application/json" \
  -d '{"name":"semantic","dimensions":3,"distance_metric":"cosine","normalization":"unit"}'

curl -X POST http://localhost:17001/records -H "content-type: application/json" \
  -d '{"id":"r1","payload":{"text":"prepared externally"},"vectors":[{"space":"semantic","values":[1,0,0]}]}'

curl -X POST http://localhost:17001/search -H "content-type: application/json" \
  -d '{"space":"semantic","vector":[1,0,0],"limit":10,"filter":{}}'
```

Configure the process with `KUIPERDB_PATH` and `KUIPERDB_BIND`.
