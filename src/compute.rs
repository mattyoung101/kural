use crate::solve::solve_knapsack;
use crate::types::{Commodity, Station, StationMarket, TradeSolution};
use crate::LandingPad;
use chrono::{NaiveDate, NaiveDateTime, TimeDelta};
use color_eyre::Result;
use dashmap::DashMap;
use futures::StreamExt;
use indicatif::{ProgressBar, ProgressIterator};
use itertools::Itertools;
use lazy_static::lazy_static;
use log::info;
use ordered_float::OrderedFloat;
use owo_colors::colors::css::Orange;
use owo_colors::colors::*;
use owo_colors::OwoColorize;
use rand::{rngs::SmallRng, seq::IteratorRandom, SeedableRng};
use rayon::iter::IntoParallelRefIterator;
use rayon::iter::ParallelIterator;
use regex::Regex;
use sqlx::postgres::PgPoolOptions;
use sqlx::types::chrono::Utc;
use sqlx::{Pool, Postgres};
use std::sync::{Arc, Mutex};

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

/// Finds commodities for a group of stations. The result is a map of IDs to the commodities at
/// that station.
async fn get_all_commodities(
    stations: &[Station],
    pool: &Pool<Postgres>,
    date_cutoff: &NaiveDateTime,
) -> Result<Arc<DashMap<i64, Vec<Commodity>>>> {
    let out: Arc<DashMap<i64, Vec<Commodity>>> = Arc::new(DashMap::new());

    let bar = Arc::new(ProgressBar::new(stations.len().try_into().unwrap()));
    futures::stream::iter(stations.iter())
        .for_each(|station1| {
            let pool = pool.clone();
            let bar = bar.clone();
            let out = out.clone();
            async move {
                bar.inc(1);
                let commodities = station1.get_commodities(&pool, date_cutoff).await.unwrap();
                out.insert(station1.id, commodities);
            }
        })
        .await;

    Ok(out)
}

lazy_static! {
    static ref FLEET_CARRIER_REGEX: Regex = Regex::new("[a-zA-Z0-9]{3}-[a-zA-Z0-9]{3}").unwrap();
}

/// Returns true if the station name is a fleet carrier
fn is_fleet_carrier(name: &str) -> bool {
    FLEET_CARRIER_REGEX.find(name).is_some()
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
    expiry: Option<u32>,
) -> Result<()> {
    println!("Setting up PostgreSQL pool on {}", url.fg::<Orange>());
    let var_name = PgPoolOptions::new();
    let pool = var_name.max_connections(32).connect(&url).await?;

    // compute date cutoff: if expiry is set, use now - expiry; otherwise use 1970-01-01
    let date_cutoff = match expiry {
        Some(exp) => (Utc::now() - TimeDelta::days(exp.into())).naive_utc(),
        None => NaiveDate::from_ymd_opt(1970, 1, 1).unwrap().into(),
    };

    match src {
        Some(source) => Ok(()),
        None => {
            println!("Fetching all stations");
            let stations = get_all_stations(&pool, landing_pad).await?;

            // the galaxy is very large, so randomly sample a number of stations
            let sample_size: usize = (sample_factor * (stations.len() as f32)) as usize;
            println!(
                "Computing random sample, factor: {} ({} stations)",
                sample_factor.fg::<Orange>(),
                sample_size.fg::<Orange>()
            );
            // use SmallRng for speed
            let mut rng = SmallRng::from_entropy();
            // ensure that we are only selecting stations that have a market and system attached to
            // them
            let filtered_stations: Vec<Station> = stations
                .iter()
                .filter(|station| {
                    station.market_id.is_some()
                        && station.system_id.is_some()
                        && !is_fleet_carrier(&station.name)
                })
                .cloned()
                .collect();

            // now we can compute the random subsample
            let sample: Vec<Station> = filtered_stations
                .iter()
                .choose_multiple(&mut rng, sample_size)
                .iter()
                .map(|it| (*it).clone())
                .collect();

            println!(
                "Retrieving all commodities for {} sampled stations",
                sample.len().fg::<Orange>()
            );
            let all_commodities = get_all_commodities(&sample, &pool, &date_cutoff).await?;

            println!(
                "Computing trades for {} stations (approx {} individual routes)",
                sample.len().fg::<Orange>(),
                // this is because its stations^2 minus self intersecting routes (like going from
                // A->A)
                (sample.len().pow(2) - sample.len()).fg::<Green>()
            );

            let bar = Arc::new(ProgressBar::new(sample.len().try_into().unwrap()));
            let all_solutions: Mutex<Vec<TradeSolution>> = Mutex::new(Vec::new());

            // here we compare every station with every other station in the list
            sample.clone().par_iter().for_each(|station1| {
                let bar = bar.clone();
                let commodities1 = all_commodities.get(&station1.id).unwrap().to_owned();
                {
                    for station2 in &sample {
                        // skip self
                        if station2.id == station1.id {
                            continue;
                        }
                        let commodities2 = all_commodities.get(&station2.id).unwrap().to_owned();

                        let solution = solve_knapsack(
                            StationMarket::new(station1.clone(), commodities1.clone()),
                            StationMarket::new(station2.clone(), commodities2.clone()),
                            capacity,
                            capital,
                        );

                        if let Some(sol) = solution {
                            let mut access = all_solutions.lock().unwrap();
                            access.push(sol.clone());
                        }
                    }
                    bar.clone().inc(1);
                }
            });

            bar.clone().finish();

            let solutions = all_solutions.lock().unwrap();
            let best_solutions: Vec<&TradeSolution> = solutions
                .iter()
                .sorted_by_key(|x| OrderedFloat(x.profit))
                .rev()
                .collect();

            println!("{}", "✨ Most optimal trades:".bold().fg::<Green>());
            for (i, trade) in best_solutions.iter().take(5).enumerate() {
                println!("{}. {}", i + 1, trade.dump_coloured(&pool).await);
                println!();
            }

            Ok(())
        }
    }
}

