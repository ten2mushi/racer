//! Sensor Network Example
//!
//! Demonstrates full RACER peer-to-peer leaderless consensus:
//! - Custom message types via `SensorReading` struct
//! - Multi-node P2P network with proper peer discovery
//! - SPDE consensus with ECHO/READY phases
//! - Automatic per-node logging of delivered messages
//!
//! # Usage
//! pkill -f "sensor-network"
//! cargo run -p racer-examples --bin sensor-network --features bls -- examples/config/sensor_network.toml
//! cargo run -p racer-examples --bin sensor-network -- examples/config/sensor_network.toml
//!
//! # Output
//!
//! After running, check `logs/{node-id}/delivered.jsonl` for consensus ground truth.
//! Each node should have the same messages delivered (total consistency).
//! Analyze logs with racer/analyze_consensus.py: (python3 analyze_consensus.py logs)

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use racer::config::{
    At2Config, LogConfig, NodeConfig, PeerConfig, PlatoConfig, RacerConfig, SelectionType,
};
use racer::crypto::PublicKey;
use racer::network::PeerInfo;
use racer::node::Node;
use racer_core::Message;
use rand::Rng;
use serde::{Deserialize, Serialize};
use tokio::task::JoinSet;
use tokio::time::sleep;

// ============================================================================
// Custom Message Type
// ============================================================================

/// Sensor reading message - demonstrates custom message types.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SensorReading {
    /// Unix timestamp in milliseconds (used as message ID)
    pub timestamp: u64,
    /// Unique sensor identifier
    pub sensor_id: String,
    /// Type of measurement (temperature, humidity, pressure)
    pub sensor_type: String,
    /// Measurement value
    pub value: f64,
    /// Unit of measurement
    pub unit: String,
}

impl Message for SensorReading {
    fn id(&self) -> u64 {
        self.timestamp
    }
}

impl SensorReading {
    /// Creates a random sensor reading for demonstration.
    pub fn random(node_id: &str, rng: &mut impl Rng) -> Self {
        let sensor_types = [
            ("temperature", "°C", -40.0, 60.0),
            ("humidity", "%", 0.0, 100.0),
            ("pressure", "hPa", 950.0, 1050.0),
        ];
        let idx = rng.gen_range(0..sensor_types.len());
        let (sensor_type, unit, min, max) = sensor_types[idx];

        Self {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
            sensor_id: format!("{}-sensor-{}", node_id, rng.gen_range(0..10)),
            sensor_type: sensor_type.into(),
            value: rng.gen_range(min..max),
            unit: unit.into(),
        }
    }
}

// ============================================================================
// Configuration Types
// ============================================================================

#[derive(Debug, Deserialize, Clone)]
struct NetworkConfig {
    network: NetworkDef,
    consensus: ConsensusDef,
    plato: PlatoDef,
    scenarios: ScenarioDef,
    #[serde(default)]
    logging: LogConfig,
}

#[derive(Debug, Deserialize, Clone)]
struct NetworkDef {
    name: String,
    base_router_port: u16,
    base_publisher_port: u16,
    #[serde(default)]
    nodes: Vec<NodeDef>,
    node_count: Option<usize>,
}

#[derive(Debug, Deserialize, Clone)]
struct NodeDef {
    id: String,
}

#[derive(Debug, Deserialize, Clone)]
struct ConsensusDef {
    echo_sample_size: usize,
    ready_sample_size: usize,
    delivery_sample_size: usize,
    ready_threshold: usize,
    feedback_threshold: usize,
    delivery_threshold: usize,
}

#[derive(Debug, Deserialize, Clone)]
struct PlatoDef {
    target_latency_secs: f64,
    target_publishing_frequency_secs: f64,
    max_publishing_frequency_secs: f64,
    minimum_latency_secs: f64,
    max_gossip_timeout_secs: f64,
}

#[derive(Debug, Deserialize, Clone)]
struct ScenarioDef {
    messages_per_node: usize,
    message_interval_ms: u64,
}

// ============================================================================
// Node Management
// ============================================================================

/// Holds a node and its cryptographic identity
struct NodeHandle {
    node: Arc<Node<SensorReading>>,
    id: String,
    /// The node's public key for peer registration
    public_key: PublicKey,
}

/// Address info for a peer
struct PeerAddress {
    id: String,
    router: String,
    publisher: String,
}

