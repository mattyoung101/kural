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
Computing random sample, factor: 0.01 (336 stations)
Retrieving all commodities for 336 sampled stations
Computing trades for 336 stations (approx 112560 individual routes)
1. ➡️ For 10,850,000 CR profit:
    Travel to Ronis Metallurgic Complex in Unjadi and buy (for 320,500 CR):
        500x  bauxite                             (updated 4 months ago)
    Then, travel to Diesel Ring in Ramit and sell.


2. ➡️ For 10,826,500 CR profit:
    Travel to Le Guin Vision in HIP 12716 and buy (for 344,000 CR):
        500x  bauxite                             (updated a month ago)
    Then, travel to Diesel Ring in Ramit and sell.


3. ➡️ For 10,818,000 CR profit:
    Travel to Kamov Survey in HIP 17892 and buy (for 352,500 CR):
        500x  bauxite                             (updated 6 days ago)
    Then, travel to Diesel Ring in Ramit and sell.


4. ➡️ For 10,808,000 CR profit:
    Travel to Fossum Hub in Mechucos and buy (for 362,500 CR):
        500x  bauxite                             (updated 2 weeks ago)
    Then, travel to Diesel Ring in Ramit and sell.


5. ➡️ For 10,807,500 CR profit:
    Travel to Matthaus Olbers Dock in Jita and buy (for 363,000 CR):
        500x  bauxite                             (updated 3 days ago)
    Then, travel to Diesel Ring in Ramit and sell.

```

## Licence
Copyright (c) 2024-2025 M. Young.

Kural is available under the ISC licence.
