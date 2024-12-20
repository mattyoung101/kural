#[allow(unused_variables)]
use std::sync::Arc;

use crate::solve::solve_knapsack;
use crate::types::{Station, StationMarket};
use color_eyre::Result;
use futures::StreamExt;
use indicatif::{ProgressBar, ProgressIterator};
use itertools::iproduct;
use log::{debug, info};
use rand::{rngs::SmallRng, seq::IteratorRandom, SeedableRng};
use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres};
use tokio::task;

async fn get_all_stations(pool: &Pool<Postgres>) -> Result<Vec<Station>> {
    return Ok(sqlx::query_as!(
        Station,
        r#"SELECT
            id, name, distance_to_arrival, market_id, system_id
        FROM stations
        WHERE
            market_id IS NOT NULL and system_id IS NOT NULL
        ;"#
    )
    .fetch_all(pool)
    .await?);
}

/// Computes a single hop route
pub async fn compute_single(
    url: String,
    src: Option<String>,
    jump: f32,
    capital: u64,
    capacity: u32,
    sample_factor: f32,
) -> Result<()> {
    info!("Setting up PostgreSQL pool on {}", url);
    let var_name = PgPoolOptions::new();
    let pool = var_name.max_connections(32).connect(&url).await?;

    match src {
        Some(source) => Ok(()),
        None => {
            info!("Fetching all stations");
            let stations = get_all_stations(&pool).await?;

            // the galaxy is very large, so randomly sample a number of stations
            let sample_size: usize = (sample_factor * (stations.len() as f32)) as usize;
            info!(
                "Computing random sample, factor: {} ({} stations)",
                sample_factor, sample_size
            );
            // use SmallRng for speed
            let mut rng = SmallRng::from_entropy();
            // ensure that we are only selecting stations that have a market and system attached to
            // them
            let filtered_stations: Vec<Station> = stations
                .into_iter()
                .filter(|station| station.market_id.is_some() && station.system_id.is_some())
                .collect();

            // now we can compute the random subsample
            let sample = filtered_stations
                .into_iter()
                .choose_multiple(&mut rng, sample_size);

            info!("Processing sampled stations");

            for station1 in sample.clone().into_iter().progress() {
                let commodities1 = station1.get_commodities(&pool);

                // now consider this station against every other station in the sample
                for station2 in sample.clone() {
                    // skip self
                    if station2.id == station1.id {
                        continue
                    }
                }
            }

            Ok(())
        }
    }
}
