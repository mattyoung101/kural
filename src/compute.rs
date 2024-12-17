use log::info;
use sqlx::{postgres::PgPoolOptions, Executor};
use color_eyre::Result;

pub async fn compute_single(url: String, src: Option<String>, jump: f32, capital: u64) -> Result<()> {
    info!("Setting up PostgreSQL pool on {}", url);
    let var_name = PgPoolOptions::new();
    let pool = var_name
        .max_connections(8)
        .connect(&url)
        .await?;

    match src {
        Some(source) => {
            Ok(())
        }
        None => {
            info!("Fetching all systems");
            let systems = pool.fetch(sqlx::query!(r#"
                SELECT * FROM systems
            "#));

            Ok(())
        }
    }
}
