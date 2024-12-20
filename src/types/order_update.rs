use super::{Currency, Filled, LimitOrder, Mon, Pending, UserOrderIdT};

/// Contains the possible updates to limit orders.
#[derive(Debug, Clone, Eq, PartialEq, derive_more::Display)]
pub enum LimitOrderUpdate<I, const D: u8, BaseOrQuote, UserOrderId>
where
    I: Mon<D>,
    BaseOrQuote: Currency<I, D>,
    UserOrderId: UserOrderIdT,
{
    /// The limit order was partially filled.
    PartiallyFilled(LimitOrder<I, D, BaseOrQuote, UserOrderId, Pending<I, D, BaseOrQuote>>),
    /// The limit order was fully filled.
    FullyFilled(LimitOrder<I, D, BaseOrQuote, UserOrderId, Filled<I, D, BaseOrQuote>>),
}