fn get_peer_addresses(network: &NetworkDef) -> Vec<PeerAddress> {
    let nodes: Vec<NodeDef> = if let Some(count) = network.node_count {
        (0..count)
            .map(|i| NodeDef {
                id: format!("sensor-{}", i),
            })
            .collect()
    } else {
        network.nodes.clone()
    };

    nodes
        .iter()
        .enumerate()
        .map(|(i, node)| PeerAddress {
            id: node.id.clone(),
            router: format!("tcp://127.0.0.1:{}", network.base_router_port + i as u16),
            publisher: format!("tcp://127.0.0.1:{}", network.base_publisher_port + i as u16),
        })
        .collect()
}

fn make_node_config(
    idx: usize,
    node_def: &NodeDef,
    network: &NetworkDef,
    consensus: &ConsensusDef,
    plato: &PlatoDef,
    logging: &LogConfig,
) -> RacerConfig {
    RacerConfig {
        node: NodeConfig {
            id: Some(node_def.id.clone()),
            router_bind: format!("tcp://127.0.0.1:{}", network.base_router_port + idx as u16),
            publisher_bind: format!("tcp://127.0.0.1:{}", network.base_publisher_port + idx as u16),
            selection_type: SelectionType::Random,
        },
        consensus: At2Config {
            echo_sample_size: consensus.echo_sample_size,
            ready_sample_size: consensus.ready_sample_size,
            delivery_sample_size: consensus.delivery_sample_size,
            ready_threshold: consensus.ready_threshold,
            feedback_threshold: consensus.feedback_threshold,
            delivery_threshold: consensus.delivery_threshold,
        },
        plato: PlatoConfig {
            target_latency_secs: plato.target_latency_secs,
            target_publishing_frequency_secs: plato.target_publishing_frequency_secs,
            max_publishing_frequency_secs: plato.max_publishing_frequency_secs,
            minimum_latency_secs: plato.minimum_latency_secs,
            max_gossip_timeout_secs: plato.max_gossip_timeout_secs,
            ..Default::default()
        },
        // Don't use config peers - we'll connect programmatically
        peers: PeerConfig { routers: vec![] },
        logging: logging.clone(),
    }
}

async fn spawn_network(config: &NetworkConfig) -> Result<Vec<NodeHandle>> {
    let nodes: Vec<NodeDef> = if let Some(count) = config.network.node_count {
        (0..count)
            .map(|i| NodeDef {
                id: format!("sensor-{}", i),
            })
            .collect()
    } else {
        config.network.nodes.clone()
    };

    let mut handles = Vec::with_capacity(nodes.len());

    for (idx, node_def) in nodes.iter().enumerate() {
        let node_config = make_node_config(
            idx,
            node_def,
            &config.network,
            &config.consensus,
            &config.plato,
            &config.logging,
        );

        let node = Node::<SensorReading>::new(node_config)
            .await
            .with_context(|| format!("Failed to create node {}", node_def.id))?;

        let public_key = node.public_key();

        handles.push(NodeHandle {
            node: Arc::new(node),
            id: node_def.id.clone(),
            public_key,
        });
    }

    Ok(handles)
}

async fn start_nodes_concurrent(nodes: &[NodeHandle]) -> Result<()> {
    let mut join_set: JoinSet<Result<String>> = JoinSet::new();

    for handle in nodes.iter() {
        let node = handle.node.clone();
        let id = handle.id.clone();
        join_set.spawn(async move {
            node.start().await.map_err(|e| anyhow::anyhow!("{}", e))?;
            Ok(id)
        });
    }

    while let Some(result) = join_set.join_next().await {
        match result {
            Ok(Ok(id)) => println!("    ✓ {} started", id),
            Ok(Err(e)) => return Err(e),
            Err(e) => return Err(anyhow::anyhow!("Join error: {}", e)),
        }
    }

    Ok(())
}

