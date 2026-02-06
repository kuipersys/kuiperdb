import { Paper, Title, Text, Box, ScrollArea, Badge, Group, Stack, Code } from '@mantine/core';
import { IconCheck, IconX } from '@tabler/icons-react';
import type { Document } from '../api/client';
import { useEffect, useState } from 'react';
import { kuiperdbClient } from '../api/client';
import type { TreeNode } from '../types/index';

interface ContentViewerProps {
  selectedNode: TreeNode | null;
}

export function ContentViewer({ selectedNode }: ContentViewerProps) {
  const [document, setDocument] = useState<Document | null>(null);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (selectedNode?.type === 'document' && selectedNode.dbName && selectedNode.tableName && selectedNode.docId) {
      setLoading(true);
      kuiperdbClient
        .getDocument(selectedNode.dbName, selectedNode.tableName, selectedNode.docId)
        .then(setDocument)
        .catch(console.error)
        .finally(() => setLoading(false));
    } else {
      setDocument(null);
    }
  }, [selectedNode]);

  if (!selectedNode) {
    return (
      <Paper shadow="sm" p="md" withBorder style={{ height: '100%' }}>
        <Text c="dimmed">Select an item from the tree to view its contents</Text>
      </Paper>
    );
  }

  return (
    <Paper shadow="sm" p="md" withBorder style={{ height: '100%', display: 'flex', flexDirection: 'column' }}>
      <Title order={4} mb="md">
        {selectedNode.type === 'database' && `Database: ${selectedNode.dbName}`}
        {selectedNode.type === 'table' && `Table: ${selectedNode.tableName}`}
        {selectedNode.type === 'document' && `Document: ${selectedNode.docId?.substring(0, 8)}...`}
      </Title>

      <ScrollArea style={{ flex: 1 }}>
        {loading ? (
          <Text>Loading...</Text>
        ) : selectedNode.type === 'document' && document ? (
          <Stack gap="md">
            <Box>
              <Text size="sm" fw={600} mb={4}>ID</Text>
              <Code block>{document.id}</Code>
            </Box>

            <Box>
              <Text size="sm" fw={600} mb={4}>Database / Table</Text>
              <Group gap="xs">
                <Badge>{document.db}</Badge>
                <Text size="sm">/</Text>
                <Badge>{document.table}</Badge>
              </Group>
            </Box>

            <Box>
              <Text size="sm" fw={600} mb={4}>Stats</Text>
              <Group gap="md">
                {document.token_count && (
                  <Badge variant="light" color="blue">
                    {document.token_count.toLocaleString()} tokens
                  </Badge>
                )}
                <Badge 
                  variant="light" 
                  color={document.is_vectorized ? "green" : "gray"}
                  leftSection={document.is_vectorized ? <IconCheck size={14} /> : <IconX size={14} />}
                >
                  {document.is_vectorized ? "Vectorized" : "Not Vectorized"}
                </Badge>
              </Group>
            </Box>

            {document.tags && document.tags.length > 0 && (
              <Box>
                <Text size="sm" fw={600} mb={4}>Tags</Text>
                <Group gap="xs">
                  {document.tags.map(tag => (
                    <Badge key={tag} variant="light">{tag}</Badge>
                  ))}
                </Group>
              </Box>
            )}

            <Box>
              <Text size="sm" fw={600} mb={4}>Content</Text>
              <Paper p="md" withBorder style={{ whiteSpace: 'pre-wrap', backgroundColor: '#f8f9fa' }}>
                {document.content}
              </Paper>
            </Box>

            {document.parent_id && (
              <Box>
                <Text size="sm" fw={600} mb={4}>Parent ID</Text>
                <Code block>{document.parent_id}</Code>
              </Box>
            )}

            {document.metadata && Object.keys(document.metadata).length > 0 && (
              <Box>
                <Text size="sm" fw={600} mb={4}>Metadata</Text>
                <Code block>{JSON.stringify(document.metadata, null, 2)}</Code>
              </Box>
            )}

            <Box>
              <Text size="sm" fw={600} mb={4}>Timestamps</Text>
              <Stack gap="xs">
                <Text size="sm">Created: {new Date(document.created_at).toLocaleString()}</Text>
                <Text size="sm">Updated: {new Date(document.updated_at).toLocaleString()}</Text>
              </Stack>
            </Box>

            {document.vector && (
              <Box>
                <Text size="sm" fw={600} mb={4}>Vector Embedding</Text>
                <Text size="sm" c="dimmed">
                  Dimension: {document.vector.length}
                </Text>
              </Box>
            )}
          </Stack>
        ) : selectedNode.type === 'database' ? (
          <Text c="dimmed">Database information will be displayed here</Text>
        ) : selectedNode.type === 'table' ? (
          <Text c="dimmed">Table information will be displayed here</Text>
        ) : null}
      </ScrollArea>
    </Paper>
  );
}
