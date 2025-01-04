use crate::solve::solve_knapsack;
use crate::types::{Commodity, Station, StationMarket};
use crate::LandingPad;
use color_eyre::Result;
use dashmap::DashMap;
use futures::StreamExt;
use indicatif::ProgressBar;
use log::info;
use rand::{rngs::SmallRng, seq::IteratorRandom, SeedableRng};
use rayon::iter::IntoParallelRefIterator;
use rayon::iter::ParallelIterator;
use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres};
use std::sync::Arc;
#[allow(unused_variables)]

/// Gets a list of all stations
async fn get_all_stations(pool: &Pool<Postgres>, landing_pad: LandingPad) -> Result<Vec<Station>> {
    let pad_name = if landing_pad == LandingPad::Small {
        "%s%"
    } else if landing_pad == LandingPad::Medium {
        "%m%"
    } else if landing_pad == LandingPad::Large {
        "%l%"
    } else {
        panic!();
    };

    return Ok(sqlx::query_as!(
        Station,
        r#"SELECT
            id, name, distance_to_arrival, market_id, system_id
        FROM stations
        WHERE
            market_id IS NOT NULL AND system_id IS NOT NULL AND landing_pad LIKE $1
        ;"#,
        pad_name
    )
    .fetch_all(pool)
    .await?);
}

/// Finds commodities for a group of stations
async fn get_all_commodities(
    stations: &Vec<Station>,
    pool: &Pool<Postgres>,
) -> Result<Arc<DashMap<i64, Vec<Commodity>>>> {
    let out: Arc<DashMap<i64, Vec<Commodity>>> = Arc::new(DashMap::new());

    let bar = Arc::new(ProgressBar::new(stations.len().try_into().unwrap()));
    futures::stream::iter(stations.clone().iter())
        .for_each(|station1| {
            let pool = pool.clone();
            let bar = bar.clone();
            let out = out.clone();
            async move {
                bar.inc(1);
                let commodities = station1.get_commodities(&pool).await.unwrap();
                out.insert(station1.id, commodities);
            }
        })
        .await;

    return Ok(out);
}

/// Computes a single hop route
pub async fn compute_single(
    url: String,
    src: Option<String>,
    jump: f32,
    capital: u64,
    capacity: u32,
    sample_factor: f32,
    landing_pad: LandingPad,
) -> Result<()> {
    info!("Setting up PostgreSQL pool on {}", url);
    let var_name = PgPoolOptions::new();
    let pool = var_name.max_connections(32).connect(&url).await?;

    match src {
        Some(source) => Ok(()),
        None => {
            info!("Fetching all stations");
            let stations = get_all_stations(&pool, landing_pad).await?;

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

            info!(
                "Retrieving all commodities for {} sampled stations",
                sample.len()
            );
            let all_commodities = get_all_commodities(&sample, &pool).await?;

            info!(
                "Computing trades for {} stations (approx {} individual routes)",
                sample.len(),
                sample.len().pow(2) - sample.len()
            );
            let bar = ProgressBar::new(sample.len().try_into().unwrap());

            sample.par_iter().for_each(|station1| {
                let commodities1 = all_commodities.get(&station1.id).unwrap();
                for station2 in &sample {
                    // skip self
                    if station2.id == station1.id {
                        continue;
                    }
                    let commodities2 = all_commodities.get(&station2.id).unwrap();

                    let solution = solve_knapsack(
                        StationMarket::new(station1, &commodities1),
                        StationMarket::new(station2, &commodities2),
                        capacity,
                        capital,
                    );
                }
                bar.inc(1);
            });

            bar.finish();

            Ok(())
        }
    }
}
