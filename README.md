# Kural
_Kural_ is a high-performance trade route calculator for Elite: Dangerous. It works by using [integer linear
programming](https://en.wikipedia.org/wiki/Integer_programming) to solve large scale instances of the [bounded
knapsack problem](https://en.wikipedia.org/wiki/Knapsack_problem).

Currently, the tool is planned to be able to compute single-hop, multi-commodity trade routes. In the future,
I would like to also compute multi-hop, multi-commodity routes across the entire galaxy.

Kural uses data from a PostgreSQL/PostGIS instance archived by my other project,
[EDTear](https://github.com/mattyoung101/edtear), the Elite: Dangerous Trade Ear.

Why the name? Kural refers to the Tirukkuṟaḷ, an ancient and highly-regarded Tamil text that may provide the
first observation of supply and demand:

> "If people do not consume a product or service, then there will not be anybody to supply that product or
> service for the sake of price."
>
> [source](https://en.wikipedia.org/wiki/Supply_and_demand#History)

## Running
First, you need a running instance of my other project, EDTear (see above). I plan to release periodic dumps
for my database which has been running since c. Jan 2025, and has ~200 million records.

Once EDTear has been set up, running Kural is relatively straightforward. Currently, only the `compute-single`
mode is supported for computing single-hop galactic trade routes with random susbsampling.

An example of computing a route is as follows:


```bash
cargo run --release  -- compute-single \
    --url "postgres://postgres:password@lagoon:6543/edtear" \
    --jump 42.69 \
    --capital 500000 \
    --capacity 500 \
    --random-sample 0.01 \
    --landing-pad large
```

This should then produce output similar to the following:

```
Setting up PostgreSQL pool on postgres://postgres:password@lagoon:6543/edtear
Fetching all stations
Computing random sample, factor: 0.012 (404 stations)
Retrieving all commodities for 404 sampled stations
Computing trades for 404 stations (approx 162812 individual routes)
✨ Most optimal trades:
1. ➡️ For 7,432,320 CR profit:
    Travel to Nahavandi Station in HIP 23824 and buy (for 999,676 CR):
        363x  benitoite                           (updated 2 days ago)
        136x  hostage                             (updated 2 days ago)
    Then, travel to Gardner Horizons in Kremata and sell.


2. ➡️ For 7,390,736 CR profit:
    Travel to Gonnessiat City in Miquit and buy (for 998,092 CR):
        359x  benitoite                           (updated 21 hours ago)
        141x  hydrogenfuel                        (updated 21 hours ago)
    Then, travel to Gardner Horizons in Kremata and sell.


3. ➡️ For 7,142,816 CR profit:
    Travel to Murray Refinery in Sosolingati and buy (for 997,796 CR):
        336x  benitoite                           (updated 3 days ago)
        164x  hydrogenperoxide                    (updated 3 days ago)
    Then, travel to Gardner Horizons in Kremata and sell.


4. ➡️ For 7,034,530 CR profit:
    Travel to Shajn Prospect in Vajrapese and buy (for 998,162 CR):
        326x  bertrandite                         (updated 2 days ago)
        174x  imperialslaves                      (updated 2 days ago)
    Then, travel to Gardner Horizons in Kremata and sell.


5. ➡️ For 7,021,000 CR profit:
    Travel to Dogmaa in Wolfberg and buy (for 353,000 CR):
        500x  bauxite                             (updated 10 hours ago)
    Then, travel to Crown Terminal in Candiaei and sell.

```

## Licence
Copyright (c) 2024-2025 M. Young.

Kural is available under the ISC licence.
