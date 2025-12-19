use std::fs;
use std::path::PathBuf;

use clap::{Parser, ValueEnum};

use crate::crypto::KeyPair;

#[derive(Debug, Clone, Copy, ValueEnum, Default)]
pub enum KeyFormat {
    #[default]
    Hex,
    Base64,
}

#[derive(Parser, Debug)]
pub struct Args {
    #[arg(short, long, default_value = "racer.key")]
    pub output: PathBuf,

    #[arg(long)]
    pub pub_out: Option<PathBuf>,

    #[arg(long, value_enum, default_value = "hex")]
    pub format: KeyFormat,

    #[arg(long)]
    pub force: bool,
}

pub fn execute(args: Args) -> anyhow::Result<()> {
    if !args.force {
        if args.output.exists() {
            anyhow::bail!(
                "Output file already exists: {}. Use --force to overwrite.",
                args.output.display()
            );
        }
        if let Some(ref pub_path) = args.pub_out {
            if pub_path.exists() {
                anyhow::bail!(
                    "Public key file already exists: {}. Use --force to overwrite.",
                    pub_path.display()
                );
            }
        }
    }

    let keypair = KeyPair::generate();

    let private_bytes = keypair.signing_key().to_bytes();
    let private_str = match args.format {
        KeyFormat::Hex => hex::encode(&private_bytes),
        KeyFormat::Base64 => base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            &private_bytes,
        ),
    };
    fs::write(&args.output, &private_str)?;
    println!("✓ Private key written to: {}", args.output.display());

    if let Some(pub_path) = args.pub_out {
        let public_hex = keypair.public_key().to_hex();
        let public_str = match args.format {
            KeyFormat::Hex => public_hex,
            KeyFormat::Base64 => {
                let bytes = hex::decode(&public_hex)?;
                base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &bytes)
            }
        };
        fs::write(&pub_path, &public_str)?;
        println!("✓ Public key written to: {}", pub_path.display());
    }

    println!();
    println!("Public key (hex): {}", keypair.public_key().to_hex());

    Ok(())
}
