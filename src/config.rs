use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(author, version, about = "Solana Abritrge Opportunity Monitoring")]
pub struct CliArgs {
    #[clap(
        long,
        env("WS_ENDPOINT"),
        value_name = "WS_ENDPOINT",
        help = "Solana WS endpoint URL, which bot uses to subscribe to live pool price updates."
    )]
    pub ws_endpoint: String,

    #[clap(
        long,
        env("RPC_ENDPOINT"),
        value_name = "RPC_ENDPOINT",
        help = "Solana RPC endpoint URL, which bot uses to get static data like token decimals."
    )]
    pub rpc_endpoint: String,

    #[clap(
        long,
        env("WHIRLPOOL_ADDRESS"),
        value_name = "WHIRLPOOL_ADDRESS",
        help = "Whirlpool address on Solana"
    )]
    pub whirlpool_address: String,

    #[clap(
        long,
        env("CEX_TRADING_PAIR"),
        value_name = "CEX_TRADING_PAIR",
        help = "CEX Trading Pair, e.g. BTC/USD"
    )]
    pub cex_trading_pair: String,

    #[arg(
        long,
        env("MIN_GAIN_MARGIN"),
        value_name = "MIN_GAIN_MARGIN",
        default_value_t = 10,
        help = "Minumim required margin for bot to assume potential arbitrage, as parts per million"
    )]
    pub min_gain_margin: u64,

    #[arg(
        long,
        env("TOKEN_A_BUFFER"),
        value_name = "TOKEN_A_BUFFER",
        help = "Maximum token A units to invest"
    )]
    pub token_a_buffer: u64,
}
