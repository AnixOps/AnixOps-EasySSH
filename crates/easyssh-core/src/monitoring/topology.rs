//! Server network topology visualization

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Network topology for visualization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerTopology {
    pub nodes: Vec<TopologyNode>,
    pub edges: Vec<TopologyEdge>,
    pub groups: Vec<TopologyGroup>,
    pub layout: TopologyLayout,
}

impl ServerTopology {
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            groups: Vec::new(),
            layout: TopologyLayout::default(),
        }
    }

    pub fn add_node(&mut self, node: TopologyNode) {
        if !self.nodes.iter().any(|n| n.id == node.id) {
            self.nodes.push(node);
        }
    }

    pub fn remove_node(&mut self, node_id: &str) {
        self.nodes.retain(|n| n.id != node_id);
        self.edges
            .retain(|e| e.source != node_id && e.target != node_id);
    }

    pub fn add_edge(&mut self, edge: TopologyEdge) {
        if !self.edges.iter().any(|e| e.id == edge.id) {
            self.edges.push(edge);
        }
    }

    pub fn apply_layout(&mut self, layout: TopologyLayout) {
        // Apply positions from layout
        for (node_id, position) in &layout.node_positions {
            if let Some(node) = self.nodes.iter_mut().find(|n| &n.id == node_id) {
                node.x = position.x;
                node.y = position.y;
            }
        }

        self.layout = layout;
    }

    /// Auto-layout using force-directed algorithm
    pub fn auto_layout(&mut self) {
        let width = 1200.0;
        let height = 800.0;
        let center_x = width / 2.0;
        let center_y = height / 2.0;

        // Group nodes by type
        let mut servers: Vec<&mut TopologyNode> = self
            .nodes
            .iter_mut()
            .filter(|n| n.node_type == TopologyNodeType::Server)
            .collect();

        let count = servers.len() as f64;
        let radius = 300.0;

        // Arrange servers in a circle
        for (i, node) in servers.iter_mut().enumerate() {
            let angle = (i as f64 / count) * 2.0 * std::f64::consts::PI;
            node.x = center_x + radius * angle.cos();
            node.y = center_y + radius * angle.sin();
        }

        // Position load balancers at center
        for node in self
            .nodes
            .iter_mut()
            .filter(|n| n.node_type == TopologyNodeType::LoadBalancer)
        {
            node.x = center_x;
            node.y = center_y - 100.0;
        }

        // Position databases below
        for (i, node) in self
            .nodes
            .iter_mut()
            .filter(|n| n.node_type == TopologyNodeType::Database)
            .enumerate()
        {
            node.x = center_x + (i as f64 - 1.0) * 150.0;
            node.y = center_y + 350.0;
        }
    }

    /// Find paths between nodes
    pub fn find_paths(&self, source: &str, target: &str, max_hops: usize) -> Vec<Vec<String>> {
        let mut paths = Vec::new();
        let mut visited: std::collections::HashMap<String, usize> = HashMap::new();

        fn dfs(
            current: &str,
            target: &str,
            path: Vec<String>,
            edges: &[TopologyEdge],
            visited: &mut std::collections::HashMap<String, usize>,
            max_hops: usize,
            paths: &mut Vec<Vec<String>>,
        ) {
            if path.len() > max_hops {
                return;
            }

            if current == target {
                paths.push(path);
                return;
            }

            // Find connected nodes
            for edge in edges {
                let next = if edge.source == current {
                    &edge.target
                } else if edge.target == current {
                    &edge.source
                } else {
                    continue;
                };

                if !path.contains(next) {
                    let count = visited.get(next).copied().unwrap_or(0);
                    if count < 2 {
                        // Allow revisiting nodes up to 2 times for redundancy paths
                        visited.insert(next.clone(), count + 1);
                        let mut new_path = path.clone();
                        new_path.push(next.clone());
                        dfs(next, target, new_path, edges, visited, max_hops, paths);
                        visited.insert(next.clone(), count);
                    }
                }
            }
        }

        let initial_path = vec![source.to_string()];
        visited.insert(source.to_string(), 1);
        dfs(
            source,
            target,
            initial_path,
            &self.edges,
            &mut visited,
            max_hops,
            &mut paths,
        );

        paths
    }

    /// Calculate network metrics
    pub fn calculate_metrics(&self) -> TopologyMetrics {
        let total_nodes = self.nodes.len();
        let total_edges = self.edges.len();

        // Calculate average degree
        let avg_degree = if total_nodes > 0 {
            (total_edges * 2) as f64 / total_nodes as f64
        } else {
            0.0
        };

        // Find connected components
        let mut visited = std::collections::HashSet::new();
        let mut components = 0;

        for node in &self.nodes {
            if !visited.contains(&node.id) {
                components += 1;
                self.dfs_component(&node.id, &mut visited);
            }
        }

        // Calculate density
        let max_edges = total_nodes * (total_nodes - 1) / 2;
        let density = if max_edges > 0 {
            total_edges as f64 / max_edges as f64
        } else {
            0.0
        };

        TopologyMetrics {
            total_nodes,
            total_edges,
            avg_degree,
            connected_components: components,
            density,
            is_fully_connected: components == 1,
        }
    }

    fn dfs_component(&self, node_id: &str, visited: &mut std::collections::HashSet<String>) {
        if visited.contains(node_id) {
            return;
        }
        visited.insert(node_id.to_string());

        for edge in &self.edges {
            if edge.source == node_id {
                self.dfs_component(&edge.target, visited);
            } else if edge.target == node_id {
                self.dfs_component(&edge.source, visited);
            }
        }
    }
}

