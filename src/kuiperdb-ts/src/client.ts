import axios, { type AxiosInstance } from 'axios';
import type {
  IndexConfig, KuiperDbClientConfig, KuiperRecord, RecordInput, Relation, RelationInput,
  SearchResult, VectorQuery, VectorSpace,
} from './types.js';

export class KuiperDbClient {
  private readonly api: AxiosInstance;

  constructor(config: KuiperDbClientConfig) {
    this.api = axios.create({
      baseURL: config.baseURL,
      timeout: config.timeout ?? 30_000,
      headers: { 'Content-Type': 'application/json', ...config.headers },
    });
  }

  async createVectorSpace(space: VectorSpace): Promise<void> {
    await this.api.post('/spaces', space);
  }

  async getVectorSpaces(): Promise<VectorSpace[]> {
    return (await this.api.get<VectorSpace[]>('/spaces')).data;
  }

  async deleteVectorSpace(name: string): Promise<void> {
    await this.api.delete(`/spaces/${encodeURIComponent(name)}`);
  }

  async updateIndexConfig(name: string, config: IndexConfig): Promise<void> {
    await this.api.put(`/spaces/${encodeURIComponent(name)}/index`, config);
  }

  async putRecord(input: RecordInput): Promise<KuiperRecord> {
    return (await this.api.post<KuiperRecord>('/records', input)).data;
  }

  async putRecords(inputs: RecordInput[]): Promise<KuiperRecord[]> {
    return (await this.api.post<KuiperRecord[]>('/records/bulk', inputs)).data;
  }

  async getRecord(id: string): Promise<KuiperRecord | null> {
    try {
      return (await this.api.get<KuiperRecord>(`/records/${encodeURIComponent(id)}`)).data;
    } catch (error) {
      if (axios.isAxiosError(error) && error.response?.status === 404) return null;
      throw error;
    }
  }

  async getRecords(limit = 100, offset = 0): Promise<KuiperRecord[]> {
    return (await this.api.get<KuiperRecord[]>('/records', { params: { limit, offset } })).data;
  }

  async deleteRecord(id: string): Promise<void> {
    await this.api.delete(`/records/${encodeURIComponent(id)}`);
  }

  async search(query: VectorQuery): Promise<SearchResult[]> {
    return (await this.api.post<SearchResult[]>('/search', query)).data;
  }

  async createRelation(input: RelationInput): Promise<Relation> {
    return (await this.api.post<Relation>('/relations', input)).data;
  }

  async getRelations(recordId: string): Promise<Relation[]> {
    return (await this.api.get<Relation[]>(`/records/${encodeURIComponent(recordId)}/relations`)).data;
  }

  async deleteRelation(id: string): Promise<void> {
    await this.api.delete(`/relations/${encodeURIComponent(id)}`);
  }

  async healthCheck(): Promise<boolean> {
    try { await this.api.get('/health'); return true; } catch { return false; }
  }
}

export function createClient(config: KuiperDbClientConfig): KuiperDbClient {
  return new KuiperDbClient(config);
}
