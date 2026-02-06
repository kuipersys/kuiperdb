import { Paper, ScrollArea, Title, Box, Text, Loader } from '@mantine/core';
import type { TreeNode } from '../types/index';
import { TreeNodeItem } from './TreeNodeItem';

interface TreeViewProps {
  data: TreeNode[];
  onSelect: (node: TreeNode) => void;
  onExpand: (node: TreeNode) => void;
  onDelete?: (node: TreeNode) => void;
  selectedId?: string;
  loading?: boolean;
}

export function TreeView({ data, onSelect, onExpand, onDelete, selectedId, loading }: TreeViewProps) {
  return (
    <Paper shadow="sm" p="md" withBorder style={{ height: '100%', display: 'flex', flexDirection: 'column' }}>
      <Title order={4} mb="md">Database Explorer</Title>
      <ScrollArea style={{ flex: 1 }}>
        {loading ? (
          <Box style={{ display: 'flex', justifyContent: 'center', padding: 20 }}>
            <Loader size="sm" />
          </Box>
        ) : data.length === 0 ? (
          <Text c="dimmed" size="sm">No databases found</Text>
        ) : (
          data.map(node => (
            <TreeNodeItem
              key={node.id}
              node={node}
              level={0}
              onSelect={onSelect}
              onExpand={onExpand}
              onDelete={onDelete}
              selectedId={selectedId}
            />
          ))
        )}
      </ScrollArea>
    </Paper>
  );
}
