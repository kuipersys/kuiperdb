import { useCallback, useEffect, useMemo, useState } from 'react';
import {
  ActionIcon,
  Badge,
  Box,
  Button,
  Code,
  Divider,
  Group,
  Loader,
  NumberInput,
  Paper,
  ScrollArea,
  Select,
  SimpleGrid,
  Stack,
  Switch,
  Text,
  Textarea,
  TextInput,
  Title,
  Tooltip,
} from '@mantine/core';
import { notifications } from '@mantine/notifications';
import {
  IconDatabasePlus,
  IconExternalLink,
  IconFilePlus,
  IconRefresh,
  IconSearch,
  IconTablePlus,
} from '@tabler/icons-react';
import { kuiperdbClient, type Database, type Document, type SearchResult, type Table } from '../api/client';
import type { TreeNode } from '../types';

type SearchType = 'hybrid' | 'fulltext' | 'vector';

interface RecordManagerProps {
  selectedNode: TreeNode | null;
  onRecordCreated: (document: Document) => void;
  onRecordSelected: (dbName: string, tableName: string, docId: string) => void;
  onSchemaChanged: (dbName: string, tableName?: string) => void;
}

function getErrorMessage(error: unknown): string {
  if (typeof error === 'object' && error !== null) {
    const maybeAxios = error as {
      response?: { data?: { message?: string; error?: string } };
      message?: string;
    };

    return (
      maybeAxios.response?.data?.message ||
      maybeAxios.response?.data?.error ||
      maybeAxios.message ||
      'Request failed'
    );
  }

  return 'Request failed';
}

function parseMetadata(value: string): Record<string, unknown> {
  if (!value.trim()) {
    return {};
  }

  const parsed = JSON.parse(value) as unknown;
  if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) {
    throw new Error('Metadata must be a JSON object');
  }

  return parsed as Record<string, unknown>;
}

function parseTags(value: string): string[] {
  return value
    .split(',')
    .map((tag) => tag.trim())
    .filter(Boolean);
}

function formatScore(value: number | null | undefined): string {
  if (typeof value !== 'number') {
    return 'n/a';
  }

  return value.toFixed(4);
}

