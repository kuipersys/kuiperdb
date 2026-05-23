/**
 * KuiperDb TypeScript Client - Type Definitions
 */

export interface Document {
  id: string;
  db: string;
  table: string;
  content: string;
  metadata: Record<string, any>;
  tags: string[];
  parent_id?: string | null;
  chunk_index?: number;
  created_at: number;
  updated_at: number;
  token_count?: number;
  is_vectorized: boolean;
}

export interface DocumentRelation {
  id: string;
  source_id: string;
  target_id: string;
  relation_type: string;
  metadata: Record<string, any>;
  created_at: number;
}

export interface Database {
  name: string;
}

export interface Table {
  name: string;
}

export interface SearchRequest {
  query: string;
  type?: 'vector' | 'fulltext' | 'hybrid';
  limit?: number;
  filters?: Record<string, any>;
  include_chunks?: boolean;
  group_by_parent?: boolean;
}

export interface SearchResult {
  id: string;
  content: string;
  metadata: Record<string, any>;
  score: number;
  fts_rank?: number | null;
  vector_similarity?: number | null;
  is_chunk: boolean;
  parent_id?: string | null;
  chunk_index?: number | null;
}

export interface GraphTraversalRequest {
  start_id: string;
  max_depth: number;
  relation_types?: string[];
}

export interface GraphTraversalResult {
  document_ids: string[];
  relations: DocumentRelation[];
  depth_map: Record<string, number>;
}

export interface GraphStatistics {
  node_count: number;
  edge_count: number;
  has_cycles: boolean;
  in_degrees: Record<string, number>;
  out_degrees: Record<string, number>;
}

export interface CreateRelationRequest {
  source_id: string;
  target_id: string;
  relation_type: string;
  metadata?: Record<string, any>;
}

export interface StoreDocumentRequest {
  id?: string;
  content: string;
  metadata?: Record<string, any>;
  tags?: string[];
  vectorize?: boolean;
}

export interface StoreDocumentOptions {
  asyncEmbedding?: boolean;
}

export interface KuiperDbClientConfig {
  baseURL: string;
  timeout?: number;
  headers?: Record<string, string>;
}
