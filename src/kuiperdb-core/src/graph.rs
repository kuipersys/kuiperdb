use crate::models::DocumentRelation;
use anyhow::Result;
use petgraph::algo::dijkstra;
use petgraph::graph::{DiGraph, NodeIndex};
use serde::Serialize;
use std::collections::{HashMap, HashSet};

/// Graph traversal result
#[derive(Debug, Clone)]
pub struct TraversalResult {
    pub document_ids: Vec<String>,
    pub relations: Vec<DocumentRelation>,
    pub depth_map: HashMap<String, usize>, // doc_id -> depth from start
}

/// Shortest path result
#[derive(Debug, Clone)]
pub struct ShortestPath {
    pub path: Vec<String>, // Document IDs in order
    pub relations: Vec<DocumentRelation>,
    pub total_weight: usize,
}

/// Graph algorithms for document relationships
#[derive(Default)]
pub struct DocumentGraph {
    // This is a helper struct; actual graph is built on-demand from relations
}

impl DocumentGraph {
    pub fn new() -> Self {
        Self::default()
    }

    /// Build a petgraph from document relations
    fn build_graph(
        &self,
        relations: &[DocumentRelation],
    ) -> (DiGraph<String, String>, HashMap<String, NodeIndex>) {
        let mut graph = DiGraph::new();
        let mut node_map = HashMap::new();

        // Create nodes for all unique document IDs
        let mut doc_ids = HashSet::new();
        for rel in relations {
            doc_ids.insert(rel.source_id.clone());
            doc_ids.insert(rel.target_id.clone());
        }

        for doc_id in doc_ids {
            let idx = graph.add_node(doc_id.clone());
            node_map.insert(doc_id, idx);
        }

        // Add edges
        for rel in relations {
            if let (Some(&source_idx), Some(&target_idx)) =
                (node_map.get(&rel.source_id), node_map.get(&rel.target_id))
            {
                graph.add_edge(source_idx, target_idx, rel.relation_type.clone());
            }
        }

        (graph, node_map)
    }

    /// Breadth-first traversal from a starting document
    pub fn traverse_bfs(
        &self,
        start_id: &str,
        relations: &[DocumentRelation],
        max_depth: usize,
        relation_type_filter: Option<&[String]>,
    ) -> Result<TraversalResult> {
        // Filter relations if needed
        let filtered_relations: Vec<_> = if let Some(types) = relation_type_filter {
            relations
                .iter()
                .filter(|r| types.contains(&r.relation_type))
                .cloned()
                .collect()
        } else {
            relations.to_vec()
        };

        let (graph, node_map) = self.build_graph(&filtered_relations);

        let start_idx = match node_map.get(start_id) {
            Some(&idx) => idx,
            None => {
                return Ok(TraversalResult {
                    document_ids: vec![],
                    relations: vec![],
                    depth_map: HashMap::new(),
                });
            }
        };

        let mut visited = Vec::new();
        let mut depth_map = HashMap::new();
        let mut current_depth = 0;
        let mut nodes_at_depth = vec![start_idx];

        depth_map.insert(start_id.to_string(), 0);
        visited.push(start_id.to_string());

        while current_depth < max_depth && !nodes_at_depth.is_empty() {
            let mut next_depth_nodes = Vec::new();

            for &node_idx in &nodes_at_depth {
                for neighbor in graph.neighbors(node_idx) {
                    let neighbor_id = &graph[neighbor];

                    if !depth_map.contains_key(neighbor_id) {
                        depth_map.insert(neighbor_id.clone(), current_depth + 1);
                        visited.push(neighbor_id.clone());
                        next_depth_nodes.push(neighbor);
                    }
                }
            }

            nodes_at_depth = next_depth_nodes;
            current_depth += 1;
        }

        // Collect relations that are part of the traversal
        let visited_set: HashSet<_> = visited.iter().cloned().collect();
        let traversal_relations: Vec<_> = filtered_relations
            .into_iter()
            .filter(|r| visited_set.contains(&r.source_id) && visited_set.contains(&r.target_id))
            .collect();

        Ok(TraversalResult {
            document_ids: visited,
            relations: traversal_relations,
            depth_map,
        })
    }

