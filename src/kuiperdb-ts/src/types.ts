export type Metadata = Record<string, unknown>;

export interface IndexConfig {
  enabled: boolean;
  hnsw_m: number;
  hnsw_ef_construction: number;
  hnsw_ef_search: number;
}

export interface VectorSpace {
  name: string;
  dimensions: number;
  distance_metric: 'cosine' | 'euclidean' | 'dot_product';
  normalization?: 'none' | 'unit';
  index?: Partial<IndexConfig>;
}

export interface NamedVector {
  space: string;
  values: number[];
}

export interface KuiperRecord {
  id: string;
  payload?: unknown;
  metadata: Metadata;
  created_at: string;
  updated_at: string;
}

export interface RecordInput {
  id?: string;
  payload?: unknown;
  metadata?: Metadata;
  vectors?: NamedVector[];
}

export interface MetadataFilter { [key: string]: unknown }

export interface VectorQuery {
  space: string;
  vector: number[];
  limit?: number;
  filter?: MetadataFilter;
}

export interface SearchResult {
  record: KuiperRecord;
  distance: number;
}

export interface Relation {
  id: string;
  source_id: string;
  target_id: string;
  kind: string;
  metadata: Metadata;
  created_at: string;
}

export interface RelationInput {
  id?: string;
  source_id: string;
  target_id: string;
  kind: string;
  metadata?: Metadata;
}

export interface KuiperDbClientConfig {
  baseURL: string;
  timeout?: number;
  headers?: Record<string, string>;
}
