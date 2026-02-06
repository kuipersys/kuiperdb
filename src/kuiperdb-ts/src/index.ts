/**
 * KuiperDb TypeScript Client
 * 
 * A TypeScript/JavaScript client library for interacting with KuiperDb,
 * a vector database with document relations and graph capabilities.
 */

export { KuiperDbClient, createClient } from './client';
export type {
  Document,
  DocumentRelation,
  Database,
  Table,
  SearchRequest,
  SearchResult,
  GraphTraversalRequest,
  GraphTraversalResult,
  GraphStatistics,
  CreateRelationRequest,
  StoreDocumentRequest,
  KuiperDbClientConfig,
} from './types';