/// Connect all nodes to each other
/// Each node registers all other nodes as peers, subscribing to their publishers.
/// In real world deployment (mesh of iot devices for ex), this would be delegated to a peer discovery method
async fn connect_all_peers(nodes: &[NodeHandle], network: &NetworkDef) -> Result<()> {
    let addresses = get_peer_addresses(network);

    // Build a map of node_id -> public_key from our handles
    let pubkey_map: HashMap<String, PublicKey> = nodes
        .iter()
        .map(|h| (h.id.clone(), h.public_key.clone()))
        .collect();

    println!("  Connecting peers (full mesh topology)...");

    for handle in nodes {
        let mut peer_count = 0;
        for addr in &addresses {
            if addr.id == handle.id {
                continue; // Skip self
            }

            let peer_pubkey = pubkey_map
                .get(&addr.id)
                .ok_or_else(|| anyhow::anyhow!("Missing public key for {}", addr.id))?;

            let peer_info = PeerInfo::new(&addr.id, peer_pubkey.clone(), &addr.router, &addr.publisher);
            
            // add_peer now spawns network operations, so this is non-blocking
            handle.node.add_peer(peer_info).await;
            peer_count += 1;
        }
        println!("    ✓ {} registered {} peers", handle.id, peer_count);
    }

    // Allow background connection tasks to complete
    println!("    Waiting for connections to establish...");
    sleep(Duration::from_secs(1)).await;

    Ok(())
}

async fn stop_nodes(nodes: &[NodeHandle]) {
    for handle in nodes {
        handle.node.stop().await;
    }
}

/// Count lines in a JSONL file
fn count_delivered(path: &std::path::Path) -> usize {
    std::fs::read_to_string(path)
        .map(|s| s.lines().filter(|l| !l.trim().is_empty()).count())
        .unwrap_or(0)
}

// ============================================================================
// Main Scenario
// ============================================================================

async fn run_sensor_scenario(config: &NetworkConfig) -> Result<()> {
    println!("\n╔════════════════════════════════════════════════════════════╗");
    println!("║           RACER P2P Consensus Demonstration                ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");

    // Phase 1: Create nodes
    println!("  Phase 1: Creating {} sensor nodes...", config.network.nodes.len());
    let nodes = spawn_network(config).await?;
    println!("    Created {} nodes with unique keypairs\n", nodes.len());

    // Phase 2: Start nodes (binds sockets)
    println!("  Phase 2: Starting nodes (binding sockets)...");
    start_nodes_concurrent(&nodes).await?;
    println!();

    // Phase 3: Connect all peers (critical for consensus)
    println!("  Phase 3: Establishing P2P mesh network...");
    connect_all_peers(&nodes, &config.network).await?;
    println!("    All nodes now connected in full mesh\n");

    // Phase 4: Submit messages
    println!("  Phase 4: Submitting sensor readings...");
    let total_expected = config.scenarios.messages_per_node * nodes.len();
    let mut submit_count = 0;
    let mut rng = rand::thread_rng();

    for round in 0..config.scenarios.messages_per_node {
        for handle in nodes.iter() {
            let reading = SensorReading::random(&handle.id, &mut rng);

            match handle.node.submit(reading.clone()).await {
                Ok(_batch_id) => {
                    submit_count += 1;
                    if submit_count <= 5 || submit_count % 10 == 0 {
                        println!(
                            "    [{}/{}] {} → {} = {:.1}{}",
                            submit_count,
                            total_expected,
                            handle.id,
                            reading.sensor_type,
                            reading.value,
                            reading.unit
                        );
                    }
                }
                Err(e) => {
                    println!("    ✗ {} failed: {}", handle.id, e);
                }
            }
        }

        if round < config.scenarios.messages_per_node - 1 {
            sleep(Duration::from_millis(config.scenarios.message_interval_ms)).await;
        }
    }

    println!("\n    Submitted {} messages total", submit_count);

    // Phase 5: Wait for consensus
    println!("\n  Phase 5: Waiting for consensus completion...");
    let consensus_timeout = Duration::from_secs(10);
    let start = std::time::Instant::now();

    loop {
        let mut all_complete = true;
        for handle in &nodes {
            let stats = handle.node.gossip_stats().await;
            if stats.active_rounds > 0 {
                all_complete = false;
                break;
            }
        }

        if all_complete {
            println!("    ✓ All consensus rounds complete!");
            break;
        }

        if start.elapsed() > consensus_timeout {
            println!("    ⚠ Timeout waiting for consensus (some rounds may be incomplete)");
            break;
        }

        sleep(Duration::from_millis(500)).await;
    }

    // Phase 6: Report results
    println!("\n  Phase 6: Results");
    println!("  ┌────────────────────────────────────────────────────────┐");
    println!("  │ Node         │ Active │ Latency │ Delivered           │");
    println!("  ├────────────────────────────────────────────────────────┤");

    let mut total_delivered = 0;
    for handle in nodes.iter() {
        let gossip_stats = handle.node.gossip_stats().await;
        let plato_stats = handle.node.plato_stats().await;
        let log_path = config.logging.delivered_path(&handle.id);
        let delivered = count_delivered(&log_path);
        total_delivered += delivered;

        println!(
            "  │ {:12} │ {:6} │ {:7.2}s │ {:8} messages   │",
            handle.id, gossip_stats.active_rounds, plato_stats.current_latency, delivered
        );
    }
    println!("  └────────────────────────────────────────────────────────┘");

    // Verify consensus consistency
    println!("\n  Consensus Verification:");
    println!("    Expected: {} messages × {} nodes = {} total deliveries",
        submit_count, nodes.len(), submit_count * nodes.len());
    println!("    Actual:   {} total deliveries", total_delivered);

    let delivery_rate = if submit_count * nodes.len() > 0 {
        (total_delivered as f64 / (submit_count * nodes.len()) as f64) * 100.0
    } else {
        0.0
    };
    println!("    Rate:     {:.1}%", delivery_rate);

    if delivery_rate >= 80.0 {
        println!("    Status:   ✓ CONSENSUS ACHIEVED");
    } else if delivery_rate >= 50.0 {
        println!("    Status:   ⚠ PARTIAL CONSENSUS (check thresholds)");
    } else {
        println!("    Status:   ✗ CONSENSUS FAILED (check network connectivity)");
    }

    // Log file locations
    println!("\n  Log Files:");
    for handle in &nodes {
        let log_path = config.logging.delivered_path(&handle.id);
        if log_path.exists() {
            let size = std::fs::metadata(&log_path).map(|m| m.len()).unwrap_or(0);
            println!("    {} → {} ({} bytes)", handle.id, log_path.display(), size);
        }
    }

    // Cleanup
    println!("\n  Stopping nodes...");
    stop_nodes(&nodes).await;
    drop(nodes);
    sleep(Duration::from_millis(300)).await;

    println!("\n  ✓ Demonstration complete\n");
    Ok(())
}