/// Topology node types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TopologyNodeType {
    Server,
    LoadBalancer,
    Database,
    Cache,
    Queue,
    Gateway,
    Firewall,
    Switch,
    Router,
    Cloud,
    Cluster,
    Custom,
}

/// Node status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TopologyStatus {
    Online,
    Offline,
    Degraded,
    Unknown,
    Maintenance,
}

/// Topology node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyNode {
    pub id: String,
    pub node_type: TopologyNodeType,
    pub label: String,
    pub status: TopologyStatus,
    pub metrics: HashMap<String, f64>,
    pub x: f64,
    pub y: f64,
    pub group_id: Option<String>,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Topology edge (connection)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub edge_type: TopologyEdgeType,
    pub label: Option<String>,
    pub bandwidth_mbps: Option<f64>,
    pub latency_ms: Option<f64>,
    pub packet_loss_percent: Option<f64>,
    pub status: TopologyStatus,
    pub thickness: f64,
    pub color: Option<String>,
    pub dashed: bool,
    pub directed: bool,
}

/// Edge types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TopologyEdgeType {
    Network,
    Dependency,
    DataFlow,
    ServiceMesh,
    Physical,
    Virtual,
    Custom,
}

/// Topology group (for clustering)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyGroup {
    pub id: String,
    pub name: String,
    pub color: String,
    pub node_ids: Vec<String>,
    pub collapsed: bool,
}

/// Topology layout configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyLayout {
    pub algorithm: LayoutAlgorithm,
    pub node_positions: HashMap<String, Position>,
    pub zoom_level: f64,
    pub pan_x: f64,
    pub pan_y: f64,
    pub show_labels: bool,
    pub show_metrics: bool,
    pub animated: bool,
}

impl Default for TopologyLayout {
    fn default() -> Self {
        Self {
            algorithm: LayoutAlgorithm::ForceDirected,
            node_positions: HashMap::new(),
            zoom_level: 1.0,
            pan_x: 0.0,
            pan_y: 0.0,
            show_labels: true,
            show_metrics: true,
            animated: true,
        }
    }
}

/// Layout algorithms
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LayoutAlgorithm {
    ForceDirected,
    Hierarchical,
    Circular,
    Grid,
    Concentric,
    Dagre,
    Spring,
    Custom,
}

/// 2D Position
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

/// Topology metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyMetrics {
    pub total_nodes: usize,
    pub total_edges: usize,
    pub avg_degree: f64,
    pub connected_components: usize,
    pub density: f64,
    pub is_fully_connected: bool,
}

