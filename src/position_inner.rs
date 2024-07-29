use std::cmp::Ordering;

use fpdec::{Dec, Decimal};
use getset::{CopyGetters, Getters};
use tracing::{debug, trace};

use crate::{
    prelude::{
        Transaction, TransactionAccounting, TREASURY_ACCOUNT, USER_POSITION_MARGIN_ACCOUNT,
        USER_WALLET_ACCOUNT,
    },
    quote,
    types::{Currency, MarginCurrency, QuoteCurrency},
};

/// Describes the position information of the account.
/// It assumes isolated margining mechanism, because the margin is directly associated with the position.
#[derive(Debug, Clone, Default, Eq, PartialEq, Getters, CopyGetters)]
pub struct PositionInner<Q>
where
    Q: Currency,
    Q::PairedCurrency: MarginCurrency,
{
    /// The number of futures contracts making up the position.
    #[getset(get_copy = "pub")]
    quantity: Q,

    /// The entry price of the position.
    #[getset(get_copy = "pub")]
    entry_price: QuoteCurrency,
}

impl<Q> PositionInner<Q>
where
    Q: Currency,
    Q::PairedCurrency: MarginCurrency,
{
    /// Create a new instance.
    ///
    /// # Panics:
    /// if `quantity` or `entry_price` are invalid.
    pub fn new<T>(
        quantity: Q,
        entry_price: QuoteCurrency,
        accounting: &mut T,
        init_margin_req: Decimal,
    ) -> Self
    where
        T: TransactionAccounting<Q::PairedCurrency>,
    {
        trace!("new position: qty {quantity} @ {entry_price}");
        assert!(quantity > Q::new_zero());
        assert!(entry_price > quote!(0));

        let margin = quantity.convert(entry_price) * init_margin_req;
        let transaction =
            Transaction::new(USER_POSITION_MARGIN_ACCOUNT, USER_WALLET_ACCOUNT, margin);
        accounting
            .create_margin_transfer(transaction)
            .expect("margin transfer for opening a new position works.");

        Self {
            quantity,
            entry_price,
        }
    }

    /// Return the positions unrealized profit and loss
    /// denoted in QUOTE when using linear futures,
    /// denoted in BASE when using inverse futures
    pub fn unrealized_pnl(&self, mark_to_market_price: QuoteCurrency) -> Q::PairedCurrency {
        Q::PairedCurrency::pnl(self.entry_price, mark_to_market_price, self.quantity)
    }

    /// Add contracts to the position.
    pub(crate) fn increase_contracts<T>(
        &mut self,
        qty: Q,
        entry_price: QuoteCurrency,
        accounting: &mut T,
        init_margin_req: Decimal,
    ) where
        T: TransactionAccounting<Q::PairedCurrency>,
    {
        debug!(
            "increase_contracts: qty: {qty} @ {entry_price}; self: {:?}",
            self
        );
        assert!(qty > Q::new_zero());
        assert!(entry_price > quote!(0));

        self.entry_price = self.new_avg_entry_price(qty, entry_price);
        self.quantity += qty;

        let margin = qty.convert(entry_price) * init_margin_req;
        let transaction =
            Transaction::new(USER_POSITION_MARGIN_ACCOUNT, USER_WALLET_ACCOUNT, margin);
        accounting
            .create_margin_transfer(transaction)
            .expect("is an internal call and must work");
    }

    /// Decrease the position.
    pub(crate) fn decrease_contracts<T>(
        &mut self,
        qty: Q,
        liquidation_price: QuoteCurrency,
        accounting: &mut T,
        init_margin_req: Decimal,
        direction_multiplier: Decimal,
    ) where
        T: TransactionAccounting<Q::PairedCurrency>,
    {
        debug!(
            "decrease_contracts: qty: {qty} @ {liquidation_price}; self: {:?}",
            self
        );
        assert!(qty > Q::new_zero());
        assert!(qty <= self.quantity);
        debug_assert!(direction_multiplier == Dec!(1) || direction_multiplier == Dec!(-1));

        self.quantity -= qty;
        debug_assert!(self.quantity >= Q::new_zero());

        let pnl = Q::PairedCurrency::pnl(
            self.entry_price,
            liquidation_price,
            qty * direction_multiplier,
        );
        match pnl.cmp(&Q::PairedCurrency::new_zero()) {
            Ordering::Greater => {
                let transaction = Transaction::new(USER_WALLET_ACCOUNT, TREASURY_ACCOUNT, pnl);
                accounting
                    .create_margin_transfer(transaction)
                    .expect("margin transfer must work");
            }
            Ordering::Less => {
                let transaction =
                    Transaction::new(TREASURY_ACCOUNT, USER_WALLET_ACCOUNT, pnl.abs());
                accounting
                    .create_margin_transfer(transaction)
                    .expect("margin transfer must work");
            }
            Ordering::Equal => {}
        }
        let margin_to_free = qty.convert(self.entry_price) * init_margin_req;
        let transaction = Transaction::new(
            USER_WALLET_ACCOUNT,
            USER_POSITION_MARGIN_ACCOUNT,
            margin_to_free,
        );
        accounting
            .create_margin_transfer(transaction)
            .expect("margin transfer must work");
    }

    /// Compute the new entry price of the position when some quantity is added at a specifiy `entry_price`.
    fn new_avg_entry_price(&self, added_qty: Q, entry_price: QuoteCurrency) -> QuoteCurrency {
        debug_assert!(added_qty > Q::new_zero());
        debug_assert!(entry_price > quote!(0));

        let new_qty = self.quantity + added_qty;
        QuoteCurrency::new(
            ((*self.quantity.as_ref() * *self.entry_price.as_ref())
                + (*added_qty.as_ref() * *entry_price.as_ref()))
                / *new_qty.as_ref(),
        )
    }

    #[cfg(test)]
    pub(crate) fn from_parts(quantity: Q, entry_price: QuoteCurrency) -> Self {
        Self {
            quantity,
            entry_price,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{base, prelude::InMemoryTransactionAccounting};

    #[test]
    fn position_inner_new_avg_entry_price() {
        let pos = PositionInner {
            quantity: base!(0.1),
            entry_price: quote!(100),
        };
        assert_eq!(pos.new_avg_entry_price(base!(0.1), quote!(50)), quote!(75));
        assert_eq!(pos.new_avg_entry_price(base!(0.1), quote!(90)), quote!(95));
        assert_eq!(
            pos.new_avg_entry_price(base!(0.1), quote!(150)),
            quote!(125)
        );
        assert_eq!(
            pos.new_avg_entry_price(base!(0.3), quote!(200)),
            quote!(175)
        );
    }

    #[test_case::test_matrix([1, 2, 5])]
    fn position_inner_new(leverage: u32) {
        let mut ta = InMemoryTransactionAccounting::new(quote!(1000));
        let init_margin_req = Dec!(1) / Decimal::from(leverage);
        let pos = PositionInner::new(base!(0.5), quote!(100), &mut ta, init_margin_req);
        assert_eq!(
            pos,
            PositionInner {
                quantity: base!(0.5),
                entry_price: quote!(100)
            }
        );
        assert_eq!(
            ta.margin_balance_of(USER_POSITION_MARGIN_ACCOUNT).unwrap(),
            quote!(50) * init_margin_req
        );
        assert_eq!(
            ta.margin_balance_of(USER_WALLET_ACCOUNT).unwrap(),
            quote!(1000) - quote!(50) * init_margin_req
        );
    }

    #[test_case::test_matrix([1, 2, 5])]
    fn position_inner_increase_contracts(leverage: u32) {
        let mut ta = InMemoryTransactionAccounting::new(quote!(1000));
        let init_margin_req = Dec!(1) / Decimal::from(leverage);
        let mut pos = PositionInner::new(base!(0.5), quote!(100), &mut ta, init_margin_req);
        pos.increase_contracts(base!(0.5), quote!(150), &mut ta, init_margin_req);
        assert_eq!(
            pos,
            PositionInner {
                quantity: base!(1),
                entry_price: quote!(125),
            }
        );
        assert_eq!(
            ta.margin_balance_of(USER_POSITION_MARGIN_ACCOUNT).unwrap(),
            quote!(125) * init_margin_req
        );
        assert_eq!(
            ta.margin_balance_of(USER_WALLET_ACCOUNT).unwrap(),
            quote!(1000) - quote!(125) * init_margin_req
        );
    }

    #[test_case::test_matrix([1, 2, 5])]
    fn position_inner_decrease_contracts(leverage: u32) {
        let mut ta = InMemoryTransactionAccounting::new(quote!(1000));
        let init_margin_req = Dec!(1) / Decimal::from(leverage);
        let mut pos = PositionInner::new(base!(0.5), quote!(100), &mut ta, init_margin_req);
        pos.decrease_contracts(base!(0.5), quote!(100), &mut ta, init_margin_req, Dec!(1));
        assert_eq!(
            pos,
            PositionInner {
                quantity: base!(0),
                entry_price: quote!(100),
            }
        );
        assert_eq!(
            ta.margin_balance_of(USER_POSITION_MARGIN_ACCOUNT).unwrap(),
            quote!(0)
        );
        assert_eq!(
            ta.margin_balance_of(USER_WALLET_ACCOUNT).unwrap(),
            quote!(1000)
        );
    }
}
