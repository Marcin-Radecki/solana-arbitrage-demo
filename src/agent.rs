use eyre::Result;
use rust_decimal::{Decimal, prelude::One};
use tokio::sync::mpsc::Receiver;
use tracing::info;

use crate::{
    config::CliArgs,
    dex_monitoring::DexData,
    order_book::{OrderBook, Side},
};

pub struct ArbitrageAgent {
    config: CliArgs,
    cex_receiver: Receiver<OrderBook>,
    dex_receiver: Receiver<DexData>,

    latest_cex_orderbook: Option<OrderBook>,
    latest_dex_price: Option<Decimal>,
}

impl ArbitrageAgent {
    pub fn new(
        config: CliArgs,
        cex_receiver: Receiver<OrderBook>,
        dex_receiver: Receiver<DexData>,
    ) -> Self {
        Self {
            config,
            cex_receiver,
            dex_receiver,
            latest_cex_orderbook: None,
            latest_dex_price: None,
        }
    }

    pub async fn run(&mut self) -> Result<()> {
        info!("Running with config {:?}", &self.config);

        loop {
            tokio::select! {
                Some(msg) = self.cex_receiver.recv() => {
                    self.handle_cex_message(msg);
                },
                Some(msg) = self.dex_receiver.recv() => {
                    self.handle_dex_message(msg);
                },
                else => {
                    info!("Both monitoring channels closed. Shutting down.");
                    break;
                }
            }

            self.check_for_arbitrage();
        }

        Ok(())
    }

    fn handle_cex_message(&mut self, msg: OrderBook) {
        self.latest_cex_orderbook = Some(msg);
    }

    fn handle_dex_message(&mut self, msg: DexData) {
        self.latest_dex_price = Some(msg.pool_price);
    }

    fn check_for_arbitrage(&self) {
        let (Some(order_book), Some(dex_price)) =
            (&self.latest_cex_orderbook, self.latest_dex_price)
        else {
            return;
        };
        let mid_cex_price = order_book.get_mid_price();
        if mid_cex_price.is_none() {
            return;
        }
        let mid_cex_price = mid_cex_price.expect("Just checked above");

        let token_a_buffer = Decimal::from(self.config.token_a_buffer);

        if dex_price > mid_cex_price {
            // trivial, not counting CEX fees
            if let Some(cex_buy_price) =
                order_book.calculate_average_filled_price(token_a_buffer, Side::Ask)
            {
                // very trivial, not counting pool fees or gas fees
                let dex_net_sell_price = dex_price;
                if dex_net_sell_price
                    > cex_buy_price
                        * (Decimal::one()
                            + Decimal::from(self.config.min_gain_margin) / Decimal::from(1_000_000))
                {
                    let units_of_token_b_to_sell = token_a_buffer * dex_net_sell_price;
                    info!(
                        "Potential arbitrage: Buy on CEX {} units on token A and sell on DEX {} units of token B",
                        token_a_buffer, units_of_token_b_to_sell
                    );
                    info!(
                        "dex_price: = {}, mid_cex_price = {}",
                        dex_price, mid_cex_price
                    );
                }
            }
        }
        if dex_price < mid_cex_price {
            // trivial, not counting CEX fees
            if let Some(cex_sell_price) =
                order_book.calculate_average_filled_price(token_a_buffer, Side::Bid)
            {
                // very trivial, not counting pool fees or gas fees
                let dex_gross_buy_price = dex_price;
                if cex_sell_price
                    > dex_gross_buy_price
                        * (Decimal::one()
                            + Decimal::from(self.config.min_gain_margin) / Decimal::from(1_000_000))
                {
                    let units_of_token_b_to_buy = token_a_buffer / dex_gross_buy_price;
                    info!(
                        "Potential arbitrage: Sell on CEX {} units on token A and buy on DEX {} units of token B",
                        token_a_buffer, units_of_token_b_to_buy
                    );
                    info!(
                        "dex_price: = {}, mid_cex_price = {}",
                        dex_price, mid_cex_price
                    );
                }
            }
        }
    }
}
