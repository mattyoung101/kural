use geozero::wkb;
use sqlx::{types::chrono::NaiveDateTime, FromRow, Pool, Postgres};
use color_eyre::Result;

#[derive(Debug, FromRow)]
pub struct System {
    pub id: i64,
    pub name: String,
    pub date: NaiveDateTime,
    pub coords: wkb::Decode<geo_types::Geometry<f64>>,
}

#[derive(Debug, FromRow, Clone)]
pub struct Station {
    pub id: i64,
    pub name: String,
    pub distance_to_arrival: Option<f32>,
    pub market_id: Option<i64>,
    pub system_id: Option<i64>,
}

#[derive(Debug, FromRow, Clone)]
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
    pub listed_at: NaiveDateTime,
}

/// A station with an attached market
#[derive(Debug, Clone)]
pub struct StationMarket<'a> {
    pub station: &'a Station,
    pub commodities: &'a Vec<Commodity>
}

#[derive(Debug, FromRow, Clone)]
/// Order of commodities to buy or sell in a system
pub struct Order<'a> {
    pub commodity: &'a Commodity,
    pub count: u32
}

#[derive(Debug, FromRow, Clone)]
/// Solution to a knapsack problem
pub struct TradeSolution<'a> {
    /// List of commodities to buy in the source system
    pub buy: Vec<Order<'a>>,
    /// List of commodities to sell in the destination system
    pub sell: Vec<Order<'a>>
}

impl<'a> TradeSolution<'a> {
    pub fn new(buy: Vec<Order<'a>>, sell: Vec<Order<'a>>) -> Self {
        Self { buy, sell }
    }
}

impl<'a> StationMarket<'a> {
    pub fn new(station: &'a Station, commodities: &'a Vec<Commodity>) -> Self {
        Self { station, commodities }
    }

    /// Finds the commodity in the market
    pub fn get_commodity(self: &Self, name: &String) -> Option<&Commodity> {
        // FIXME we should look this up in a hashtable for perf; O(n) -> O(1)
        return self.commodities.into_iter().find(|commodity| *commodity.name == *name);
    }
}

impl Station {
    /// Gets the commodities in this station, assuming it has a market
    pub async fn get_commodities(self: &Station, pool: &Pool<Postgres>) -> Result<Vec<Commodity>, sqlx::Error> {
        // fetch commodities, for each commodity, only selecting the most recent
        // one using a common table subexpression
        // FIXME we should build this into the database instead of querying it every time for perf
        // like we should keep latest commodity in the database
        return sqlx::query_as!(Commodity,
            r#"
            WITH latest_listings AS (
                SELECT
                    market_id,
                    name,
                    MAX(listed_at) AS latest_listed_at
                FROM
                    listings
                WHERE
                    market_id = $1
                GROUP BY
                    market_id, name
            )
            SELECT
                l.market_id,
                l.name,
                l.mean_price,
                l.buy_price,
                l.sell_price,
                l.demand,
                l.demand_bracket,
                l.stock,
                l.stock_bracket,
                l.listed_at
            FROM
                listings l
            INNER JOIN
                latest_listings ll
            ON
                l.market_id = ll.market_id
                AND l.name = ll.name
                AND l.listed_at = ll.latest_listed_at;
            "#, self.market_id.unwrap())
        .fetch_all(pool).await;
    }
}