    /// Find shortest path between two documents using Dijkstra
    pub fn shortest_path(
        &self,
        from_id: &str,
        to_id: &str,
        relations: &[DocumentRelation],
    ) -> Result<Option<ShortestPath>> {
        let (graph, node_map) = self.build_graph(relations);

        let from_idx = match node_map.get(from_id) {
            Some(&idx) => idx,
            None => return Ok(None),
        };

        let to_idx = match node_map.get(to_id) {
            Some(&idx) => idx,
            None => return Ok(None),
        };

        // Run Dijkstra (all edges have weight 1)
        let distances = dijkstra(&graph, from_idx, Some(to_idx), |_| 1);

        if !distances.contains_key(&to_idx) {
            return Ok(None); // No path exists
        }

        // Reconstruct path
        let mut path = vec![to_id.to_string()];
        let mut current = to_idx;

        while current != from_idx {
            let mut found = false;

            // Find predecessor
            for predecessor in graph.neighbors_directed(current, petgraph::Direction::Incoming) {
                if let Some(&pred_dist) = distances.get(&predecessor) {
                    if let Some(&curr_dist) = distances.get(&current) {
                        if pred_dist + 1 == curr_dist {
                            path.push(graph[predecessor].clone());
                            current = predecessor;
                            found = true;
                            break;
                        }
                    }
                }
            }

            if !found {
                return Ok(None); // Path reconstruction failed
            }
        }

        path.reverse();

        // Collect relations along the path
        let mut path_relations = Vec::new();
        for i in 0..path.len() - 1 {
            for rel in relations {
                if rel.source_id == path[i] && rel.target_id == path[i + 1] {
                    path_relations.push(rel.clone());
                    break;
                }
            }
        }

        Ok(Some(ShortestPath {
            path,
            relations: path_relations,
            total_weight: distances[&to_idx],
        }))
    }

    /// Detect cycles in the graph
    pub fn has_cycles(&self, relations: &[DocumentRelation]) -> bool {
        let (graph, _) = self.build_graph(relations);
        petgraph::algo::is_cyclic_directed(&graph)
    }

