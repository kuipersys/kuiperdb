import { useState } from 'react';
import { type TreeNode } from '../types';
import { kuiperdbClient, type Database } from '../api/client';

export function useTreeData() {
  const [treeData, setTreeData] = useState<TreeNode[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const loadDatabases = async () => {
    setLoading(true);
    setError(null);
    try {
      const databases = await kuiperdbClient.getDatabases();
      const nodes: TreeNode[] = databases.map((db: Database) => ({
        id: `db-${db.name}`,
        label: db.name,
        type: 'database' as const,
        dbName: db.name,
        hasChildren: true,
        children: [],
      }));
      setTreeData(nodes);
    } catch (err) {
      setError('Failed to load databases');
      console.error(err);
    } finally {
      setLoading(false);
    }
  };

  const expandNode = async (nodeId: string) => {
    // TODO: Implement node expansion for tables and documents
    console.log('Expand node:', nodeId);
  };

  return {
    treeData,
    loading,
    error,
    loadDatabases,
    expandNode,
  };
}
