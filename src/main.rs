use std::{env, io};
use tokio::sync::mpsc::{channel};

use crate::{agent::ArbitrageAgent, cex_monitoring::CexMonitoring, config::CliArgs, dex_monitoring::{DexData, DexMonitoring}, order_book::OrderBook};

mod cex_monitoring;
mod config;
mod dex_monitoring;
mod order_book;
mod agent;

use clap::Parser;
use eyre::{Result, eyre};
use tracing::{error};
use tracing_subscriber::EnvFilter;

fn init_logging() -> Result<()> {
    const LOG_CONFIGURATION_ENVVAR: &str = "RUST_LOG";
    let filter = EnvFilter::new(
        env::var(LOG_CONFIGURATION_ENVVAR)
            .as_deref()
            .unwrap_or("info"),
    );

    let subscriber = tracing_subscriber::fmt()
        .with_writer(io::stdout)
        .with_target(true)
        .with_env_filter(filter);

    subscriber.try_init().map_err(|err| eyre!(err))
}

#[tokio::main]
async fn main() {
    init_logging().expect("Failed to initialize logging!");

    let cli_args = CliArgs::parse();

    let (cex_messages_to_bot, cex_messages_from_bot) = channel::<OrderBook>(32);
    let cex_monitororing = CexMonitoring::new(cex_messages_to_bot);
    let cex_trading_pair = cli_args.cex_trading_pair.clone();
    let cex_monitoring_handler = tokio::spawn(async move {
        if let Err(e) = cex_monitororing.run(cex_trading_pair).await {
            error!("CEX Monitoring task failed: {}", e);
        }
    });

    let (dex_messages_to_bot, dex_messages_from_bot) = channel::<DexData>(32);
    let dex_monitoring = DexMonitoring::new(
        dex_messages_to_bot,
        cli_args.clone(),
    )
    .await
    .expect("Failed to create DEX Monitoring!");
    let dex_monitor_handle = tokio::spawn(async move {
        if let Err(e) = dex_monitoring.run().await {
            error!("DEX Monitoring task failed: {}", e);
        }
    });
    
    let mut agent = ArbitrageAgent::new(cli_args, cex_messages_from_bot, dex_messages_from_bot);
    let agent_handle = tokio::spawn(async move {
        if let Err(e) = agent.run().await {
            error!("Arbitrage Agent failed: {}", e);
        }
    });

    agent_handle.await.unwrap();
    dex_monitor_handle.await.unwrap();
    cex_monitoring_handler.await.unwrap();
}
