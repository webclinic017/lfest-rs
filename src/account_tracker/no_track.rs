use std::fmt::Display;

use crate::{
    account_tracker::AccountTracker,
    prelude::{MarketState, Mon, QuoteCurrency, Side, UserBalances},
    types::{LimitOrder, MarginCurrency, MarketOrder, NewOrder, UserOrderIdT},
};

/// Performs no tracking of account performance
#[derive(Default, Debug, Clone)]
pub struct NoAccountTracker;

impl<I, const D: u8, BaseOrQuote, UserOrderId> AccountTracker<I, D, BaseOrQuote, UserOrderId>
    for NoAccountTracker
where
    I: Mon<D>,
    BaseOrQuote: MarginCurrency<I, D>,
    UserOrderId: UserOrderIdT,
{
    #[inline(always)]
    fn update(&mut self, _market_state: &MarketState<I, D>) {}

    #[inline(always)]
    fn sample_user_balances(
        &mut self,
        _user_balances: &UserBalances<I, D, BaseOrQuote>,
        _mid_price: QuoteCurrency<I, D>,
    ) {
    }

    #[inline(always)]
    fn log_limit_order_submission(
        &mut self,
        _limit_order: &LimitOrder<I, D, BaseOrQuote::PairedCurrency, UserOrderId, NewOrder>,
    ) {
    }

    #[inline(always)]
    fn log_limit_order_cancellation(&mut self) {}

    #[inline(always)]
    fn log_limit_order_fill(
        &mut self,
        _fully_filled: bool,
        _filled_qty: BaseOrQuote::PairedCurrency,
    ) {
    }

    #[inline(always)]
    fn log_market_order_submission(
        &mut self,
        _market_order: &MarketOrder<I, D, BaseOrQuote::PairedCurrency, UserOrderId, NewOrder>,
    ) {
    }

    #[inline(always)]
    fn log_market_order_fill(&mut self) {}

    #[inline(always)]
    fn log_trade(
        &mut self,
        _side: Side,
        _price: QuoteCurrency<I, D>,
        _quantity: BaseOrQuote::PairedCurrency,
    ) {
    }
}

impl Display for NoAccountTracker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "")
    }
}
