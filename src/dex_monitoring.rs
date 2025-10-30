use eyre::{Result, bail};
use futures_util::stream::StreamExt;
use orca_whirlpools_client::Whirlpool;
use orca_whirlpools_core::sqrt_price_to_price;
use rust_decimal::{Decimal, prelude::FromPrimitive};
use solana_account_decoder_client_types::UiAccountEncoding;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_pubkey::Pubkey as Pubkey1;
use solana_sdk::{program_pack::Pack, pubkey::Pubkey};
use spl_token_2022::state::Mint;
use tokio::sync::mpsc::Sender;
use tracing::{debug, info};

use solana_commitment_config::CommitmentConfig;
use solana_pubsub_client::nonblocking::pubsub_client::PubsubClient;
use solana_rpc_client_types::config::RpcAccountInfoConfig;

use crate::config::CliArgs;

#[derive(Debug)]
pub struct DexData {
    pub pool_price: Decimal,
}

pub struct DexMonitoring {
    messages_to_bot: Sender<DexData>,
    ws_client: PubsubClient,
    rpc_client: RpcClient,
    whirlpool_address: String,
}

impl DexMonitoring {
    pub async fn new(
        messages_to_bot: Sender<DexData>,
        config: CliArgs,
    ) -> Result<Self> {
        Ok(Self {
            messages_to_bot,
            ws_client: PubsubClient::new(&config.ws_endpoint).await?,
            rpc_client: RpcClient::new(config.rpc_endpoint),
            whirlpool_address: config.whirlpool_address.clone(),
        })
    }

    async fn fetch_mint(&self, mint_address: &Pubkey) -> Result<Mint> {
        let mint_account = self.rpc_client.get_account(mint_address).await?;
        let mint = Mint::unpack(&mint_account.data)?;
        Ok(mint)
    }

    async fn parse_account_update(&self, whirlpool: Whirlpool) -> Result<DexData> {
        let token_mint_a = self.fetch_mint(&whirlpool.token_mint_a).await?;
        let token_mint_b = self.fetch_mint(&whirlpool.token_mint_b).await?;

        let current_price = sqrt_price_to_price(
            whirlpool.sqrt_price,
            token_mint_a.decimals,
            token_mint_b.decimals,
        );
        let pool_price =  Decimal::from_f64(current_price)
                .ok_or(eyre::eyre!("Failed to convert f64 to Decimal!"))?;
        let dex_data = DexData {
            pool_price,
        };
        Ok(dex_data)
    }

    pub async fn run(&self) -> Result<()> {
        let pools_account = Pubkey1::from_str_const(&self.whirlpool_address);

        let (mut stream, _) = self
            .ws_client
            .account_subscribe(
                &pools_account,
                Some(RpcAccountInfoConfig {
                    encoding: Some(UiAccountEncoding::Base64),
                    data_slice: None,
                    commitment: Some(CommitmentConfig::confirmed()),
                    min_context_slot: None,
                }),
            )
            .await?;

        while let Some(account) = stream.next().await {
            let account_data = &account
                .value
                .data
                .decode()
                .ok_or(eyre::eyre!("Failed to decode account data!"))?;
            let whirlpool = Whirlpool::from_bytes(account_data)?;
            
            let dex_data = self.parse_account_update(whirlpool).await?;
            debug!("{:?}", dex_data);
            
            if let Err(e) = self.messages_to_bot.send(dex_data).await {
                bail!(format!("Arbitrage bot channel closed: {:?}", e));
            }
        }

        info!("DEX Monitoring finished job");

        Ok(())
    }
}
