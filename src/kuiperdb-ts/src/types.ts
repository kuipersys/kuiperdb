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
  vector?: number[];
  parent_id?: string | null;
  chunk_index?: number;
  created_at: string;
  updated_at: string;
  token_count?: number;
  is_vectorized: boolean;
}

export interface DocumentRelation {
  id: string;
  source_id: string;
  target_id: string;
  relation_type: string;
  metadata: Record<string, any>;
  created_at: string;
}

export interface Database {
  name: string;
}

export interface Table {
  name: string;
}

export interface SearchRequest {
  query: string;
  limit?: number;
  threshold?: number;
  include_metadata?: boolean;
}

export interface SearchResult {
  document: Document;
  score: number;
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
  content: string;
  metadata?: Record<string, any>;
  tags?: string[];
  parent_id?: string | null;
}

export interface KuiperDbClientConfig {
  baseURL: string;
  timeout?: number;
  headers?: Record<string, string>;
}
