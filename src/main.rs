mod process;

use process::{ManaboxConfig, init_mana};
use clap::{Parser, Subcommand};
use anyhow::Result;

#[derive(Parser)]
#[command(name = "mn")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    Init {
        #[arg(short, long)]
        name: Option<String>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::Init { name }) => {
            init_mana(name)?;
        }
        None => {
            println!("Hello mana!");
            // If .manabox is present, execute the scan.
            if let Ok(config) = ManaboxConfig::load() {
                let snapshot = process::scan_workspace(&config)?;
                println!("✅ Snapshot created with {} files.", snapshot.files.len());
                if let Some((path, hash)) = snapshot.files.iter().next() {
                    println!("Example: {} -> {}", path, &hash[..8]);
                }
            } else {
                println!("💡 Tip: Use 'mn init' to create a .manabox file.");
            }
        }
    }

    Ok(())
}