/// Traffic flow between nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrafficFlow {
    pub source: String,
    pub target: String,
    pub rx_bytes_per_sec: f64,
    pub tx_bytes_per_sec: f64,
    pub connections: u32,
    pub timestamp: u64,
}

/// Network path analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkPath {
    pub source: String,
    pub target: String,
    pub hops: Vec<PathHop>,
    pub total_latency_ms: f64,
    pub total_bandwidth_mbps: f64,
    pub is_healthy: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathHop {
    pub node_id: String,
    pub ingress_edge: Option<String>,
    pub egress_edge: Option<String>,
    pub latency_ms: f64,
}

/// Topology discovery result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyDiscoveryResult {
    pub discovered_nodes: Vec<TopologyNode>,
    pub discovered_edges: Vec<TopologyEdge>,
    pub scan_duration_ms: u64,
    pub timestamp: u64,
}

/// Network scan configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkScanConfig {
    pub target_servers: Vec<String>,
    pub scan_ports: Vec<u16>,
    pub probe_timeout_ms: u64,
    pub max_hops: u32,
    pub discover_services: bool,
    pub trace_routes: bool,
}

/// Service discovery result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDiscoveryResult {
    pub server_id: String,
    pub services: Vec<DiscoveredService>,
    pub open_ports: Vec<u16>,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredService {
    pub name: String,
    pub port: u16,
    pub protocol: String,
    pub version: Option<String>,
    pub metadata: HashMap<String, String>,
}

/// Topology builder for creating topologies programmatically
pub struct TopologyBuilder {
    topology: ServerTopology,
}

impl TopologyBuilder {
    pub fn new() -> Self {
        Self {
            topology: ServerTopology::new(),
        }
    }

    pub fn with_server(mut self, id: &str, name: &str, status: TopologyStatus) -> Self {
        self.topology.add_node(TopologyNode {
            id: id.to_string(),
            node_type: TopologyNodeType::Server,
            label: name.to_string(),
            status,
            metrics: HashMap::new(),
            x: 0.0,
            y: 0.0,
            group_id: None,
            icon: Some("server".to_string()),
            color: None,
            metadata: HashMap::new(),
        });
        self
    }

    pub fn with_database(mut self, id: &str, name: &str, status: TopologyStatus) -> Self {
        self.topology.add_node(TopologyNode {
            id: id.to_string(),
            node_type: TopologyNodeType::Database,
            label: name.to_string(),
            status,
            metrics: HashMap::new(),
            x: 0.0,
            y: 0.0,
            group_id: None,
            icon: Some("database".to_string()),
            color: Some("#3b82f6".to_string()),
            metadata: HashMap::new(),
        });
        self
    }

    pub fn with_load_balancer(mut self, id: &str, name: &str) -> Self {
        self.topology.add_node(TopologyNode {
            id: id.to_string(),
            node_type: TopologyNodeType::LoadBalancer,
            label: name.to_string(),
            status: TopologyStatus::Online,
            metrics: HashMap::new(),
            x: 0.0,
            y: 0.0,
            group_id: None,
            icon: Some("load-balancer".to_string()),
            color: Some("#22c55e".to_string()),
            metadata: HashMap::new(),
        });
        self
    }

    pub fn with_connection(mut self, from: &str, to: &str) -> Self {
        let edge_id = format!("{}-{}", from, to);
        self.topology.add_edge(TopologyEdge {
            id: edge_id,
            source: from.to_string(),
            target: to.to_string(),
            edge_type: TopologyEdgeType::Network,
            label: None,
            bandwidth_mbps: None,
            latency_ms: None,
            packet_loss_percent: None,
            status: TopologyStatus::Online,
            thickness: 2.0,
            color: None,
            dashed: false,
            directed: false,
        });
        self
    }

    pub fn build(mut self) -> ServerTopology {
        self.topology.auto_layout();
        self.topology
    }
}

impl Default for ServerTopology {
    fn default() -> Self {
        Self::new()
    }
}
