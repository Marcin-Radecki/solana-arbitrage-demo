# Solana Arbitrage Bot

## Overview

It's not really a bot, it's rather an illustration of how to use Rust to get Orca Whirlpool price on Solana and price of some CEX market (from Kraken).
Those prices are compared to get potential arbitrage. There is no execution of any transactions, just logging to std out. Also, neither fees are taken
into account on DEX side nor CEX side, although whole order book is taken into account in CEX side. 

## Design

Overall, the bot works as simple down-to-bottom pipeline, with two tasks providing prices and third task compares them:
1. Using given solanna WS endpoint from config, subscribe to given Orcla Whirlpool, which resembles a given market (e.g. SOL/USDC).
This is just an account on Solana, which has a `.data` field that is decoded into Orca Whirlpool structure. That structure has `price_sqrt` field.
2. On the same time, bot monitors the same market on CEX. by using Kraken WS subscriptiion. Whole order book updates are processed.
3. Both order book and DEX price is streamed down to third task, called `ArbitrageAgent`. If the difference betwen dex price and average filled price 
for some given token A amount (from config), is more than configured minimum margin (also from config), bot marks this as potential arbitrage 
and logs to std out.

What is not calculated and therefore make this program a very basic usage is lack of fee calculation, on either side. On Solana side, one can quite
easily extend Swap fee calculation (https://dev.orca.so/SDKs/Trade/), but this needs some local Solana wallet.

## Usage

Just run
```fish
source .env.fish
cargo run --release
```

or in bash
```bash
source .env
cargo run --release
```

If nothing happens for a while, you can run
```bash
RUST_LOG=debug cargo run --release
```
to see live stream of prices.

To shut down server, press Ctrl+C.