export function RecordManager({
  selectedNode,
  onRecordCreated,
  onRecordSelected,
  onSchemaChanged,
}: RecordManagerProps) {
  const [databases, setDatabases] = useState<Database[]>([]);
  const [tables, setTables] = useState<Table[]>([]);
  const [dbName, setDbName] = useState('');
  const [tableName, setTableName] = useState('');
  const [newDbName, setNewDbName] = useState('');
  const [newTableName, setNewTableName] = useState('');
  const [schemaLoading, setSchemaLoading] = useState(false);
  const [schemaSaving, setSchemaSaving] = useState(false);

  const [query, setQuery] = useState('');
  const [searchType, setSearchType] = useState<SearchType>('hybrid');
  const [limit, setLimit] = useState<number | string>(10);
  const [searchLoading, setSearchLoading] = useState(false);
  const [searchResults, setSearchResults] = useState<SearchResult[]>([]);

  const [docId, setDocId] = useState('');
  const [content, setContent] = useState('');
  const [tags, setTags] = useState('');
  const [metadata, setMetadata] = useState('{}');
  const [vectorize, setVectorize] = useState(true);
  const [createLoading, setCreateLoading] = useState(false);

  const databaseOptions = useMemo(
    () => databases.map((db) => ({ value: db.name, label: db.name })),
    [databases]
  );

  const tableOptions = useMemo(
    () => tables.map((table) => ({ value: table.name, label: table.name })),
    [tables]
  );

  const hasTarget = Boolean(dbName && tableName);

  const loadDatabases = useCallback(async () => {
    setSchemaLoading(true);
    try {
      const nextDatabases = await kuiperdbClient.getDatabases();
      setDatabases(nextDatabases);
      setDbName((current) => current || nextDatabases[0]?.name || '');
    } catch (error) {
      notifications.show({
        title: 'Could not load databases',
        message: getErrorMessage(error),
        color: 'red',
      });
    } finally {
      setSchemaLoading(false);
    }
  }, []);

  const loadTables = useCallback(async (targetDb: string, preferredTable?: string) => {
    if (!targetDb) {
      setTables([]);
      setTableName('');
      return;
    }

    setSchemaLoading(true);
    try {
      const nextTables = await kuiperdbClient.getTables(targetDb);
      const tableNames = nextTables.map((table) => table.name);

      setTables(nextTables);
      setTableName((current) => {
        if (preferredTable && tableNames.includes(preferredTable)) {
          return preferredTable;
        }

        if (current && tableNames.includes(current)) {
          return current;
        }

        return tableNames[0] || '';
      });
    } catch (error) {
      notifications.show({
        title: 'Could not load tables',
        message: getErrorMessage(error),
        color: 'red',
      });
    } finally {
      setSchemaLoading(false);
    }
  }, []);

  useEffect(() => {
    loadDatabases();
  }, [loadDatabases]);

  useEffect(() => {
    if (selectedNode?.dbName) {
      setDbName(selectedNode.dbName);
    }

    if (selectedNode?.tableName) {
      setTableName(selectedNode.tableName);
    }
  }, [selectedNode]);

  useEffect(() => {
    const preferredTable =
      selectedNode?.dbName === dbName ? selectedNode.tableName : undefined;
    loadTables(dbName, preferredTable);
  }, [dbName, loadTables, selectedNode?.dbName, selectedNode?.tableName]);

  const handleCreateDatabase = async () => {
    const name = newDbName.trim();
    if (!name) {
      return;
    }

    setSchemaSaving(true);
    try {
      await kuiperdbClient.createDatabase(name);
      await loadDatabases();
      setDbName(name);
      setTableName('');
      setNewDbName('');
      onSchemaChanged(name);
      notifications.show({
        title: 'Database ready',
        message: `${name} is available`,
        color: 'green',
      });
    } catch (error) {
      notifications.show({
        title: 'Database creation failed',
        message: getErrorMessage(error),
        color: 'red',
      });
    } finally {
      setSchemaSaving(false);
    }
  };

  const handleCreateTable = async () => {
    const name = newTableName.trim();
    if (!dbName || !name) {
      return;
    }

    setSchemaSaving(true);
    try {
      await kuiperdbClient.createTable(dbName, name);
      await loadDatabases();
      await loadTables(dbName, name);
      setNewTableName('');
      onSchemaChanged(dbName, name);
      notifications.show({
        title: 'Table ready',
        message: `${dbName}.${name} is available`,
        color: 'green',
      });
    } catch (error) {
      notifications.show({
        title: 'Table creation failed',
        message: getErrorMessage(error),
        color: 'red',
      });
    } finally {
      setSchemaSaving(false);
    }
  };

  const handleSearch = async () => {
    if (!hasTarget || !query.trim()) {
      return;
    }

    setSearchLoading(true);
    try {
      const results = await kuiperdbClient.search(dbName, tableName, {
        query: query.trim(),
        type: searchType,
        limit: typeof limit === 'number' ? limit : Number(limit) || 10,
      });
      setSearchResults(results);
    } catch (error) {
      notifications.show({
        title: 'Search failed',
        message: getErrorMessage(error),
        color: 'red',
      });
    } finally {
      setSearchLoading(false);
    }
  };

  const handleCreateRecord = async () => {
    if (!hasTarget || !content.trim()) {
      return;
    }

    setCreateLoading(true);
    try {
      const created = await kuiperdbClient.storeDocument(dbName, tableName, {
        id: docId.trim() || undefined,
        content: content.trim(),
        metadata: parseMetadata(metadata),
        tags: parseTags(tags),
        vectorize,
      });

      setDocId('');
      setContent('');
      onRecordCreated(created);
      notifications.show({
        title: 'Record created',
        message: created.id,
        color: 'green',
      });
    } catch (error) {
      notifications.show({
        title: 'Record creation failed',
        message: getErrorMessage(error),
        color: 'red',
      });
    } finally {
      setCreateLoading(false);
    }
  };

  return (
    <Paper
      shadow="sm"
      p="md"
      withBorder
      style={{ height: '100%', display: 'flex', flexDirection: 'column' }}
    >
      <Group justify="space-between" mb="md">
        <Title order={4}>Records</Title>
        <Tooltip label="Refresh databases and tables">
          <ActionIcon variant="light" onClick={loadDatabases} loading={schemaLoading}>
            <IconRefresh size={16} />
          </ActionIcon>
        </Tooltip>
      </Group>

      <ScrollArea style={{ flex: 1 }}>
        <Stack gap="md">
          <SimpleGrid cols={{ base: 1, md: 2 }} spacing="md">
            <Group align="end" wrap="nowrap">
              <TextInput
                label="New database"
                placeholder="app_data"
                value={newDbName}
                onChange={(event) => setNewDbName(event.currentTarget.value)}
                style={{ flex: 1 }}
              />
              <Button
                leftSection={<IconDatabasePlus size={16} />}
                onClick={handleCreateDatabase}
                loading={schemaSaving}
                disabled={!newDbName.trim()}
              >
                Create
              </Button>
            </Group>

            <Group align="end" wrap="nowrap">
              <TextInput
                label="New table"
                placeholder="documents"
                value={newTableName}
                onChange={(event) => setNewTableName(event.currentTarget.value)}
                disabled={!dbName}
                style={{ flex: 1 }}
              />
              <Button
                leftSection={<IconTablePlus size={16} />}
                onClick={handleCreateTable}
                loading={schemaSaving}
                disabled={!dbName || !newTableName.trim()}
              >
                Create
              </Button>
            </Group>
          </SimpleGrid>

          <SimpleGrid cols={{ base: 1, md: 2 }} spacing="md">
            <Select
              label="Database"
              placeholder="Select database"
              data={databaseOptions}
              value={dbName || null}
              onChange={(value) => setDbName(value || '')}
              searchable
              nothingFoundMessage="No databases"
            />
            <Select
              label="Table"
              placeholder="Select table"
              data={tableOptions}
              value={tableName || null}
              onChange={(value) => setTableName(value || '')}
              searchable
              disabled={!dbName}
              nothingFoundMessage="No tables"
            />
          </SimpleGrid>

          <Divider />

          <SimpleGrid cols={{ base: 1, md: 2 }} spacing="md">
            <Stack gap="sm">
              <Title order={5}>Create Record</Title>
              <TextInput
                label="Custom ID"
                placeholder="Optional"
                value={docId}
                onChange={(event) => setDocId(event.currentTarget.value)}
                disabled={!hasTarget}
              />
              <Textarea
                label="Content"
                placeholder="Record content"
                minRows={7}
                autosize
                value={content}
                onChange={(event) => setContent(event.currentTarget.value)}
                disabled={!hasTarget}
              />
              <TextInput
                label="Tags"
                placeholder="notes, customer, active"
                value={tags}
                onChange={(event) => setTags(event.currentTarget.value)}
                disabled={!hasTarget}
              />
              <Textarea
                label="Metadata"
                minRows={4}
                autosize
                value={metadata}
                onChange={(event) => setMetadata(event.currentTarget.value)}
                disabled={!hasTarget}
              />
              <Group justify="space-between" align="center">
                <Switch
                  label="Vectorize"
                  checked={vectorize}
                  onChange={(event) => setVectorize(event.currentTarget.checked)}
                  disabled={!hasTarget}
                />
                <Button
                  leftSection={<IconFilePlus size={16} />}
                  onClick={handleCreateRecord}
                  loading={createLoading}
                  disabled={!hasTarget || !content.trim()}
                >
                  Save
                </Button>
              </Group>
            </Stack>

            <Stack gap="sm">
              <Title order={5}>Search Records</Title>
              <Group align="end" wrap="nowrap">
                <TextInput
                  label="Query"
                  placeholder="Search content"
                  value={query}
                  onChange={(event) => setQuery(event.currentTarget.value)}
                  onKeyDown={(event) => {
                    if (event.key === 'Enter') {
                      handleSearch();
                    }
                  }}
                  disabled={!hasTarget}
                  style={{ flex: 1 }}
                />
                <Button
                  leftSection={<IconSearch size={16} />}
                  onClick={handleSearch}
                  loading={searchLoading}
                  disabled={!hasTarget || !query.trim()}
                >
                  Search
                </Button>
              </Group>
              <SimpleGrid cols={{ base: 1, sm: 2 }} spacing="sm">
                <Select
                  label="Mode"
                  value={searchType}
                  onChange={(value) => setSearchType((value || 'hybrid') as SearchType)}
                  data={[
                    { value: 'hybrid', label: 'Hybrid' },
                    { value: 'fulltext', label: 'Full text' },
                    { value: 'vector', label: 'Vector' },
                  ]}
                  disabled={!hasTarget}
                />
                <NumberInput
                  label="Limit"
                  min={1}
                  max={100}
                  value={limit}
                  onChange={setLimit}
                  disabled={!hasTarget}
                />
              </SimpleGrid>

              <Box>
                <Group justify="space-between" mb="xs">
                  <Text size="sm" fw={600}>
                    Results
                  </Text>
                  {searchLoading ? (
                    <Loader size="xs" />
                  ) : (
                    <Badge variant="light">{searchResults.length}</Badge>
                  )}
                </Group>

                <Stack gap="xs">
                  {searchResults.length === 0 ? (
                    <Text c="dimmed" size="sm">
                      No records found
                    </Text>
                  ) : (
                    searchResults.map((result) => (
                      <Paper key={result.id} p="sm" withBorder radius="sm">
                        <Stack gap={6}>
                          <Group justify="space-between" align="flex-start" wrap="nowrap">
                            <Box style={{ minWidth: 0 }}>
                              <Text size="sm" fw={600} truncate>
                                {result.id}
                              </Text>
                              <Group gap={6} mt={4}>
                                <Badge size="xs" variant="light">
                                  score {formatScore(result.score)}
                                </Badge>
                                {result.is_chunk && (
                                  <Badge size="xs" color="violet" variant="light">
                                    chunk {result.chunk_index ?? ''}
                                  </Badge>
                                )}
                              </Group>
                            </Box>
                            <Tooltip label="Open record">
                              <ActionIcon
                                variant="subtle"
                                onClick={() => onRecordSelected(dbName, tableName, result.id)}
                              >
                                <IconExternalLink size={16} />
                              </ActionIcon>
                            </Tooltip>
                          </Group>

                          <Text size="sm" lineClamp={4} style={{ whiteSpace: 'pre-wrap' }}>
                            {result.content}
                          </Text>

                          {Object.keys(result.metadata || {}).length > 0 && (
                            <Code block>{JSON.stringify(result.metadata, null, 2)}</Code>
                          )}
                        </Stack>
                      </Paper>
                    ))
                  )}
                </Stack>
              </Box>
            </Stack>
          </SimpleGrid>
        </Stack>
      </ScrollArea>
    </Paper>
  );
}
