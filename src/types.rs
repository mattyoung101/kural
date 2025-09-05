use chrono::DateTime;
use chrono::NaiveDate;
use chrono::NaiveDateTime;
use chrono::Utc;
use color_eyre::Result;
use count_digits::CountDigits;
use geozero::wkb;
use owo_colors::colors::css::DarkOrange;
use owo_colors::colors::css::Orange;
use owo_colors::colors::*;
use owo_colors::OwoColorize;
use sqlx::{FromRow, Pool, Postgres};
use thousands::Separable;

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
    pub system_name: Option<String>,
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
pub struct StationMarket {
    pub station: Station,
    pub commodities: Vec<Commodity>,
}

#[derive(Debug, FromRow, Clone)]
/// Order of commodities to buy or sell in a system
pub struct Order {
    pub commodity_name: String,
    pub count: u32,
}

impl Order {
    pub fn new(commodity_name: String, count: u32) -> Self {
        Self {
            commodity_name,
            count,
        }
    }
}

#[derive(Debug, FromRow, Clone)]
/// Solution to a knapsack problem
pub struct TradeSolution {
    /// Source station
    pub source: Station,
    /// Destination station
    pub destination: Station,
    /// List of commodities to buy in the source system
    pub buy: Vec<Order>,
    /// Profit expected
    pub profit: f64,
    /// Cost to execute the trade
    pub cost: f64,
}

impl TradeSolution {
    pub fn new(
        source: Station,
        destination: Station,
        buy: Vec<Order>,
        profit: f64,
        cost: f64,
    ) -> Self {
        Self {
            source,
            destination,
            buy,
            profit,
            cost,
        }
    }

    pub async fn dump_coloured(&self, pool: &Pool<Postgres>) -> String {
        let mut str = format!(
            "➡️ For {} CR profit:\n    Travel to {} in {} and buy (for {} CR):\n",
            self.profit.round().separate_with_commas().fg::<Green>(),
            self.source.name.fg::<Orange>(),
            self.source.get_system_name(pool).await.fg::<Orange>(),
            // often we just get like .000006, so ignore it for the buy cost
            self.cost.round().separate_with_commas().fg::<Red>(),
        );

        let commodities = self
            .source
            .get_commodities(pool, &NaiveDate::from_ymd_opt(1970, 1, 1).unwrap().into())
            .await
            .unwrap();
        let market = StationMarket::new(self.source.clone(), commodities);

        for order in &self.buy {
            if order.count == 0 {
                continue;
            }

            let update = market
                .get_commodity(&order.commodity_name)
                .unwrap()
                .listed_at;
            let dur = chrono_humanize::HumanTime::from(update - Utc::now().naive_utc());
            let spacing = 32 - order.commodity_name.len() + 4;

            let digit_spacing = 4 - order.count.count_digits() + 1;

            str += &format!(
                "        {}x{}{}{}(updated {})\n",
                order.count,
                " ".repeat(digit_spacing),
                order.commodity_name,
                " ".repeat(spacing),
                dur.fg::<DarkOrange>()
            )
            .to_string();
        }
        str += &format!(
            "    Then, travel to {} in {} and sell.\n",
            self.destination.name.fg::<Orange>(),
            self.destination.get_system_name(pool).await.fg::<Orange>()
        );

        str
    }
}

impl StationMarket {
    pub fn new(station: Station, commodities: Vec<Commodity>) -> Self {
        Self {
            station,
            commodities,
        }
    }

    /// Finds the commodity in the market
    pub fn get_commodity(&self, name: &String) -> Option<Commodity> {
        // FIXME we should look this up in a hashtable for perf; O(n) -> O(1)
        self.commodities
            .iter()
            .find(|commodity| *commodity.name == *name)
            .cloned()
    }
}

impl Station {
    pub async fn get_system_name(self: &Station, pool: &Pool<Postgres>) -> String {
        return sqlx::query!(
            r#"
                SELECT name
                FROM systems
                WHERE id = $1;
            "#,
            self.system_id
        )
        .fetch_one(pool)
        .await
        .unwrap()
        .name;
    }

    /// Gets the commodities in this station, assuming it has a market
    pub async fn get_commodities(
        self: &Station,
        pool: &Pool<Postgres>,
        date_cutoff: &NaiveDateTime,
    ) -> Result<Vec<Commodity>, sqlx::Error> {
        // fetch commodities, for each commodity, only selecting the most recent
        // one using a common table subexpression
        return sqlx::query_as!(
            Commodity,
            r#"
                SELECT DISTINCT ON (l.name)
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
                FROM listings l
                WHERE l.market_id = $1 AND l.listed_at >= $2
                ORDER BY l.name, l.listed_at DESC;
            "#,
            self.market_id.unwrap(),
            date_cutoff,
        )
        .fetch_all(pool)
        .await;
    }
}
