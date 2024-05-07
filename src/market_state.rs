use getset::{CopyGetters, Getters, Setters};

use crate::{
    prelude::PriceFilter,
    quote,
    types::{Currency, MarketUpdate, QuoteCurrency, Result, TimestampNs},
};

/// Some information regarding the state of the market.
#[derive(Debug, Clone, Getters, CopyGetters, Setters)]
pub struct MarketState {
    /// Used to validate states
    // TODO: remove here and pass through were needed
    price_filter: PriceFilter,

    /// The current bid
    #[getset(get_copy = "pub", set = "pub(crate)")]
    bid: QuoteCurrency,

    /// The current ask
    #[getset(get_copy = "pub", set = "pub(crate)")]
    ask: QuoteCurrency,

    /// The current timestamp in nanoseconds
    #[getset(get_copy = "pub")]
    current_ts_ns: TimestampNs,

    /// Used for synchronizing orders.
    #[getset(get_copy = "pub")]
    step: u64,
}

impl MarketState {
    pub(crate) fn new(price_filter: PriceFilter) -> Self {
        Self {
            price_filter,
            bid: quote!(0),
            ask: quote!(0),
            current_ts_ns: 0,
            step: 0,
        }
    }

    /// Update the exchange state with new information
    ///
    /// ### Parameters:
    /// `timestamp_ns`: Is used in the AccountTracker `A`
    ///     and if setting order timestamps is enabled in the config.
    /// `market_update`: Newest market information
    ///
    pub(crate) fn update_state<U, Q, UserOrderId>(
        &mut self,
        timestamp_ns: TimestampNs,
        market_update: &U,
    ) -> Result<()>
    where
        U: MarketUpdate<Q, UserOrderId>,
        Q: Currency,
        UserOrderId: Clone,
    {
        market_update.validate_market_update(&self.price_filter)?;
        market_update.update_market_state(self);

        self.current_ts_ns = timestamp_ns;
        self.step += 1;

        Ok(())
    }

    /// Get the mid price
    #[inline]
    pub fn mid_price(&self) -> QuoteCurrency {
        (self.bid + self.ask) / quote!(2)
    }

    /// Get the last observed timestamp in nanoseconts
    #[inline]
    pub fn current_timestamp_ns(&self) -> TimestampNs {
        self.current_ts_ns
    }

    #[cfg(test)]
    pub fn from_components(
        price_filter: PriceFilter,
        bid: QuoteCurrency,
        ask: QuoteCurrency,
        current_ts_ns: i64,
        step: u64,
    ) -> Self {
        Self {
            price_filter,
            bid,
            ask,
            current_ts_ns,
            step,
        }
    }
}
