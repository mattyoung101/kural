use crate::types::{Order, StationMarket, TradeSolution};
use good_lp::{constraint, highs, variable, Expression, ProblemVariables, Variable};
use good_lp::{Solution, SolverModel};
use log::{debug, error};
use std::collections::BTreeMap;

/// Solves an instance of the bounded knapsack problem using linear programming. Returns Some if a
/// solution could be computed, otherwise None.
pub fn solve_knapsack(
    source: StationMarket,
    destination: StationMarket,
    capacity: u32,
    capital: u64,
) -> Option<TradeSolution> {
    // FIXME we *need* to stop unwrappping shit in this routine

    // first, compute profit for all commodities from dest to source per unit carried
    // this maps a commodity name to an expected profit
    // we use a btreemap here for deterministic iteration order
    let mut profit: BTreeMap<String, i32> = BTreeMap::new();
    let all_dest_commodity_names: Vec<String> = destination
        .commodities
        .iter()
        .map(|commodity| commodity.name.clone())
        .collect();

    for commodity in &source.commodities {
        // check that this commodity is present in the destination
        if !all_dest_commodity_names.contains(&commodity.name) {
            continue;
        }

        let dest_commodity = destination.get_commodity(&commodity.name);
        if dest_commodity.is_none() {
            // commodity doesn't exist in destination system
            continue;
        }

        profit.insert(
            commodity.name.clone(),
            dest_commodity.unwrap().sell_price - commodity.buy_price,
        );
    }

    // no routes available
    if profit.is_empty() {
        return None;
    }

    // now, model the bounded knapsack problem:
    //
    // maximise
    //          sum_(i=1)^n v_i x_i
    // subject to (cargo hold constraint)
    //          sum_(i=1)^n x_i <= W where x_i in {0, 1, 2, ..., t_i}
    // subject to (capital constraint)
    //          sum_(i=1)^n c_i x_i <= C
    //
    // where:
    //  v_i = profit for the item
    //  x_i = number of copies of item x_i
    //  W = cargo hold capacity
    //  t_i = total available quantity for the item
    //  c_i = cost of item i
    //  C = total available capital

    let mut vars = ProblemVariables::new();
    // n items
    let n = profit.len();
    // this represents the number items
    let mut x: Vec<Variable> = Vec::with_capacity(n);

    for com in profit.keys() {
        // the max is the maximum number of items we can pick up in the source system
        let max = source.get_commodity(com).unwrap().stock;
        x.push(vars.add(variable().min(0).max(max).integer()));
    }

    // setup our objective which is sum_(i=1)^n v_i x_i
    // i.e. quantity x profit
    let mut objective = Expression::from(0.0);
    for (i, prof) in profit.values().enumerate() {
        objective += x[i] * *prof;
    }

    // setup the quantity and capital constraints
    let mut quantity_expr = Expression::from(0.0);
    let mut capital_expr = Expression::from(0.0);
    for (i, com) in profit.keys().enumerate() {
        quantity_expr += x[i];
        capital_expr += x[i] * source.get_commodity(com).unwrap().buy_price;
    }

    let solution = vars
        .maximise(&objective)
        .using(highs)
        .with(constraint!(quantity_expr <= capacity))
        .with(constraint!(capital_expr.clone() <= (capital as f64)))
        .solve();

    match solution {
        Ok(sol) => {
            let profit = sol.eval(&objective);
            let cost = sol.eval(capital_expr.clone());
            debug!(
                "Computed {} -> {} with profit {}",
                source.station.name, destination.station.name, profit
            );

            // the ILP solver will tell us how many of each commodity to order
            let orders: Vec<Order> = x
                .iter()
                .enumerate()
                .map(|(idx, var)| {
                    Order::new(
                        source.commodities[idx].name.clone(),
                        // FIXME we may be stupid -> .floor() as u32 is kind of dumb
                        // why is our ILP solve returning float valued constraints anyway?
                        sol.value(*var).floor() as u32,
                    )
                })
                .collect();

            Some(TradeSolution::new(
                source.station,
                destination.station,
                orders,
                profit,
                cost,
            ))
        }
        Err(err) => {
            error!(
                "Could not solve {} -> {}: {}",
                source.station.name, destination.station.name, err
            );
            None
        }
    }
}