// ============================================================================
// Main
// ============================================================================

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("racer=info".parse()?)
                .add_directive("racer::node=debug".parse()?)
                .add_directive("racer_examples=debug".parse()?),
        )
        .init();

    // Parse arguments
    let args: Vec<String> = std::env::args().collect();
    let config_path = args
        .get(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("config/sensor_network.toml"));

    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║        RACER Sensor Network - P2P Consensus Demo           ║");
    println!("║                                                            ║");
    println!("║  Features:                                                 ║");
    println!("║  • Leaderless Byzantine fault-tolerant consensus           ║");
    println!("║  • SPDE protocol with ECHO/READY phases                    ║");
    println!("║  • PLATO congestion control                                ║");
    #[cfg(feature = "bls")]
    println!("║  • BLS Signature Aggregation (Enabled)                     ║");
    #[cfg(not(feature = "bls"))]
    println!("║  • BLS Signature Aggregation (Disabled)                    ║");
    println!("║  • Custom IoT sensor message types                         ║");
    println!("╚════════════════════════════════════════════════════════════╝");
    println!();

    // Load configuration
    let config_str = std::fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read config: {}", config_path.display()))?;
    let config: NetworkConfig =
        toml::from_str(&config_str).with_context(|| "Failed to parse network config")?;

    println!("Configuration:");
    println!("  File:     {}", config_path.display());
    println!("  Network:  {}", config.network.name);
    let node_count = config.network.node_count.unwrap_or(config.network.nodes.len());
    println!("  Nodes:    {}", node_count);
    println!("  Logging:  {} → {}",
        if config.logging.enabled { "enabled" } else { "disabled" },
        config.logging.log_dir);
    println!();

    println!("Consensus Parameters:");
    println!("  Echo sample:      {}", config.consensus.echo_sample_size);
    println!("  Ready sample:     {}", config.consensus.ready_sample_size);
    println!("  Ready threshold:  {} (echoes needed)", config.consensus.ready_threshold);
    println!("  Delivery thresh:  {} (readys needed)", config.consensus.delivery_threshold);

    tokio::select! {
        result = run_sensor_scenario(&config) => {
            result?;
        }
        _ = tokio::signal::ctrl_c() => {
            println!("\n\nInterrupted! Exiting...");
        }
    }

    Ok(())
}
