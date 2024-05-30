use fpdec::{Dec, Decimal};
use getset::{CopyGetters, Getters};
use tracing::trace;

use crate::{
    prelude::{
        Transaction, TransactionAccounting, TREASURY_ACCOUNT, USER_POSITION_MARGIN_ACCOUNT,
        USER_WALLET_ACCOUNT,
    },
    quote,
    types::{Currency, MarginCurrency, QuoteCurrency, Side},
    utils::assert_user_wallet_balance,
};

/// A futures position can be one of three variants.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Position<Q>
where
    Q: Currency,
    Q::PairedCurrency: MarginCurrency,
{
    /// No position present.
    Neutral,
    /// A position in the long direction.
    Long(PositionInner<Q>),
    /// A position in the short direction.
    Short(PositionInner<Q>),
}

impl<Q> Default for Position<Q>
where
    Q: Currency,
    Q::PairedCurrency: MarginCurrency,
{
    fn default() -> Self {
        Position::Neutral
    }
}

impl<Q> Position<Q>
where
    Q: Currency,
    Q::PairedCurrency: MarginCurrency,
{
    /// Return the positions unrealized profit and loss.
    pub fn unrealized_pnl(&self, bid: QuoteCurrency, ask: QuoteCurrency) -> Q::PairedCurrency {
        match self {
            Position::Neutral => Q::PairedCurrency::new_zero(),
            Position::Long(inner) => inner.unrealized_pnl(bid, Dec!(1)),
            Position::Short(inner) => inner.unrealized_pnl(ask, Dec!(-1)).into_negative(),
        }
    }

    /// Change a position while doing proper accounting and balance transfers.
    pub(crate) fn change_position<T>(
        &mut self,
        filled_qty: Q,
        fill_price: QuoteCurrency,
        side: Side,
        transaction_accounting: &mut T,
        init_margin_req: Decimal,
    ) where
        T: TransactionAccounting<Q::PairedCurrency>,
    {
        trace!("old position: {}", self);
        match self {
            Position::Neutral => match side {
                Side::Buy => {
                    *self = Position::Long(PositionInner::new(
                        filled_qty,
                        fill_price,
                        transaction_accounting,
                        init_margin_req,
                    ))
                }
                Side::Sell => {
                    *self = Position::Short(PositionInner::new(
                        filled_qty,
                        fill_price,
                        transaction_accounting,
                        init_margin_req,
                    ))
                }
            },
            Position::Long(inner) => match side {
                Side::Buy => {
                    inner.increase_contracts(
                        filled_qty,
                        fill_price,
                        transaction_accounting,
                        init_margin_req,
                    );
                }
                Side::Sell => {
                    if filled_qty > inner.quantity {
                        let new_short_qty = filled_qty - inner.quantity;
                        inner.decrease_contracts(
                            inner.quantity,
                            fill_price,
                            transaction_accounting,
                            init_margin_req,
                            Dec!(1),
                        );
                        *self = Position::Short(PositionInner::new(
                            new_short_qty,
                            fill_price,
                            transaction_accounting,
                            init_margin_req,
                        ));
                    } else if filled_qty == inner.quantity {
                        inner.decrease_contracts(
                            filled_qty,
                            fill_price,
                            transaction_accounting,
                            init_margin_req,
                            Dec!(1),
                        );
                        *self = Position::Neutral;
                    } else {
                        inner.decrease_contracts(
                            filled_qty,
                            fill_price,
                            transaction_accounting,
                            init_margin_req,
                            Dec!(1),
                        );
                    }
                }
            },
            Position::Short(inner) => match side {
                Side::Buy => {
                    if filled_qty > inner.quantity {
                        let new_long_qty = filled_qty - inner.quantity;
                        inner.decrease_contracts(
                            inner.quantity,
                            fill_price,
                            transaction_accounting,
                            init_margin_req,
                            Dec!(-1),
                        );
                        *self = Position::Long(PositionInner::new(
                            new_long_qty,
                            fill_price,
                            transaction_accounting,
                            init_margin_req,
                        ));
                    } else if filled_qty == inner.quantity {
                        inner.decrease_contracts(
                            filled_qty,
                            fill_price,
                            transaction_accounting,
                            init_margin_req,
                            Dec!(-1),
                        );
                        *self = Position::Neutral;
                    } else {
                        inner.decrease_contracts(
                            filled_qty,
                            fill_price,
                            transaction_accounting,
                            init_margin_req,
                            Dec!(-1),
                        );
                    }
                }
                Side::Sell => {
                    inner.increase_contracts(
                        filled_qty,
                        fill_price,
                        transaction_accounting,
                        init_margin_req,
                    );
                }
            },
        };
        assert_user_wallet_balance(transaction_accounting);
        trace!("new position: {}", self);
    }
}

