//! `racer run` subcommand implementation.
//!
//! Starts a RACER node with configuration layering:
//! 1. TOML config file (base)
//! 2. Environment variables (override)
//! 3. CLI arguments (highest priority)

use std::path::PathBuf;
use std::sync::Arc;

use clap::Parser;
use tokio::signal;

use crate::config::RacerConfig;
use crate::node::Node;
use racer_core::message::DefaultMessage;

use super::logging;

#[derive(Parser, Debug)]
pub struct Args {
    #[arg(short, long, default_value = "racer.toml")]
    pub config: PathBuf,

    #[arg(long, env = "RACER_NODE_ID")]
    pub node_id: Option<String>,

    #[arg(long, env = "RACER_ROUTER_BIND")]
    pub router_bind: Option<String>,

    #[arg(long, env = "RACER_PUBLISHER_BIND")]
    pub publisher_bind: Option<String>,

    #[arg(long, env = "RACER_PEERS", value_delimiter = ',')]
    pub peers: Option<Vec<String>>,

    #[arg(long, default_value = "./logs")]
    pub log_dir: PathBuf,

    #[arg(long, default_value = "100")]
    pub log_max_size_mb: u64,

    #[arg(long, default_value = "10")]
    pub log_max_files: usize,
}

pub async fn execute(args: Args) -> anyhow::Result<()> {
    let mut config = if args.config.exists() {
        RacerConfig::from_file(&args.config)?
    } else {
        tracing::warn!(
            path = %args.config.display(),
            "Config file not found, using defaults"
        );
        RacerConfig::default()
    };

    if let Some(id) = args.node_id {
        config.node.id = Some(id);
    }
    if let Some(router) = args.router_bind {
        config.node.router_bind = router;
    }
    if let Some(publisher) = args.publisher_bind {
        config.node.publisher_bind = publisher;
    }
    if let Some(peers) = args.peers {
        config.peers.routers = peers;
    }

    config.validate()?;

    let log_config = logging::LoggingConfig {
        log_dir: args.log_dir,
        max_size_mb: args.log_max_size_mb,
        max_files: args.log_max_files,
    };
    logging::init_logging(log_config)?;

    tracing::info!(
        node_id = ?config.node.id,
        router = %config.node.router_bind,
        publisher = %config.node.publisher_bind,
        peers = ?config.peers.routers,
        "Starting RACER node"
    );

    let node = Arc::new(Node::<DefaultMessage>::new(config).await?);
    node.start().await?;

    tracing::info!(id = %node.id(), "Node started, waiting for shutdown signal");

    signal::ctrl_c().await?;

    tracing::info!("Shutdown signal received");
    node.stop().await;

    Ok(())
}
