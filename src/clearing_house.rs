//! A clearinghouse clears and settles all trades and collects margin

use fpdec::Decimal;

use crate::{
    prelude::{Account, AccountTracker},
    types::{Currency, Fee, MarginCurrency, QuoteCurrency},
};

/// A clearing house acts as an intermediary in futures transactions.
/// It guarantees the performance of the parties to each transaction.
/// The main task of the clearing house is to keep track of all the transactions
/// that take place, so that at can calculate the net position of each account.
///
/// If in total the transactions have lost money,
/// the account is required to provide variation margin to the exchange clearing
/// house. If there has been a gain on the transactions, the account receives
/// variation margin from the clearing house.
#[derive(Debug, Clone)]
pub struct ClearingHouse<A, M> {
    /// Keeps track of all trades of the `Account`.
    account_tracker: A,
    _margin_curr: std::marker::PhantomData<M>,
}

impl<A, M> ClearingHouse<A, M>
where
    A: AccountTracker<M>,
    M: Currency + MarginCurrency,
{
    /// Create a new instance with a user account
    pub(crate) fn new(account_tracker: A) -> Self {
        Self {
            account_tracker,
            _margin_curr: Default::default(),
        }
    }

    /// The margin accounts are adjusted to reflect investors gain or loss.
    pub(crate) fn mark_to_market(&mut self, mark_price: QuoteCurrency) {
        // let position_value = self.user_account.position().size().convert(mark_price);

        todo!()
    }

    /// The funding period for perpetual futures has ended.
    /// Funding = `mark_value` * `funding_rate`.
    /// `mark_value` is denoted in the margin currency.
    /// If the funding rate is positive, longs pay shorts.
    /// Else its the otherway around.
    /// TODO: not used but may be in the future.
    pub(crate) fn settle_funding_period(&mut self, mark_value: M, funding_rate: Decimal) {
        todo!()
    }

    /// Settlement referes to the actual transfer of funds or assets between the buyer and seller to fulfill the trade.
    /// As the `ClearingHouse` is the central counterparty to every trade,
    /// it is the buyer of every sell order,
    /// and the seller of every buy order.
    ///
    /// # Arguments:
    /// `quantity`: The number of contract traded, where a negative number indicates a sell.
    /// `fill_price`: The execution price of the trade
    /// `req_margin`: The additional required margin as computed by the `RiskEngine`.
    ///
    pub(crate) fn settle_filled_order(
        &self,
        account: &mut Account<M>,
        quantity: M::PairedCurrency,
        fill_price: QuoteCurrency,
        req_margin: M,
        fee: Fee,
    ) {
        if quantity > M::PairedCurrency::new_zero() {
            self.settle_buy_order(account, quantity, fill_price, req_margin, fee);
        } else {
            self.settle_sell_order(account, quantity.abs(), fill_price, req_margin, fee);
        }
    }

    fn settle_buy_order(
        &self,
        account: &mut Account<M>,
        quantity: M::PairedCurrency,
        fill_price: QuoteCurrency,
        req_margin: M,
        fee: Fee,
    ) {
        if account.position.size() >= M::PairedCurrency::new_zero() {
            account
                .position
                .increase_long(quantity, fill_price, req_margin);
            let fee = quantity.convert(fill_price) * fee;
            account.wallet_balance -= fee;
        } else {
            // decrease short position (and maybe open long)
            todo!()
        }
    }

    fn settle_sell_order(
        &self,
        account: &mut Account<M>,
        quantity: M::PairedCurrency,
        fill_price: QuoteCurrency,
        req_margin: M,
        fee: Fee,
    ) {
        if account.position.size() > M::PairedCurrency::new_zero() {
            // TODO: decrease long position (maybe open short)
            todo!()
        } else {
            account
                .position
                .increase_short(quantity, fill_price, req_margin);
            let fee = quantity.convert(fill_price) * fee;
            account.wallet_balance -= fee;
        }
    }
}