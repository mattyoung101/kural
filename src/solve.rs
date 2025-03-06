use good_lp::{constraint, highs, microlp, solvers::coin_cbc::{self, coin_cbc}, variable, variables, ProblemVariables};
use log::info;
use good_lp::SolverModel;
use crate::types::{StationMarket, TradeSolution};
use std::collections::HashMap;

/// Solves an instance of the bounded knapsack problem using linear programming. Returns Some if a
/// solution could be computed, otherwise None.
pub fn solve_knapsack<'a>(
    source: StationMarket<'a>,
    destination: StationMarket<'a>,
    capacity: u32,
    capital: u64,
) -> Option<TradeSolution<'a>> {
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

        profit.insert(
            &commodity.name,
            dest_commodity.unwrap().sell_price - commodity.buy_price,
        );
    }

    // no routes available
    if profit.is_empty() {
        return None
    }

    // now, model the bounded knapsack problem:
    //
    // maximise
    //          sum_(i=1)^n v_i x_i
    // subject to
    //          sum_(i=1)^n x_i <= W where x_i in {0, 1, 2, ..., t_i}
    // subject to
    //          sum_(i=1)^n c_i x_i <= C
    //
    // where:
    //  v_i = profit for the item
    //  x_i = number of copies of item x_i
    //  W = cargo hold capacity
    //  t_i = total available quantity for the item
    //  c_i = cost of item i
    //  C = total available capital

    // let mut vars = ProblemVariables::new();
    // for commodity in profit.keys() {
    //     vars.add(variable().min(0).max(max));
    // }

    // variables! {
    //     vars:
    //            a <= 1;
    //       2 <= b <= 4;
    // } // variables can also be added dynamically
    // let solution = vars.maximise(10 * (a - b / 5) - b)
    //     .using(microlp) // multiple solvers available
    //     .with(constraint!(a + 2 <= b))
    //     .with(constraint!(1 + a >= 4 - b))
    //     .solve().unwrap();

    return Some(TradeSolution::new(Vec::new(), Vec::new()));
}
