use crate::solve::solve_knapsack;
use crate::types::Coordinate;
use crate::types::{Commodity, Station, StationMarket, System, TradeSolution};
use crate::LandingPad;
use chrono::{NaiveDate, NaiveDateTime, TimeDelta};
use color_eyre::Result;
use dashmap::DashMap;
use futures::{executor, StreamExt};
use geozero::wkb;
use indicatif::ProgressBar;
use itertools::Itertools;
use lazy_static::lazy_static;
use ordered_float::OrderedFloat;
use owo_colors::colors::css::{DarkOrange, Orange};
use owo_colors::colors::*;
use owo_colors::OwoColorize;
use rand::{rngs::SmallRng, seq::IteratorRandom, SeedableRng};
use rayon::iter::IntoParallelRefIterator;
use rayon::iter::ParallelIterator;
use regex::Regex;
use sqlx::postgres::PgPoolOptions;
use sqlx::types::chrono::Utc;
use sqlx::{Pool, Postgres};
use std::collections::{HashMap, HashSet};
use std::process::exit;
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
        r#"
            SELECT s.id, s.name AS name, s.distance_to_arrival, s.market_id, s.system_id, y.name AS system_name
                FROM stations s
            INNER JOIN systems y ON y.id = s.system_id
                WHERE s.market_id IS NOT NULL AND s.system_id IS NOT NULL AND s.landing_pad LIKE $1;
        "#,
        pad_name
    )
    .fetch_all(pool)
    .await?);
}

/// Gets a list of all systems in range of the given system
async fn get_all_systems_in_range(
    pool: &Pool<Postgres>,
    source: &System,
    range: f64,
) -> Result<Vec<System>> {
    let coord = source.coords.geometry.expect("no coordinate");

    return Ok(sqlx::query_as!(
        System,
        r#"
            SELECT id, name, date, coords AS "coords!: wkb::Decode<Coordinate>"
                FROM systems
            WHERE ST_3DDWithin(coords, ST_MakePoint($1, $2, $3), $4)
        "#,
        coord.x,
        coord.y,
        coord.z,
        range,
    )
    .fetch_all(pool)
    .await?);
}