/// Finds cheapest commodities in the database
pub async fn find_cheapest(
    url: String,
    landing_pad: LandingPad,
    name: String,
    max_age: u32,
    min_quantity: u32,
) -> Result<()> {
    // info!("Setting up PostgreSQL pool on {url}");
    // let var_name = PgPoolOptions::new();
    // let pool = var_name.max_connections(32).connect(&url).await?;
    //
    // info!("Fetching all stations");
    // let stations = get_all_stations(&pool, landing_pad).await?;
    //
    // // ensure that we are only selecting stations that have a market and system attached to
    // // them
    // let filtered_stations: Vec<Station> = stations
    //     .into_iter()
    //     .filter(|station| {
    //         station.market_id.is_some()
    //             && station.system_id.is_some()
    //             && !is_fleet_carrier(&station.name)
    //     })
    //     .collect();
    //
    // info!(
    //     "Retrieving all commodities for {} filtered stations",
    //     filtered_stations.len()
    // );
    // let all_commodities = get_all_commodities(&filtered_stations, &pool, &cutoff).await?;
    //
    // info!("Finding best values");
    // let mut best_station: Option<Station> = None;
    // let mut best_commodity: Option<Commodity> = None;
    // let now = Utc::now().naive_utc();
    // for station in filtered_stations.iter().progress() {
    //     let commodities = all_commodities.get(&station.id).unwrap();
    //
    //     for commodity in commodities.iter() {
    //         // apply filter criteria
    //         let dur = now - commodity.listed_at;
    //         if commodity.name != name
    //             || commodity.stock < min_quantity.try_into()?
    //             || dur.num_days() > max_age.into()
    //         {
    //             continue;
    //         }
    //
    //         if best_commodity
    //             .as_ref()
    //             .is_none_or(|bc| commodity.sell_price < bc.sell_price)
    //         {
    //             best_station = Some(station.clone());
    //             best_commodity = Some(commodity.clone());
    //         }
    //     }
    // }
    //
    // info!("=== Best station ===");
    // if let Some(station) = best_station {
    //     let bc = best_commodity.unwrap();
    //     let system = sqlx::query!(
    //         r#"
    //         SELECT name FROM systems WHERE id = $1;
    //     "#,
    //         station.system_id,
    //     )
    //     .fetch_one(&pool)
    //     .await?;
    //
    //     info!(
    //         "{} in {} has {} {} available for {} CR each (listed on {})",
    //         station.name, system.name, bc.stock, name, bc.sell_price, bc.listed_at
    //     );
    // }
    //
    // // TODO show best 5 stations, not best station

    Ok(())
}
