/**
 * KuiperDb TypeScript Client
 * 
 * A TypeScript/JavaScript client library for interacting with KuiperDb,
 * a model-agnostic vector database.
 */

export { KuiperDbClient, createClient } from './client.js';
export type {
  IndexConfig,
  KuiperDbClientConfig,
  KuiperRecord,
  Metadata,
  MetadataFilter,
  NamedVector,
  RecordInput,
  Relation,
  RelationInput,
  SearchResult,
  VectorQuery,
  VectorSpace,
} from './types.js';
