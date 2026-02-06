export interface TreeNode {
  id: string;
  label: string;
  type: 'database' | 'table' | 'document' | 'child';
  dbName?: string;
  tableName?: string;
  docId?: string;
  children?: TreeNode[];
  hasChildren?: boolean;
}

export interface SelectedItem {
  type: 'database' | 'table' | 'document';
  dbName?: string;
  tableName?: string;
  docId?: string;
}
