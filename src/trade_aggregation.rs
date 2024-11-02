use trade_aggregation::TakerTrade;

use crate::prelude::*;

impl<I, const D: u8, BaseOrQuote> TakerTrade for Trade<I, D, BaseOrQuote>
where
    I: Mon<D>,
    BaseOrQuote: Currency<I, D>,
{
    #[inline(always)]
    fn timestamp(&self) -> i64 {
        *self.timestamp_exchange_ns.as_ref()
    }

    #[inline(always)]
    fn price(&self) -> f64 {
        self.price.into()
    }

    #[inline(always)]
    fn size(&self) -> f64 {
        match self.side {
            Side::Buy => self.quantity.into(),
            Side::Sell => self.quantity.neg().into(),
        }
    }
}

impl<I, const D: u8, BaseOrQuote> Into<trade_aggregation::Trade> for Trade<I, D, BaseOrQuote>
where
    I: Mon<D>,
    BaseOrQuote: Currency<I, D>,
{
    #[inline]
    fn into(self) -> trade_aggregation::Trade {
        trade_aggregation::Trade {
            timestamp: *self.timestamp_exchange_ns.as_ref(),
            price: self.price.into(),
            size: <Trade<I, D, BaseOrQuote> as TakerTrade>::size(&self),
        }
    }
}
