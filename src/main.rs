use core::f32;

use clap::{Parser, Subcommand};
use color_eyre::eyre::Result;
use compute::compute_single;
use log::info;

pub mod compute;
pub mod router;
pub mod types;
pub mod solve;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Parser)] // requires `derive` feature
#[command(name = "kural")]
#[command(
    about = format!("Kural v{VERSION}: High-performance Elite: Dangerous trade route calculator"),
)]
struct KuralCli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Computes an optimal single-hop trade route.
    ///
    /// A single-hop trade route only considers A->B for some A, B in the galaxy. It does not
    /// consider round trips like A->B->A, or multi-hop routes like A->B->C->etc. It can, however,
    /// be optionally tuned to generate valid routes using your ship's jump distance.
    ComputeSingle {
        #[arg(long)]
        /// EDTear Postgres connection URL. Recommended: postgres://postgres:password@localhost/edtear
        url: String,

        #[arg(long)]
        /// Worst-case (fully laden) jump range in light years
        jump: f32,

        #[arg(long)]
        /// Initial capital funds
        capital: u64,

        #[arg(long)]
        /// Ship cargo capacity
        capacity: u32,

        #[arg(long)]
        /// Starting system name. If not specified, the entire galaxy is considered.
        src: Option<String>,

        #[arg(long)]
        /// Max jumps to get from A to B. If unspecified, hops are not considered and your ship is
        /// assumed to be able to commute from any system to any other system in The Bubble.
        max_jumps: Option<u32>,

        #[arg(long)]
        #[clap(default_value = "0.5")]
        /// For each station, this is the percent between 0.0 and 1.0 of other stations in the
        /// galaxy to randomly sample
        random_sample: f32,
    },

    /// Prints version information.
    #[command()]
    Version {},
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = KuralCli::parse();
    env_logger::init();
    color_eyre::install()?;

    match args.command {
        Commands::Version {} => {
            println!("Kural v{VERSION}: High-performance Elite: Dangerous trade route calculator");
            println!("Copyright (c) 2024 Matt Young. ISC Licence.");
            Ok(())
        }

        Commands::ComputeSingle {
            url,
            src,
            jump,
            capital,
            max_jumps,
            capacity,
            random_sample,
        } => {
            info!(
                "Computing single hop trade route {} with jump dist: {}, initial capital: {}, max jumps: {}, ship capacity: {}",
                if let Some(ref x) = src {
                    format!("from {}", x).to_string()
                } else {
                    "across the whole galaxy".to_string()
                },
                jump,
                capital,
                if let Some(x) = max_jumps {
                    x.to_string()
                } else {
                    "unspecified".to_string()
                },
                capacity
            );

            compute_single(url, src.clone(), jump, capital, capacity, random_sample).await?;

            Ok(())
        }
    }
}
