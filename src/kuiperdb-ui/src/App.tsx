import '@mantine/core/styles.css';
import '@mantine/notifications/styles.css';
import { AppShell, Container, Title, Grid, Button, Group, Alert, Tabs } from '@mantine/core';
import { modals } from '@mantine/modals';
import { notifications } from '@mantine/notifications';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { useState, useEffect } from 'react';
import { TreeView } from './components/TreeView';
import { ContentViewer } from './components/ContentViewer';
import { GraphView } from './components/GraphView';
import { RecordManager } from './components/RecordManager';
import type { TreeNode } from './types/index';
import { kuiperdbClient, type Document } from './api/client';
import { IconAlertCircle, IconRefresh, IconFileText, IconNetwork, IconDatabaseSearch } from '@tabler/icons-react';

const queryClient = new QueryClient();

function getErrorMessage(error: unknown): string {
  if (typeof error === 'object' && error !== null) {
    const maybeError = error as {
      response?: { data?: { message?: string; error?: string } };
      message?: string;
    };

    return (
      maybeError.response?.data?.message ||
      maybeError.response?.data?.error ||
      maybeError.message ||
      'Request failed'
    );
  }

  return 'Request failed';
}

function AppContent() {
  const [treeData, setTreeData] = useState<TreeNode[]>([]);
  const [selectedNode, setSelectedNode] = useState<TreeNode | null>(null);
  const [loading, setLoading] = useState(false);
  const [healthStatus, setHealthStatus] = useState<boolean | null>(null);
  const [activeTab, setActiveTab] = useState<string>('records');

  const checkHealth = async () => {
    const healthy = await kuiperdbClient.healthCheck();
    setHealthStatus(healthy);
  };

  const loadDatabases = async () => {
    setLoading(true);
    try {
      const databases = await kuiperdbClient.getDatabases();
      const nodes: TreeNode[] = databases.map(db => ({
        id: `db-${db.name}`,
        label: db.name,
        type: 'database' as const,
        dbName: db.name,
        hasChildren: true,
        children: [],
      }));
      setTreeData(nodes);
    } catch (err) {
      console.error('Failed to load databases:', err);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    checkHealth();
    loadDatabases();
  }, []);

  const handleExpand = async (node: TreeNode) => {
    if (node.type === 'database' && node.dbName) {
      // Load tables for this database
      try {
        const tables = await kuiperdbClient.getTables(node.dbName);
        const tableNodes: TreeNode[] = tables.map(table => ({
          id: `table-${node.dbName}-${table.name}`,
          label: table.name,
          type: 'table' as const,
          dbName: node.dbName,
          tableName: table.name,
          hasChildren: true,
          children: [],
        }));
        
        // Update the tree data
        setTreeData(prevData => 
          prevData.map(db => 
            db.id === node.id ? { ...db, children: tableNodes } : db
          )
        );
      } catch (err) {
        console.error('Failed to load tables:', err);
      }
    } else if (node.type === 'table' && node.dbName && node.tableName) {
      // Load root documents for this table
      try {
        const documents = await kuiperdbClient.getDocuments(node.dbName, node.tableName);
        const docNodes: TreeNode[] = documents.map((doc: Document) => ({
          id: `doc-${doc.id}`,
          label: doc.id.substring(0, 12) + '...',
          type: 'document' as const,
          dbName: node.dbName,
          tableName: node.tableName,
          docId: doc.id,
          hasChildren: true, // Documents might have chunks
          children: [],
        }));
        
        // Update the tree data
        setTreeData(prevData => 
          prevData.map(db => ({
            ...db,
            children: db.children?.map(table => 
              table.id === node.id ? { ...table, children: docNodes } : table
            ),
          }))
        );
      } catch (err) {
        console.error('Failed to load documents:', err);
      }
    } else if (node.type === 'document' && node.dbName && node.tableName && node.docId) {
      // Load chunks for this document
      try {
        const chunks = await kuiperdbClient.getChunks(node.dbName, node.tableName, node.docId);
        const chunkNodes: TreeNode[] = chunks.map((chunk: Document) => ({
          id: `chunk-${chunk.id}`,
          label: `Chunk ${chunk.chunk_index} (${chunk.token_count} tokens)`,
          type: 'document' as const,
          dbName: node.dbName,
          tableName: node.tableName,
          docId: chunk.id,
          hasChildren: false,
        }));
        
        // Update the tree data
        setTreeData(prevData => 
          prevData.map(db => ({
            ...db,
            children: db.children?.map(table => ({
              ...table,
              children: table.children?.map(doc => 
                doc.id === node.id ? { ...doc, children: chunkNodes } : doc
              ),
            })),
          }))
        );
      } catch (err) {
        console.error('Failed to load chunks:', err);
      }
    }
  };

  const handleSelect = (node: TreeNode) => {
    setSelectedNode(node);
  };

  const createDocumentNode = (dbName: string, tableName: string, docId: string): TreeNode => ({
    id: `doc-${docId}`,
    label: docId.length > 12 ? `${docId.substring(0, 12)}...` : docId,
    type: 'document' as const,
    dbName,
    tableName,
    docId,
    hasChildren: true,
    children: [],
  });

  const createTableNode = (dbName: string, tableName: string, children: TreeNode[] = []): TreeNode => ({
    id: `table-${dbName}-${tableName}`,
    label: tableName,
    type: 'table' as const,
    dbName,
    tableName,
    hasChildren: true,
    children,
  });

  const createDatabaseNode = (dbName: string, children: TreeNode[] = []): TreeNode => ({
    id: `db-${dbName}`,
    label: dbName,
    type: 'database' as const,
    dbName,
    hasChildren: true,
    children,
  });

  const handleRecordSelected = (dbName: string, tableName: string, docId: string) => {
    setSelectedNode(createDocumentNode(dbName, tableName, docId));
    setActiveTab('content');
  };

  const handleRecordCreated = (document: Document) => {
    const docNode = createDocumentNode(document.db, document.table, document.id);

    setTreeData(prevData => {
      let foundDb = false;
      const nextData = prevData.map(db => {
        if (db.dbName !== document.db) {
          return db;
        }

        foundDb = true;
        let foundTable = false;
        const tableChildren = db.children || [];
        const nextTables = tableChildren.map(table => {
          if (table.tableName !== document.table) {
            return table;
          }

          foundTable = true;
          const existingDocs = table.children || [];
          return {
            ...table,
            hasChildren: true,
            children: [
              docNode,
              ...existingDocs.filter(doc => doc.docId !== document.id),
            ],
          };
        });

        if (!foundTable) {
          nextTables.push(createTableNode(document.db, document.table, [docNode]));
        }

        return {
          ...db,
          hasChildren: true,
          children: nextTables,
        };
      });

      if (!foundDb) {
        return [
          ...nextData,
          createDatabaseNode(document.db, [
            createTableNode(document.db, document.table, [docNode]),
          ]),
        ];
      }

      return nextData;
    });

    setSelectedNode(docNode);
    setActiveTab('content');
  };

  const handleSchemaChanged = (dbName: string, tableName?: string) => {
    const nextSelectedNode = tableName
      ? createTableNode(dbName, tableName)
      : createDatabaseNode(dbName);

    setTreeData(prevData => {
      let foundDb = false;
      const nextData = prevData.map(db => {
        if (db.dbName !== dbName) {
          return db;
        }

        foundDb = true;
        if (!tableName) {
          return db;
        }

        const tableChildren = db.children || [];
        const hasTable = tableChildren.some(table => table.tableName === tableName);

        return {
          ...db,
          hasChildren: true,
          children: hasTable
            ? tableChildren
            : [...tableChildren, createTableNode(dbName, tableName)],
        };
      });

      if (!foundDb) {
        return [
          ...nextData,
          createDatabaseNode(dbName, tableName ? [createTableNode(dbName, tableName)] : []),
        ];
      }

      return nextData;
    });

    setSelectedNode(nextSelectedNode);
  };

  const handleDelete = async (node: TreeNode) => {
    const confirmMessage = node.type === 'database'
      ? `Delete database "${node.label}" and ALL its tables and documents? This cannot be undone!`
      : node.type === 'table'
      ? `Delete table "${node.label}" and all its documents?`
      : `Delete document "${node.label}"${node.hasChildren ? ' and all its chunks?' : '?'}`;

    modals.openConfirmModal({
      title: `Delete ${node.type}`,
      children: confirmMessage,
      labels: { confirm: 'Delete', cancel: 'Cancel' },
      confirmProps: { color: 'red' },
      onConfirm: async () => {
        try {
          if (node.type === 'database' && node.dbName) {
            await kuiperdbClient.deleteDatabase(node.dbName);
            notifications.show({
              title: 'Database deleted',
              message: `Database "${node.label}" deleted successfully`,
              color: 'green',
            });
            // Remove database from tree
            setTreeData(prev => prev.filter(db => db.id !== node.id));
            if (selectedNode?.id === node.id) {
              setSelectedNode(null);
            }
          } else if (node.type === 'table' && node.dbName && node.tableName) {
            await kuiperdbClient.deleteTable(node.dbName, node.tableName);
            notifications.show({
              title: 'Table deleted',
              message: `Table "${node.label}" deleted successfully`,
              color: 'green',
            });
            // Remove table from tree
            setTreeData(prev => prev.map(db => {
              if (db.id === `db-${node.dbName}`) {
                return {
                  ...db,
                  children: db.children?.filter(t => t.id !== node.id) || [],
                };
              }
              return db;
            }));
            if (selectedNode?.id === node.id) {
              setSelectedNode(null);
            }
          } else if (node.type === 'document' && node.dbName && node.tableName && node.docId) {
            await kuiperdbClient.deleteDocument(node.dbName, node.tableName, node.docId);
            notifications.show({
              title: 'Document deleted',
              message: `Document "${node.label}" deleted successfully`,
              color: 'green',
            });
            // Remove document from tree
            setTreeData(prev => prev.map(db => ({
              ...db,
              children: db.children?.map(table => {
                if (table.id === `table-${node.dbName}-${node.tableName}`) {
                  return {
                    ...table,
                    children: table.children?.filter(d => d.id !== node.id) || [],
                  };
                }
                return table;
              }) || [],
            })));
            if (selectedNode?.id === node.id) {
              setSelectedNode(null);
            }
          }
        } catch (err) {
          notifications.show({
            title: 'Delete failed',
            message: getErrorMessage(err),
            color: 'red',
          });
        }
      },
    });
  };

  return (
    <AppShell
      header={{ height: 60 }}
      padding="md"
    >
      <AppShell.Header>
        <Container size="xl" h="100%" style={{ display: 'flex', alignItems: 'center' }}>
          <Group justify="space-between" style={{ width: '100%' }}>
            <Title order={2}>KuiperDb Explorer</Title>
            <Group>
              {healthStatus === false && (
                <Alert icon={<IconAlertCircle size={16} />} color="red" variant="light">
                  Server offline
                </Alert>
              )}
              <Button
                leftSection={<IconRefresh size={16} />}
                variant="light"
                onClick={() => {
                  checkHealth();
                  loadDatabases();
                }}
              >
                Refresh
              </Button>
            </Group>
          </Group>
        </Container>
      </AppShell.Header>

      <AppShell.Main>
        <Container size="xl" h="calc(100vh - 100px)">
          <Grid gutter="md" h="100%">
            <Grid.Col span={4} h="100%">
              <TreeView
                data={treeData}
                onSelect={handleSelect}
                onExpand={handleExpand}
                onDelete={handleDelete}
                selectedId={selectedNode?.id}
                loading={loading}
              />
            </Grid.Col>
            <Grid.Col span={8} h="100%">
              <Tabs value={activeTab} onChange={(value) => setActiveTab(value || 'content')} h="100%">
                <Tabs.List>
                  <Tabs.Tab value="records" leftSection={<IconDatabaseSearch size={16} />}>
                    Records
                  </Tabs.Tab>
                  <Tabs.Tab value="content" leftSection={<IconFileText size={16} />}>
                    Content
                  </Tabs.Tab>
                  <Tabs.Tab 
                    value="graph" 
                    leftSection={<IconNetwork size={16} />}
                    disabled={!selectedNode || selectedNode.type !== 'database'}
                  >
                    Graph
                  </Tabs.Tab>
                </Tabs.List>

                <Tabs.Panel value="records" h="calc(100% - 40px)" pt="xs">
                  <RecordManager
                    selectedNode={selectedNode}
                    onRecordCreated={handleRecordCreated}
                    onRecordSelected={handleRecordSelected}
                    onSchemaChanged={handleSchemaChanged}
                  />
                </Tabs.Panel>

                <Tabs.Panel value="content" h="calc(100% - 40px)" pt="xs">
                  <ContentViewer selectedNode={selectedNode} />
                </Tabs.Panel>

                <Tabs.Panel value="graph" h="calc(100% - 40px)" pt="xs">
                  {selectedNode?.type === 'database' && (
                    <GraphView dbName={selectedNode.dbName!} />
                  )}
                </Tabs.Panel>
              </Tabs>
            </Grid.Col>
          </Grid>
        </Container>
      </AppShell.Main>
    </AppShell>
  );
}

function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <AppContent />
    </QueryClientProvider>
  );
}

export default App;
