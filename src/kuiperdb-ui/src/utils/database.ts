import { type Database, type Table } from '../api/client';
import { type TreeNode } from '../types';

// Since the API doesn't fully support listing databases/tables yet,
// we'll work with what we can query from the file system
// In production, these would come from the API

export async function scanDataDirectory(): Promise<Database[]> {
  // For now, return empty - the API endpoint list_databases returns empty
  // In a real scenario, you'd need to implement backend endpoints
  return [];
}

export async function getDatabaseTables(_dbName: string): Promise<Table[]> {
  // This would need a backend API endpoint
  // For now, we'll simulate with known tables
  return [];
}

export function createTreeFromDatabases(databases: Database[]): TreeNode[] {
  return databases.map(db => ({
    id: `db-${db.name}`,
    label: db.name,
    type: 'database' as const,
    dbName: db.name,
    hasChildren: true,
    children: [],
  }));
}

export function createDocumentNode(
  doc: any,
  dbName: string,
  tableName: string
): TreeNode {
  return {
    id: `doc-${doc.id}`,
    label: doc.id.substring(0, 8) + '...',
    type: 'document' as const,
    dbName,
    tableName,
    docId: doc.id,
    hasChildren: !!doc.parent_id === false, // Only roots can have children
  };
}
