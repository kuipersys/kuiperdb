/**
 * KuiperDb TypeScript Client
 */

import axios, { AxiosInstance, AxiosError } from 'axios';
import type {
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

export class KuiperDbClient {
  private api: AxiosInstance;

  constructor(config: KuiperDbClientConfig) {
    this.api = axios.create({
      baseURL: config.baseURL,
      timeout: config.timeout || 30000,
      headers: {
        'Content-Type': 'application/json',
        ...config.headers,
      },
    });
  }

  // ===== Database Operations =====

  /**
   * List all databases
   */
  async getDatabases(): Promise<Database[]> {
    const response = await this.api.get('/db');
    return response.data.databases || [];
  }

  /**
   * Delete a database
   */
  async deleteDatabase(dbName: string): Promise<void> {
    await this.api.delete(`/db/${dbName}`);
  }

  // ===== Table Operations =====

  /**
   * List all tables in a database
   */
  async getTables(dbName: string): Promise<Table[]> {
    const response = await this.api.get(`/db/${dbName}/tables`);
    return response.data.tables || [];
  }

  /**
   * Delete a table
   */
  async deleteTable(dbName: string, tableName: string): Promise<void> {
    await this.api.delete(`/db/${dbName}/${tableName}`);
  }

  // ===== Document Operations =====

  /**
   * List all documents in a table
   */
  async getDocuments(dbName: string, tableName: string): Promise<Document[]> {
    const response = await this.api.get(`/db/${dbName}/${tableName}/documents`);
    return response.data.documents || [];
  }

  /**
   * Get a specific document by ID
   */
  async getDocument(dbName: string, tableName: string, docId: string): Promise<Document | null> {
    try {
      const response = await this.api.get(`/db/${dbName}/${tableName}/${docId}`);
      return response.data;
    } catch (error) {
      if (axios.isAxiosError(error) && error.response?.status === 404) {
        return null;
      }
      throw error;
    }
  }

  /**
   * Store a new document or update an existing one
   */
  async storeDocument(
    dbName: string,
    tableName: string,
    document: StoreDocumentRequest
  ): Promise<Document> {
    const response = await this.api.post(`/db/${dbName}/${tableName}`, document);
    return response.data;
  }

  /**
   * Delete a document by ID
   */
  async deleteDocument(dbName: string, tableName: string, docId: string): Promise<void> {
    await this.api.delete(`/db/${dbName}/${tableName}/${docId}`);
  }

  /**
   * Get chunks for a document (if it was chunked)
   */
  async getChunks(dbName: string, tableName: string, docId: string): Promise<Document[]> {
    try {
      const response = await this.api.get(`/db/${dbName}/${tableName}/${docId}/chunks`);
      return Array.isArray(response.data) ? response.data : [];
    } catch (error) {
      console.error('Error fetching chunks:', error);
      return [];
    }
  }

  /**
   * Rechunk a document with a new strategy
   */
  async rechunkDocument(
    dbName: string,
    tableName: string,
    docId: string,
    strategy?: string
  ): Promise<Document[]> {
    const response = await this.api.post(`/db/${dbName}/${tableName}/${docId}/rechunk`, {
      strategy,
    });
    return response.data;
  }

  // ===== Search Operations =====

  /**
   * Semantic search using vector embeddings
   */
  async search(dbName: string, tableName: string, request: SearchRequest): Promise<SearchResult[]> {
    const response = await this.api.post(`/db/${dbName}/${tableName}/search`, request);
    return response.data.results || [];
  }

  // ===== Relation Operations =====

  /**
   * Create a relation between two documents
   */
  async createRelation(dbName: string, relation: CreateRelationRequest): Promise<DocumentRelation> {
    const response = await this.api.post(`/db/${dbName}/relations`, relation);
    return response.data;
  }

  /**
   * Get a specific relation by ID
   */
  async getRelation(dbName: string, relationId: string): Promise<DocumentRelation | null> {
    try {
      const response = await this.api.get(`/db/${dbName}/relations/${relationId}`);
      return response.data;
    } catch (error) {
      if (axios.isAxiosError(error) && error.response?.status === 404) {
        return null;
      }
      throw error;
    }
  }

  /**
   * Delete a relation by ID
   */
  async deleteRelation(dbName: string, relationId: string): Promise<void> {
    await this.api.delete(`/db/${dbName}/relations/${relationId}`);
  }

  /**
   * Get all relations for a specific document
   */
  async getDocumentRelations(dbName: string, docId: string): Promise<DocumentRelation[]> {
    try {
      const response = await this.api.get(`/db/${dbName}/documents/${docId}/relations`);
      return Array.isArray(response.data) ? response.data : [];
    } catch (error) {
      console.error('Error fetching relations:', error);
      return [];
    }
  }

  // ===== Graph Operations =====

  /**
   * Traverse the document graph from a starting node
   */
  async graphTraverse(
    dbName: string,
    request: GraphTraversalRequest
  ): Promise<GraphTraversalResult> {
    const response = await this.api.post(`/db/${dbName}/graph/traverse`, request);
    return response.data;
  }

  /**
   * Get graph statistics for a database
   */
  async getGraphStats(dbName: string): Promise<GraphStatistics> {
    const response = await this.api.get(`/db/${dbName}/graph/stats`);
    return response.data;
  }

  // ===== Health Check =====

  /**
   * Check if the KuiperDb server is healthy
   */
  async healthCheck(): Promise<boolean> {
    try {
      await this.api.get('/health');
      return true;
    } catch {
      return false;
    }
  }
}

/**
 * Create a new KuiperDb client instance
 */
export function createClient(config: KuiperDbClientConfig): KuiperDbClient {
  return new KuiperDbClient(config);
}
