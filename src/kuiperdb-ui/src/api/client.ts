/**
 * Legacy client export - now uses @kuiperdb/client package
 * 
 * This module re-exports types and creates a default client instance
 * for backwards compatibility. New code should use the KuiperDbProvider
 * and useKuiperDb hook instead.
 */

import { createClient } from '@kuiperdb/client';

// Re-export types from @kuiperdb/client
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
} from '@kuiperdb/client';

// Create a default client instance for backwards compatibility
const defaultClient = createClient({ baseURL: '/' });

// Export legacy client interface
export const kuiperdbClient = {
  getDatabases: () => defaultClient.getDatabases(),
  getTables: (dbName: string) => defaultClient.getTables(dbName),
  getDocuments: (dbName: string, tableName: string) => defaultClient.getDocuments(dbName, tableName),
  getDocument: (dbName: string, tableName: string, docId: string) => defaultClient.getDocument(dbName, tableName, docId),
  getDocumentRelations: (dbName: string, docId: string) => defaultClient.getDocumentRelations(dbName, docId),
  healthCheck: () => defaultClient.healthCheck(),
  getChunks: (dbName: string, tableName: string, docId: string) => defaultClient.getChunks(dbName, tableName, docId),
  deleteDocument: (dbName: string, tableName: string, docId: string) => defaultClient.deleteDocument(dbName, tableName, docId),
  deleteTable: (dbName: string, tableName: string) => defaultClient.deleteTable(dbName, tableName),
  deleteDatabase: (dbName: string) => defaultClient.deleteDatabase(dbName),
  getGraphStats: (dbName: string) => defaultClient.getGraphStats(dbName),
};
