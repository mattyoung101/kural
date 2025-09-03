use core::f32;
use std::process::exit;

use ansi_term::Style;
use clap::{Parser, Subcommand};
use color_eyre::eyre::Result;
use compute::{compute_single, find_cheapest};
use env_logger::{Builder, Env};
use log::{error, info};

pub mod compute;
pub mod router;
pub mod solve;
pub mod types;

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

#[derive(Debug, Clone, Copy, clap::ValueEnum, PartialEq, Eq)]
pub enum LandingPad {
    Small,
    Medium,
    Large,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Computes an optimal single-hop trade route.
    ///
    /// A single-hop trade route only considers A->B for any A, B in the galaxy. It does not
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

        #[arg(long)]
        /// Landing pad size
        landing_pad: LandingPad,
    },

    /// Finds the cheapest commodities. Does not consider player carriers in the search.
    FindCheapest {
        #[arg(long)]
        /// EDTear Postgres connection URL. Recommended: postgres://postgres:password@localhost/edtear
        url: String,

        #[arg(long)]
        /// Landing pad size
        landing_pad: LandingPad,

        #[arg(long)]
        /// Name of the commodity to search for, e.g. "steel"
        name: String,

        #[arg(long)]
        /// Max age of commodities to consider in days
        max_age: u32,

        #[arg(long)]
        /// Minimum available quantity
        min_quantity: u32,
    },

    /// Prints version information.
    #[command()]
    Version {},
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = KuralCli::parse();
    let env = Env::new().filter_or("RUST_LOG", "info");
    Builder::from_env(env).init();
    color_eyre::install()?;

    match args.command {
        Commands::Version {} => {
            println!(
                "Kural v{VERSION}: High-performance {} trade route calculator",
                Style::new().italic().paint("Elite: Dangerous")
            );
            println!("Copyright (c) 2024-2025 Matt Young. ISC Licence.");
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
            landing_pad,
        } => {
            if random_sample <= 0.0 || random_sample > 1.0 {
                error!("Illegal random_sample value");
                exit(1);
            }

            compute_single(
                url,
                src.clone(),
                jump,
                capital,
                capacity,
                random_sample,
                landing_pad,
            )
            .await?;

            Ok(())
        }

        Commands::FindCheapest {
            url,
            landing_pad,
            name,
            max_age,
            min_quantity,
        } => find_cheapest(url, landing_pad, name, max_age, min_quantity).await,
    }
}