/// Gets a system by its name
async fn get_system_by_name(pool: &Pool<Postgres>, name: &String) -> Result<System> {
    return Ok(sqlx::query_as!(
        System,
        r#"
            SELECT id, name, date, coords AS "coords!: wkb::Decode<Coordinate>"
                FROM systems
            WHERE LOWER(name) = LOWER($1);
        "#,
        name,
    )
    .fetch_one(pool)
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
    src_search_ly: Option<f32>,
    capital: u64,
    capacity: u32,
    sample_factor: f32,
    landing_pad: LandingPad,
    expiry: Option<u32>,
    max_dst: Option<f32>,
) -> Result<()> {
    println!("Setting up PostgreSQL pool on {}", url.fg::<Orange>());
    let var_name = PgPoolOptions::new();
    let pool = var_name.max_connections(32).connect(&url).await?;

    // compute date cutoff: if expiry is set, use now - expiry; otherwise use 1970-01-01
    let date_cutoff = match expiry {
        Some(exp) => (Utc::now() - TimeDelta::days(exp.into())).naive_utc(),
        None => NaiveDate::from_ymd_opt(1970, 1, 1).unwrap().into(),
    };

    println!("Fetching all stations");
    let stations = get_all_stations(&pool, landing_pad).await?;

    // the galaxy is very large, so randomly sample a number of stations
    // FIXME handle cases where the number of stations is very small and we end up with a size of 0
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
    let mut sample: Vec<Station> = filtered_stations
        .iter()
        .choose_multiple(&mut rng, sample_size)
        .iter()
        .map(|it| (*it).clone())
        .collect();

    let all_solutions: Mutex<Vec<TradeSolution>> = Mutex::new(Vec::new());

    // FIXME this match needs a massive cleanup, we should collapse the Some and None arms
    match src {
        Some(ref source) => {
            let mut stations_filtered: Vec<Station> = Vec::new();

            if let Some(dst) = src_search_ly {
                let source_system =
                    get_system_by_name(&pool, &src.clone().expect("src must be specified")).await?;

                println!(
                    "Finding acceptable systems in {} LY range of {}",
                    dst.fg::<Orange>(),
                    source.fg::<Orange>()
                );
                let systems: HashSet<String> =
                    get_all_systems_in_range(&pool, &source_system, dst.into())
                        .await?
                        .iter()
                        .map(|x| x.name.clone())
                        .collect();
                println!(
                    "...found {} acceptable systems",
                    systems.len().fg::<Orange>()
                );

                println!("Now filtering stations");
                stations_filtered = stations
                    .iter()
                    .filter(|x| {
                        !is_fleet_carrier(&x.name)
                            && x.system_name
                                .clone()
                                .is_some_and(|it| systems.contains(&it))
                    })
                    .map(|x| (*x).clone())
                    .collect();
                println!(
                    "Have {} stations after filtering",
                    stations_filtered.len().fg::<Orange>()
                );
                // TODO randomly subsample stations_filtered further? if it's a large number?
            } else {
                // fixed source set
                // compare each station
                println!("Filtering all stations to fixed starting system '{source}'");
                stations_filtered = stations
                    .iter()
                    .filter(|x| {
                        x.system_name
                            .as_ref()
                            .is_some_and(|s| s.to_lowercase() == source.to_lowercase())
                    })
                    .map(|x| (*x).clone())
                    .collect();
            }

            // extend the random sample with our fixed subsample (for when we do market lookup)
            sample.extend(stations_filtered.clone().into_iter());

            println!(
                "Retrieving all commodities for {} sampled stations",
                sample.len().fg::<Orange>()
            );
            let all_commodities = get_all_commodities(&sample, &pool, &date_cutoff).await?;

            if all_commodities.is_empty() {
                eprintln!("No commodities could be found after applying filtering. Maybe adjust your date cutoff?");
                exit(1);
            }

            // nasty ass hack that we'll do to associate station names with system instances, since
            // we can't async inside the stations_filtered.par_iter()
            println!("Associating station names with system instances (hack), standby...");
            let mut stations_systems_map: HashMap<String, System> = HashMap::new();
            for station in &sample {
                if let Some(system_name) = &station.system_name {
                    stations_systems_map.insert(
                        station.name.clone(),
                        get_system_by_name(&pool, &system_name).await?,
                    );
                }
            }

            println!(
                "Computing trades for approx {} stations ({} '{source}'{})",
                stations_filtered.len().fg::<Orange>(),
                "with fixed start location".fg::<DarkOrange>(),
                if let Some(dst) = src_search_ly {
                    format!(" and within {dst} LY")
                        .fg::<DarkOrange>()
                        .to_string()
                } else {
                    "".to_string()
                }
            );

            let bar = Arc::new(ProgressBar::new(
                stations_filtered.len().try_into().unwrap(),
            ));

            stations_filtered.clone().par_iter().for_each(|station1| {
                let bar = bar.clone();
                let commodities1 = all_commodities.get(&station1.id).unwrap().to_owned();
                let station1_system = stations_systems_map
                    .get(&station1.name)
                    .expect("couldn't find system name");
                {
                    for station2 in &sample {
                        // skip self
                        if station2.id == station1.id {
                            continue;
                        }

                        // ensure the other station is within the max distance (if it was specified)
                        if let Some(dst) = max_dst {
                            let station2_system = stations_systems_map
                                .get(&station2.name)
                                .expect("couldn't find system name");

                            if station1_system
                                .coords
                                .geometry
                                .unwrap()
                                .dst(&station2_system.coords.geometry.unwrap())
                                > dst.into()
                            {
                                continue;
                            }
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
        }

        None => {
            // no fixed source set
            // here we compare every station with every other station in the list
            println!(
                "Retrieving all commodities for {} sampled stations",
                sample.len().fg::<Orange>()
            );
            let all_commodities = get_all_commodities(&sample, &pool, &date_cutoff).await?;
            if all_commodities.is_empty() {
                eprintln!("No commodities could be found after applying filtering. Maybe adjust your date cutoff?");
                exit(1);
            }

            println!(
                "Computing trades for {} stations (approx {} individual routes)",
                sample.len().fg::<Orange>(),
                // this is because its stations^2 minus self intersecting routes (like going from
                // A->A)
                (sample.len().pow(2) - sample.len()).fg::<Green>()
            );

            let bar = Arc::new(ProgressBar::new(sample.len().try_into().unwrap()));

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
        }
    }

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

/// Finds cheapest commodities in the database
pub async fn find_cheapest(
    url: String,
    landing_pad: LandingPad,
    name: String,
    max_age: u32,
    min_quantity: u32,
) -> Result<()> {
    Ok(())
}
