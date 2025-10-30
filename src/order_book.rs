use kraken_async_rs::wss::BidAsk;
use rust_decimal::{Decimal};
use std::collections::BTreeMap;

type CexMarketPriceType = Decimal;
type CexQtyType = Decimal;
type PriceAndQuantities = BTreeMap<CexMarketPriceType, CexQtyType>;

pub enum Side {
    Bid,
    Ask,
}

#[derive(Default, Clone)]
pub struct OrderBook {
    bids: PriceAndQuantities,
    asks: PriceAndQuantities,
}

impl OrderBook {
    /// Applies a list of updates (snapshot or delta) to one side of the book.
    pub fn apply_updates(&mut self, side: Side, updates: &[BidAsk]) {
        let book = match side {
            Side::Bid => &mut self.bids,
            Side::Ask => &mut self.asks,
        };

        for bid_ask in updates {
            if bid_ask.quantity == Decimal::ZERO {
                book.remove(&bid_ask.price);
            } else {
                book.insert(bid_ask.price, bid_ask.quantity);
            }
        }
    }

    pub fn get_mid_price(&self) -> Option<Decimal> {
        let best_bid_price = self.bids.keys().next_back();
        let best_ask_price = self.asks.keys().next();

        match (best_bid_price, best_ask_price) {
            (Some(bid), Some(ask)) => {
                let sum = *bid + *ask;
                let two = Decimal::from(2);
                let mid_price = sum.checked_div(two)?;
                Some(mid_price)
            }
            _ => None,
        }
    }

    pub fn calculate_average_filled_price(
        &self,
        target_qty: Decimal,
        side: Side,
    ) -> Option<Decimal> {
        if target_qty.is_sign_negative() || target_qty.is_zero() {
            return Some(Decimal::ZERO);
        }

        let levels_iter: Box<dyn Iterator<Item = (&Decimal, &Decimal)>> = match side {
            Side::Ask => Box::new(self.asks.iter()),
            Side::Bid => Box::new(self.bids.iter().rev()),
        };

        let mut total_cost = Decimal::ZERO;
        let mut remaining_qty = target_qty;

        for (price, available_qty) in levels_iter {
            if remaining_qty.is_zero() {
                break;
            }

            let qty_to_take = remaining_qty.min(*available_qty);
            total_cost += price.checked_mul(qty_to_take)?;
            remaining_qty -= qty_to_take;
        }

        if remaining_qty.is_zero() {
            total_cost.checked_div(target_qty)
        } else {
            None
        }
    }
}