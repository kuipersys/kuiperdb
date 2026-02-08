import { useEffect, useState, useCallback } from 'react';
import ReactFlow, {
  type Node,
  type Edge,
  Controls,
  Background,
  useNodesState,
  useEdgesState,
  MarkerType,
} from 'reactflow';
import 'reactflow/dist/style.css';
import { Box, Text, Loader, Center } from '@mantine/core';
import { kuiperdbClient } from '../api/client';

interface GraphViewProps {
  dbName: string;
}

interface DocumentRelation {
  id: string;
  source_id: string;
  target_id: string;
  relation_type: string;
  metadata?: Record<string, any>;
  created_at: number;
}

interface Document {
  id: string;
  content: string;
}

export function GraphView({ dbName }: GraphViewProps) {
  const [nodes, setNodes, onNodesChange] = useNodesState([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const loadGraphData = useCallback(async () => {
    if (!dbName) {
      setLoading(false);
      return;
    }

    setLoading(true);
    setError(null);

    try {
      // Get graph statistics to find all nodes
      console.log('GraphView: Loading graph for database:', dbName);
      const stats = await kuiperdbClient.getGraphStats(dbName);
      console.log('GraphView: Graph stats:', stats);
      
      // Get all document IDs from the stats
      const allDocIds = new Set<string>();
      if (stats.in_degrees) {
        Object.keys(stats.in_degrees).forEach(id => allDocIds.add(id));
      }
      if (stats.out_degrees) {
        Object.keys(stats.out_degrees).forEach(id => allDocIds.add(id));
      }

      console.log('GraphView: Found document IDs:', Array.from(allDocIds));

      if (allDocIds.size === 0) {
        console.log('GraphView: No document IDs found');
        setNodes([]);
        setEdges([]);
        setLoading(false);
        return;
      }

      // For each document, get its details and relations
      const docPromises = Array.from(allDocIds).map(async (docId) => {
        // We need to find which table this document belongs to
        const tables = await kuiperdbClient.getTables(dbName);
        
        for (const table of tables) {
          try {
            const docs = await kuiperdbClient.getDocuments(dbName, table.name);
            const doc = docs.find((d: Document) => d.id === docId);
            if (doc) {
              return { id: docId, doc, table: table.name };
            }
          } catch (e) {
            // Table might not exist or have no documents
            continue;
          }
        }
        return { id: docId, doc: null, table: null };
      });

      const docResults = await Promise.all(docPromises);
      console.log('GraphView: Document results:', docResults);

      // Get all relations by traversing from each node
      const allRelations: DocumentRelation[] = [];
      for (const docId of Array.from(allDocIds)) {
        try {
          const relations = await kuiperdbClient.getDocumentRelations(dbName, docId);
          console.log(`GraphView: Relations for ${docId}:`, relations);
          for (const rel of relations) {
            // Only add each relation once (check if already added from other direction)
            if (!allRelations.find(r => r.id === rel.id)) {
              allRelations.push(rel);
            }
          }
        } catch (e) {
          console.error(`Failed to get relations for ${docId}:`, e);
        }
      }

      console.log('GraphView: All relations:', allRelations);

      console.log('GraphView: All relations:', allRelations);

      // Create nodes
      const reactFlowNodes: Node[] = docResults.map((result, index) => {
        const name = result.doc?.content.match(/# (.+)/)?.[1]?.trim() || 'Unknown';
        const angle = (index / allDocIds.size) * 2 * Math.PI;
        const radius = 200;
        
        return {
          id: result.id,
          type: 'default',
          data: { 
            label: name,
          },
          position: { 
            x: 400 + radius * Math.cos(angle), 
            y: 300 + radius * Math.sin(angle) 
          },
          style: {
            background: '#4dabf7',
            color: '#fff',
            border: '2px solid #228be6',
            borderRadius: '8px',
            padding: '10px',
            fontSize: '14px',
            fontWeight: 'bold',
          },
        };
      });

      console.log('GraphView: Created nodes:', reactFlowNodes);

      // Create edges
      const reactFlowEdges: Edge[] = allRelations.map((rel) => ({
        id: rel.id,
        source: rel.source_id,
        target: rel.target_id,
        label: rel.relation_type,
        type: 'default',
        animated: true,
        markerEnd: {
          type: MarkerType.ArrowClosed,
          width: 20,
          height: 20,
          color: '#228be6',
        },
        style: {
          stroke: '#228be6',
          strokeWidth: 2,
        },
        labelStyle: {
          fill: '#228be6',
          fontWeight: 700,
          fontSize: 11,
        },
        labelBgStyle: {
          fill: '#ffffff',
          fillOpacity: 0.9,
        },
        labelBgPadding: [8, 4] as [number, number],
        labelBgBorderRadius: 4,
      }));

      console.log('GraphView: Created edges:', reactFlowEdges);

      setNodes(reactFlowNodes);
      setEdges(reactFlowEdges);
      console.log('GraphView: Set nodes and edges successfully');
    } catch (err) {
      console.error('Failed to load graph data:', err);
      setError(err instanceof Error ? err.message : 'Failed to load graph data');
    } finally {
      setLoading(false);
    }
  }, [dbName, setNodes, setEdges]);

  useEffect(() => {
    loadGraphData();
  }, [loadGraphData]);

  if (loading) {
    return (
      <Center h="100%">
        <Loader size="lg" />
      </Center>
    );
  }

  if (error) {
    return (
      <Center h="100%">
        <Text c="red">{error}</Text>
      </Center>
    );
  }

  if (nodes.length === 0) {
    return (
      <Center h="100%">
        <Text c="dimmed">No document relations found. Create relations using the API.</Text>
      </Center>
    );
  }

  return (
    <Box style={{ width: '100%', height: '600px', minHeight: '600px' }}>
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        fitView
      >
        <Controls />
        <Background />
      </ReactFlow>
    </Box>
  );
}
