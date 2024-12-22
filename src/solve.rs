use std::collections::HashMap;

use crate::types::{Commodity, Station, StationMarket, TradeSolution};

/// Solves an instance of the bounded knapsack problem using linear programming
pub fn solve_knapsack<'a>(
    source: StationMarket<'a>,
    destination: StationMarket<'a>,
    capacity: u32,
    capital: u64,
) -> TradeSolution<'a> {
    // first, compute profit for all commodities from dest to source per item
    let mut profit: HashMap<&String, i32> = HashMap::new();
    let all_dest_commodity_names: Vec<&String> = destination
        .commodities
        .into_iter()
        .map(|commodity| &commodity.name)
        .collect();

    for commodity in source.commodities {
        // check that this commodity is present in the destination
        if !all_dest_commodity_names.contains(&&commodity.name) {
            continue;
        }

        let dest_commodity = destination.get_commodity(&commodity.name);
        if dest_commodity.is_none() {
            // commodity doesn't exist in destination system
            continue;
        }

        profit.insert(&commodity.name, dest_commodity.unwrap().sell_price - commodity.buy_price);
    }

    return TradeSolution::new(Vec::new(), Vec::new());
}
