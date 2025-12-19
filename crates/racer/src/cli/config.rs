use std::path::PathBuf;

use clap::Parser;

use crate::config::RacerConfig;

#[derive(Parser, Debug)]
pub struct Args {
    #[arg(short, long)]
    pub config: PathBuf,

    #[arg(long)]
    pub dump: bool,

    #[arg(long)]
    pub dump_toml: bool,
}

pub fn execute(args: Args) -> anyhow::Result<()> {
    let config = RacerConfig::from_file(&args.config)?;
    config.validate()?;

    if args.dump {
        println!("{}", serde_json::to_string_pretty(&config)?);
    } else if args.dump_toml {
        println!("{}", toml::to_string_pretty(&config)?);
    } else {
        println!("✓ Configuration valid: {}", args.config.display());
        println!();
        println!("Node:");
        println!("  ID: {}", config.node.id.as_deref().unwrap_or("<auto>"));
        println!("  Router: {}", config.node.router_bind);
        println!("  Publisher: {}", config.node.publisher_bind);
        println!("  Selection: {:?}", config.node.selection_type);
        println!();
        println!("Consensus:");
        println!("  Echo sample: {}", config.consensus.echo_sample_size);
        println!("  Ready sample: {}", config.consensus.ready_sample_size);
        println!("  Ready threshold: {}", config.consensus.ready_threshold);
        println!("  Delivery threshold: {}", config.consensus.delivery_threshold);
        println!();
        println!("PLATO:");
        println!("  Target latency: {}s", config.plato.target_latency_secs);
        println!();
        println!("Peers: {}", config.peers.routers.len());
        for peer in &config.peers.routers {
            println!("  - {}", peer);
        }
    }

    Ok(())
}
