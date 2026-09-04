use crate::Relation;
use petgraph::algo::dijkstra;
use petgraph::graph::{DiGraph, NodeIndex};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TraversalResult {
    pub record_ids: Vec<String>,
    pub relations: Vec<Relation>,
    pub depth_by_record: HashMap<String, usize>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShortestPath {
    pub record_ids: Vec<String>,
    pub relations: Vec<Relation>,
    pub length: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphStatistics {
    pub record_count: usize,
    pub relation_count: usize,
    pub has_cycles: bool,
    pub incoming: HashMap<String, usize>,
    pub outgoing: HashMap<String, usize>,
}

/// Stateless algorithms over generic record relationships.
#[derive(Debug, Default)]
pub struct Graph;

impl Graph {
    pub fn new() -> Self {
        Self
    }

    fn build(relations: &[Relation]) -> (DiGraph<String, usize>, HashMap<String, NodeIndex>) {
        let mut graph = DiGraph::new();
        let mut nodes = HashMap::new();
        for relation in relations {
            for id in [&relation.source_id, &relation.target_id] {
                if !nodes.contains_key(id) {
                    nodes.insert(id.clone(), graph.add_node(id.clone()));
                }
            }
        }
        for (position, relation) in relations.iter().enumerate() {
            graph.add_edge(
                nodes[&relation.source_id],
                nodes[&relation.target_id],
                position,
            );
        }
        (graph, nodes)
    }

    pub fn traverse(
        &self,
        start_id: &str,
        relations: &[Relation],
        max_depth: usize,
        kinds: Option<&[String]>,
    ) -> TraversalResult {
        let allowed =
            |relation: &&Relation| kinds.is_none_or(|kinds| kinds.contains(&relation.kind));
        let filtered: Vec<Relation> = relations.iter().filter(allowed).cloned().collect();
        let mut queue = VecDeque::from([(start_id.to_owned(), 0usize)]);
        let mut depth_by_record = HashMap::from([(start_id.to_owned(), 0usize)]);
        while let Some((current, depth)) = queue.pop_front() {
            if depth == max_depth {
                continue;
            }
            for relation in filtered
                .iter()
                .filter(|relation| relation.source_id == current)
            {
                if !depth_by_record.contains_key(&relation.target_id) {
                    depth_by_record.insert(relation.target_id.clone(), depth + 1);
                    queue.push_back((relation.target_id.clone(), depth + 1));
                }
            }
        }
        let record_ids: Vec<_> = depth_by_record.keys().cloned().collect();
        let visited: HashSet<_> = record_ids.iter().cloned().collect();
        let relations = filtered
            .into_iter()
            .filter(|relation| {
                visited.contains(&relation.source_id) && visited.contains(&relation.target_id)
            })
            .collect();
        TraversalResult {
            record_ids,
            relations,
            depth_by_record,
        }
    }

    pub fn shortest_path(
        &self,
        from: &str,
        to: &str,
        relations: &[Relation],
    ) -> Option<ShortestPath> {
        let (graph, nodes) = Self::build(relations);
        let (&from_node, &to_node) = (nodes.get(from)?, nodes.get(to)?);
        let distances = dijkstra(&graph, from_node, Some(to_node), |_| 1usize);
        let length = *distances.get(&to_node)?;
        let mut current = to_node;
        let mut record_ids = vec![to.to_owned()];
        let mut path_relations = Vec::new();
        while current != from_node {
            let current_distance = distances[&current];
            let predecessor = graph
                .neighbors_directed(current, petgraph::Direction::Incoming)
                .find(|node| {
                    distances
                        .get(node)
                        .is_some_and(|distance| *distance + 1 == current_distance)
                })?;
            let edge = graph.find_edge(predecessor, current)?;
            path_relations.push(relations[graph[edge]].clone());
            record_ids.push(graph[predecessor].clone());
            current = predecessor;
        }
        record_ids.reverse();
        path_relations.reverse();
        Some(ShortestPath {
            record_ids,
            relations: path_relations,
            length,
        })
    }

    pub fn statistics(&self, relations: &[Relation]) -> GraphStatistics {
        let (graph, _) = Self::build(relations);
        let mut incoming = HashMap::new();
        let mut outgoing = HashMap::new();
        for node in graph.node_indices() {
            incoming.insert(
                graph[node].clone(),
                graph
                    .neighbors_directed(node, petgraph::Direction::Incoming)
                    .count(),
            );
            outgoing.insert(
                graph[node].clone(),
                graph
                    .neighbors_directed(node, petgraph::Direction::Outgoing)
                    .count(),
            );
        }
        GraphStatistics {
            record_count: graph.node_count(),
            relation_count: graph.edge_count(),
            has_cycles: petgraph::algo::is_cyclic_directed(&graph),
            incoming,
            outgoing,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn relation(id: &str, source: &str, target: &str, kind: &str) -> Relation {
        Relation {
            id: id.into(),
            source_id: source.into(),
            target_id: target.into(),
            kind: kind.into(),
            metadata: Default::default(),
            created_at: Utc::now(),
        }
    }

    #[test]
    fn relationships_are_generic_and_traversable() {
        let relations = vec![
            relation("1", "a", "b", "any-kind"),
            relation("2", "b", "c", "any-kind"),
        ];
        let graph = Graph::new();
        let traversal = graph.traverse("a", &relations, 2, None);
        assert_eq!(traversal.depth_by_record.get("c"), Some(&2));
        let path = graph.shortest_path("a", "c", &relations).unwrap();
        assert_eq!(path.record_ids, ["a", "b", "c"]);
        assert_eq!(graph.statistics(&relations).relation_count, 2);
    }
}
