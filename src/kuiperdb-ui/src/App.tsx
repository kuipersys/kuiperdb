import '@mantine/core/styles.css';
import '@mantine/notifications/styles.css';
import {
  Alert, AppShell, Badge, Button, Container, Grid, Group, NumberInput, Paper,
  Select, Stack, Table, Text, TextInput, Textarea, Title,
} from '@mantine/core';
import { useEffect, useState } from 'react';
import { useKuiperDb } from '@kuiperdb/react';
import type { KuiperRecord, SearchResult, VectorSpace } from '@kuiperdb/client';

function message(error: unknown): string {
  if (error instanceof Error) return error.message;
  return 'Request failed';
}

function parseVector(input: string): number[] {
  const vector = input.split(',').map((value) => Number(value.trim()));
  if (!vector.length || vector.some((value) => !Number.isFinite(value))) {
    throw new Error('Vector must be comma-separated finite numbers');
  }
  return vector;
}

export default function App() {
  const { client } = useKuiperDb();
  const [spaces, setSpaces] = useState<VectorSpace[]>([]);
  const [records, setRecords] = useState<KuiperRecord[]>([]);
  const [results, setResults] = useState<SearchResult[]>([]);
  const [error, setError] = useState<string>();
  const [spaceName, setSpaceName] = useState('semantic');
  const [dimensions, setDimensions] = useState<number>(3);
  const [selectedSpace, setSelectedSpace] = useState<string | null>(null);
  const [recordId, setRecordId] = useState('');
  const [payload, setPayload] = useState('{"text":"prepared by the application"}');
  const [recordVector, setRecordVector] = useState('1, 0, 0');
  const [queryVector, setQueryVector] = useState('1, 0, 0');

  const refresh = async () => {
    try {
      const [nextSpaces, nextRecords] = await Promise.all([
        client.getVectorSpaces(), client.getRecords(),
      ]);
      setSpaces(nextSpaces);
      setRecords(nextRecords);
      setSelectedSpace((current) => current ?? nextSpaces[0]?.name ?? null);
      setError(undefined);
    } catch (cause) { setError(message(cause)); }
  };

  useEffect(() => { void refresh(); }, []);

  const createSpace = async () => {
    try {
      await client.createVectorSpace({
        name: spaceName, dimensions, distance_metric: 'cosine', normalization: 'none',
      });
      setSelectedSpace(spaceName);
      await refresh();
    } catch (cause) { setError(message(cause)); }
  };

  const putRecord = async () => {
    try {
      await client.putRecord({
        id: recordId || undefined,
        payload: payload ? JSON.parse(payload) : undefined,
        vectors: selectedSpace ? [{ space: selectedSpace, values: parseVector(recordVector) }] : [],
      });
      setRecordId('');
      await refresh();
    } catch (cause) { setError(message(cause)); }
  };

  const search = async () => {
    if (!selectedSpace) return;
    try {
      setResults(await client.search({ space: selectedSpace, vector: parseVector(queryVector), limit: 10 }));
      setError(undefined);
    } catch (cause) { setError(message(cause)); }
  };

  const deleteRecord = async (id: string) => {
    if (!window.confirm(`Delete record "${id}"? This also deletes its vectors and relationships.`)) return;
    try {
      await client.deleteRecord(id);
      setResults((current) => current.filter((result) => result.record.id !== id));
      await refresh();
    } catch (cause) { setError(message(cause)); }
  };

  return (
    <AppShell header={{ height: 64 }} padding="md">
      <AppShell.Header>
        <Container size="xl" h="100%"><Group h="100%" justify="space-between">
          <Title order={2}>KuiperDB Explorer</Title>
          <Button variant="light" onClick={() => void refresh()}>Refresh</Button>
        </Group></Container>
      </AppShell.Header>
      <AppShell.Main>
        <Container size="xl">
          <Stack gap="md">
            {error && <Alert color="red" title="Request failed">{error}</Alert>}
            <Grid>
              <Grid.Col span={{ base: 12, md: 4 }}>
                <Paper withBorder p="md"><Stack>
                  <Title order={3}>Vector spaces</Title>
                  <TextInput label="Name" value={spaceName} onChange={(event) => setSpaceName(event.currentTarget.value)} />
                  <NumberInput label="Dimensions" min={1} value={dimensions} onChange={(value) => setDimensions(Number(value))} />
                  <Button onClick={() => void createSpace()}>Create cosine space</Button>
                  <Select label="Active space" data={spaces.map((space) => space.name)} value={selectedSpace} onChange={setSelectedSpace} />
                  {spaces.map((space) => <Group key={space.name} justify="space-between">
                    <Text>{space.name}</Text><Badge>{space.dimensions}d · {space.distance_metric}</Badge>
                  </Group>)}
                </Stack></Paper>
              </Grid.Col>
              <Grid.Col span={{ base: 12, md: 8 }}>
                <Paper withBorder p="md"><Stack>
                  <Title order={3}>Put prepared record</Title>
                  <TextInput label="Stable ID (optional)" value={recordId} onChange={(event) => setRecordId(event.currentTarget.value)} />
                  <Textarea label="JSON payload" autosize minRows={3} value={payload} onChange={(event) => setPayload(event.currentTarget.value)} />
                  <TextInput label="Vector" description="Comma-separated values" value={recordVector} onChange={(event) => setRecordVector(event.currentTarget.value)} />
                  <Button disabled={!selectedSpace} onClick={() => void putRecord()}>Upsert record</Button>
                </Stack></Paper>
              </Grid.Col>
            </Grid>
            <Paper withBorder p="md"><Stack>
              <Title order={3}>Vector search</Title>
              <Group align="end"><TextInput style={{ flex: 1 }} label="Query vector" value={queryVector} onChange={(event) => setQueryVector(event.currentTarget.value)} />
                <Button disabled={!selectedSpace} onClick={() => void search()}>Search</Button></Group>
              {results.map((result) => <Group key={result.record.id} justify="space-between">
                <Text>{result.record.id}</Text><Badge variant="light">distance {result.distance.toFixed(5)}</Badge>
              </Group>)}
            </Stack></Paper>
            <Paper withBorder p="md">
              <Title order={3} mb="sm">Records</Title>
              <Table striped highlightOnHover>
                <Table.Thead><Table.Tr><Table.Th>ID</Table.Th><Table.Th>Payload</Table.Th><Table.Th>Updated</Table.Th><Table.Th>Actions</Table.Th></Table.Tr></Table.Thead>
                <Table.Tbody>{records.map((record) => <Table.Tr key={record.id}>
                  <Table.Td>{record.id}</Table.Td><Table.Td><code>{JSON.stringify(record.payload)}</code></Table.Td><Table.Td>{new Date(record.updated_at).toLocaleString()}</Table.Td>
                  <Table.Td><Button color="red" size="xs" variant="light" onClick={() => void deleteRecord(record.id)}>Delete</Button></Table.Td>
                </Table.Tr>)}</Table.Tbody>
              </Table>
            </Paper>
          </Stack>
        </Container>
      </AppShell.Main>
    </AppShell>
  );
}
