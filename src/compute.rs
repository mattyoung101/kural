use std::sync::Arc;

use crate::types::{Station, System, Commodity};
use color_eyre::Result;
use futures::StreamExt;
use indicatif::ProgressBar;
use log::info;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use sqlx::{postgres::PgPoolOptions, Executor};

pub async fn compute_single(
    url: String,
    src: Option<String>,
    jump: f32,
    capital: u64,
) -> Result<()> {
    info!("Setting up PostgreSQL pool on {}", url);
    let var_name = PgPoolOptions::new();
    let pool = var_name.max_connections(32).connect(&url).await?;

    match src {
        Some(source) => Ok(()),
        None => {
            info!("Fetching all stations");

            // let systems = sqlx::query_as!(
            //     System,
            //     r#"select id, name, date, coords as "coords!: _" from systems;"#
            // )
            // .fetch_all(&pool)
            // .await?;
            //
            // let _ = systems.par_iter().for_each(|system| {
            //     info!("Consider {}", system.name);
            // });

            let stations = sqlx::query_as!(
                Station,
                r#"SELECT
                    id, name, distance_to_arrival, market_id, system_id
                FROM stations
                WHERE market_id is not null and system_id is not null
                ;"#
            )
            .fetch_all(&pool)
            .await?;

            info!("Processing stations");
            let bar = Arc::new(ProgressBar::new(stations.len().try_into().unwrap()));

            futures::stream::iter(stations)
                .for_each(|station| {
                    let value = pool.clone();
                    let bar = bar.clone();
                    async move {
                        if station.system_id.is_none() || station.market_id.is_none() {
                            return
                        }
                        let system = sqlx::query_as!(
                            System,
                            r#"select id, name, date, coords as "coords!: _" from systems where id = $1;"#,
                            station.system_id.unwrap()
                        )
                        .fetch_one(&value)
                        .await
                        .unwrap();

                        bar.inc(1);

                        // info!("Considering station {} in {}", station.name, system.name);

                        // FIXME this needs to be instructed to pick the most recent commodity for
                        // each commodity
                        let commodities = sqlx::query_as!(Commodity,
                            r#"select
                                market_id,
                                name,
                                mean_price,
                                buy_price,
                                sell_price,
                                demand,
                                demand_bracket,
                                stock,
                                stock_bracket,
                                listed_at
                            from
                                listings
                            where market_id = $1;
                                "#, station.market_id.unwrap());
                    }
                })
                .await;

            bar.finish();

            Ok(())
        }
    }
}