    /// Calculate graph statistics
    pub fn statistics(&self, relations: &[DocumentRelation]) -> GraphStatistics {
        let (graph, _) = self.build_graph(relations);

        let node_count = graph.node_count();
        let edge_count = graph.edge_count();

        // Calculate degree distribution
        let mut in_degrees = HashMap::new();
        let mut out_degrees = HashMap::new();

        for node_idx in graph.node_indices() {
            let node_id = &graph[node_idx];
            let in_degree = graph
                .neighbors_directed(node_idx, petgraph::Direction::Incoming)
                .count();
            let out_degree = graph
                .neighbors_directed(node_idx, petgraph::Direction::Outgoing)
                .count();

            in_degrees.insert(node_id.clone(), in_degree);
            out_degrees.insert(node_id.clone(), out_degree);
        }

        GraphStatistics {
            node_count,
            edge_count,
            has_cycles: self.has_cycles(relations),
            in_degrees,
            out_degrees,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct GraphStatistics {
    pub node_count: usize,
    pub edge_count: usize,
    pub has_cycles: bool,
    pub in_degrees: HashMap<String, usize>,
    pub out_degrees: HashMap<String, usize>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_test_relation(source: &str, target: &str, rel_type: &str) -> DocumentRelation {
        DocumentRelation {
            id: uuid::Uuid::new_v4().to_string(),
            source_id: source.to_string(),
            target_id: target.to_string(),
            relation_type: rel_type.to_string(),
            metadata: HashMap::new(),
            created_at: Utc::now(),
        }
    }

    #[test]
    fn test_traverse_bfs_simple() {
        let graph = DocumentGraph::new();

        // Create simple graph: A -> B -> C
        let relations = vec![
            create_test_relation("A", "B", "references"),
            create_test_relation("B", "C", "references"),
        ];

        let result = graph.traverse_bfs("A", &relations, 10, None).unwrap();

        assert_eq!(result.document_ids.len(), 3);
        assert!(result.document_ids.contains(&"A".to_string()));
        assert!(result.document_ids.contains(&"B".to_string()));
        assert!(result.document_ids.contains(&"C".to_string()));
        assert_eq!(result.depth_map.get("A"), Some(&0));
        assert_eq!(result.depth_map.get("B"), Some(&1));
        assert_eq!(result.depth_map.get("C"), Some(&2));
    }

    #[test]
    fn test_traverse_bfs_with_depth_limit() {
        let graph = DocumentGraph::new();

        let relations = vec![
            create_test_relation("A", "B", "references"),
            create_test_relation("B", "C", "references"),
        ];

        let result = graph.traverse_bfs("A", &relations, 1, None).unwrap();

        // Should only reach depth 1 (A and B)
        assert_eq!(result.document_ids.len(), 2);
        assert!(result.document_ids.contains(&"A".to_string()));
        assert!(result.document_ids.contains(&"B".to_string()));
        assert!(!result.document_ids.contains(&"C".to_string()));
    }

    #[test]
    fn test_traverse_bfs_with_filter() {
        let graph = DocumentGraph::new();

        let relations = vec![
            create_test_relation("A", "B", "references"),
            create_test_relation("A", "C", "contradicts"),
        ];

        let filter = vec!["references".to_string()];
        let result = graph
            .traverse_bfs("A", &relations, 10, Some(&filter))
            .unwrap();

        assert_eq!(result.document_ids.len(), 2);
        assert!(result.document_ids.contains(&"A".to_string()));
        assert!(result.document_ids.contains(&"B".to_string()));
        assert!(!result.document_ids.contains(&"C".to_string()));
    }

    #[test]
    fn test_shortest_path_simple() {
        let graph = DocumentGraph::new();

        let relations = vec![
            create_test_relation("A", "B", "references"),
            create_test_relation("B", "C", "references"),
        ];

        let result = graph.shortest_path("A", "C", &relations).unwrap();

        assert!(result.is_some());
        let path = result.unwrap();
        assert_eq!(path.path, vec!["A", "B", "C"]);
        assert_eq!(path.total_weight, 2);
    }

    #[test]
    fn test_shortest_path_no_path() {
        let graph = DocumentGraph::new();

        let relations = vec![create_test_relation("A", "B", "references")];

        let result = graph.shortest_path("A", "C", &relations).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_has_cycles_true() {
        let graph = DocumentGraph::new();

        // Create cycle: A -> B -> C -> A
        let relations = vec![
            create_test_relation("A", "B", "references"),
            create_test_relation("B", "C", "references"),
            create_test_relation("C", "A", "references"),
        ];

        assert!(graph.has_cycles(&relations));
    }

    #[test]
    fn test_has_cycles_false() {
        let graph = DocumentGraph::new();

        // No cycle: A -> B -> C
        let relations = vec![
            create_test_relation("A", "B", "references"),
            create_test_relation("B", "C", "references"),
        ];

        assert!(!graph.has_cycles(&relations));
    }

    #[test]
    fn test_graph_statistics() {
        let graph = DocumentGraph::new();

        let relations = vec![
            create_test_relation("A", "B", "references"),
            create_test_relation("A", "C", "references"),
            create_test_relation("B", "C", "references"),
        ];

        let stats = graph.statistics(&relations);

        assert_eq!(stats.node_count, 3);
        assert_eq!(stats.edge_count, 3);
        assert!(!stats.has_cycles);

        // A has out-degree 2, in-degree 0
        assert_eq!(stats.out_degrees.get("A"), Some(&2));
        assert_eq!(stats.in_degrees.get("A"), Some(&0));

        // C has in-degree 2, out-degree 0
        assert_eq!(stats.in_degrees.get("C"), Some(&2));
        assert_eq!(stats.out_degrees.get("C"), Some(&0));
    }
}
