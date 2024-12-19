use geozero::wkb;
use sqlx::{types::chrono::NaiveDateTime, FromRow};

#[derive(Debug, FromRow)]
pub struct System {
    pub id: i64,
    pub name: String,
    pub date: NaiveDateTime,
    pub coords: wkb::Decode<geo_types::Geometry<f64>>,
}

#[derive(Debug, FromRow)]
pub struct Station {
    pub id: i64,
    pub name: String,
    pub distance_to_arrival: Option<f32>,
    pub market_id: Option<i64>,
    pub system_id: Option<i64>,
}

#[derive(Debug, FromRow)]
pub struct Commodity {
    pub market_id: i64,
    pub name: String,
    pub mean_price: i32,
    pub buy_price: i32,
    pub sell_price: i32,
    pub demand: i32,
    pub demand_bracket: i32,
    pub stock: i32,
    pub stock_bracket: i32,
    pub listed_at: NaiveDateTime
}
