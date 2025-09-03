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

## Licence
Copyright (c) 2024-2025 M. Young.

Kural is available under the ISC licence.