/// Describes the position information of the account.
/// It assumes isolated margining mechanism, because the margin is directly associated with the position.
#[derive(Debug, Clone, Default, Eq, PartialEq, Getters, CopyGetters)]
pub struct PositionInner<Q>
where
    Q: Currency,
    Q::PairedCurrency: MarginCurrency,
{
    /// The number of futures contracts making up the position.
    /// Denoted in the currency in which the size is valued.
    /// e.g.: XBTUSD has a contract size of 1 USD, so `M::PairedCurrency` is USD.
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
        qty: Q,
        entry_price: QuoteCurrency,
        accounting: &mut T,
        init_margin_req: Decimal,
    ) -> Self
    where
        T: TransactionAccounting<Q::PairedCurrency>,
    {
        trace!("new position: qty {qty} @ {entry_price}");
        assert!(qty > Q::new_zero());
        assert!(entry_price > quote!(0));

        let margin = qty.convert(entry_price) * init_margin_req;
        let transaction =
            Transaction::new(USER_POSITION_MARGIN_ACCOUNT, USER_WALLET_ACCOUNT, margin);
        accounting
            .create_margin_transfer(transaction)
            .expect("margin transfer for opening a new position works.");

        Self {
            quantity: qty,
            entry_price,
        }
    }

    /// Return the positions unrealized profit and loss
    /// denoted in QUOTE when using linear futures,
    /// denoted in BASE when using inverse futures
    pub fn unrealized_pnl(
        &self,
        mark_to_market_price: QuoteCurrency,
        direction_multiplier: Decimal,
    ) -> Q::PairedCurrency {
        debug_assert!(
            direction_multiplier == Dec!(1) || direction_multiplier == Dec!(-1),
            "Multiplier must be one of those."
        );
        Q::PairedCurrency::pnl(
            self.entry_price,
            mark_to_market_price,
            self.quantity * direction_multiplier,
        )
    }

    /// The total position value including unrealized profit and loss.
    /// Denoted in the margin `Currency`.
    pub fn value(
        &self,
        mark_to_market_price: QuoteCurrency,
        direction_multiplier: Decimal,
    ) -> Q::PairedCurrency {
        self.quantity.convert(self.entry_price)
            + self.unrealized_pnl(mark_to_market_price, direction_multiplier)
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
        trace!("increase_contracts: qty: {qty} @ {entry_price}");
        assert!(qty > Q::new_zero());
        assert!(entry_price > quote!(0));

        self.quantity += qty;
        self.entry_price = self.new_avg_entry_price(qty, entry_price);

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
        trace!("decrease_contracts: qty: {qty} @ {liquidation_price}");
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
        if pnl > Q::PairedCurrency::new_zero() {
            let transaction = Transaction::new(USER_WALLET_ACCOUNT, TREASURY_ACCOUNT, pnl);
            accounting
                .create_margin_transfer(transaction)
                .expect("margin transfer must work");
        } else if pnl < Q::PairedCurrency::new_zero() {
            let transaction = Transaction::new(TREASURY_ACCOUNT, USER_WALLET_ACCOUNT, pnl.abs());
            accounting
                .create_margin_transfer(transaction)
                .expect("margin transfer must work");
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
}

impl<Q> std::fmt::Display for Position<Q>
where
    Q: Currency,
    Q::PairedCurrency: MarginCurrency,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Position::Neutral => write!(f, "Neutral"),
            Position::Long(inner) => write!(f, "Long {} @ {}", inner.quantity, inner.entry_price),
            Position::Short(inner) => write!(f, "Short {} @ {}", inner.quantity, inner.entry_price),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base;

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
}
