use eyre::{Context, Result, bail};
use tokio::sync::mpsc::Sender;

use kraken_async_rs::wss::{BookSubscription, Message, WssMessage};
use kraken_async_rs::wss::{KrakenWSSClient, L2};

use std::time::Duration;
use tokio::time::timeout;
use tokio_stream::StreamExt;
use tracing::{debug, error, warn};

use crate::order_book::{OrderBook, Side};

pub struct CexMonitoring {
    arbitrage_bot_sender: Sender<OrderBook>,
    order_book: OrderBook,
}

impl CexMonitoring {
    const ORDER_BOOK_DEPTH: i32 = 10;
    const KRAKEN_STREAM_TIMEOUT_SECONDS: u64 = 10;

    pub fn new(arbitrage_bot_sender: Sender<OrderBook>) -> Self {
        Self {
            arbitrage_bot_sender,
            order_book: OrderBook::default(),
        }
    }

    pub async fn run(mut self, trading_pair: String) -> Result<()> {
        let mut client = KrakenWSSClient::new();
        let mut kraken_stream = client
            .connect::<WssMessage>()
            .await
            .wrap_err("Failed to subscribe to CEX websocket!")?;

        let mut book_params = BookSubscription::new(vec![trading_pair]);
        book_params.depth = Some(Self::ORDER_BOOK_DEPTH);
        book_params.snapshot = Some(true);
        let subscription = Message::new_subscription(book_params, 0);

        kraken_stream
            .send(&subscription)
            .await
            .wrap_err("Failed to send snapshot message to Kraken stream!")?;

        while let Ok(Some(message)) = timeout(
            Duration::from_secs(Self::KRAKEN_STREAM_TIMEOUT_SECONDS),
            kraken_stream.next(),
        )
        .await
        {
            let message = message.wrap_err("Error receiving message: {}")?;
            self.parse_message(message).await?;
        }

        Ok(())
    }

    async fn parse_message(&mut self, message: WssMessage) -> Result<()> {
        debug!("{:?}", &message);
        match message {
            WssMessage::Channel(channel_message) => match channel_message {
                kraken_async_rs::wss::ChannelMessage::Heartbeat => {
                    debug!("Recieved channel heartbeat")
                }
                kraken_async_rs::wss::ChannelMessage::Status(single_response) => {
                    debug!("Received status message: {:?}", single_response)
                }
                kraken_async_rs::wss::ChannelMessage::Orderbook(single_response) => {
                    self.parse_order_book_data(single_response.data).await?
                }
                other_message => warn!("Unexpected message from the stream: {:?}", other_message),
            },
            WssMessage::Method(method_message) => {
                debug!("Unexpected message from the stream: {:?}", method_message)
            }
            WssMessage::Error(error_response) => {
                error!(
                    "Received error message from the stream: {:?}",
                    error_response
                )
            }
        }

        Ok(())
    }

    async fn parse_order_book_data(&mut self, order_book_data: L2) -> Result<()> {
        match order_book_data {
            L2::Orderbook(orderbook) => {
                self.order_book.apply_updates(Side::Ask, &orderbook.asks);
                self.order_book.apply_updates(Side::Bid, &orderbook.bids);
            }
            L2::Update(orderbook_update) => {
                self.order_book
                    .apply_updates(Side::Ask, &orderbook_update.asks);
                self.order_book
                    .apply_updates(Side::Bid, &orderbook_update.bids);
            }
        }

        if let Err(e) = self
            .arbitrage_bot_sender
            .send(self.order_book.clone())
            .await
        {
            bail!(format!("Arbitrage bot channel closed: {:?}", e));
        }

        Ok(())
    }
}
