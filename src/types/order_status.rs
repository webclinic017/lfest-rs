use getset::{CopyGetters, Getters};

use super::{order_meta::ExchangeOrderMeta, Currency, Mon, QuoteCurrency, TimestampNs};

/// A new order has not been received by the exchange and has thus some pieces of information not available.
/// This also means the various filters (e.g `PriceFilter` and `QuantityFilter`) have not been checked.
#[derive(Debug, Clone, Eq, PartialEq, derive_more::Display)]
pub struct NewOrder;

/// The order is pending execution, but it already has additional information filled in by the exchange.
/// - `I`: The numeric data type of currencies.
/// - `D`: The constant decimal precision of the currencies.
/// - `BaseOrQuote`: Either `BaseCurrency` or `QuoteCurrency` depending on the futures type.
#[derive(Debug, Clone, Eq, PartialEq, Getters)]
pub struct Pending<I, const D: u8, BaseOrQuote>
where
    I: Mon<D>,
    BaseOrQuote: Currency<I, D>,
{
    /// The now filled in order metadata.
    #[getset(get = "pub")]
    meta: ExchangeOrderMeta,

    /// Information about the filled quantity.
    #[getset(get = "pub")]
    pub(crate) filled_quantity: FilledQuantity<I, D, BaseOrQuote>,
}

impl<I, const D: u8, BaseOrQuote> Pending<I, D, BaseOrQuote>
where
    I: Mon<D>,
    BaseOrQuote: Currency<I, D>,
{
    /// Create a new instance of `Self`
    pub(crate) fn new(meta: ExchangeOrderMeta) -> Self {
        Self {
            meta,
            filled_quantity: FilledQuantity::Unfilled,
        }
    }
}

impl<I, const D: u8, BaseOrQuote> std::fmt::Display for Pending<I, D, BaseOrQuote>
where
    I: Mon<D>,
    BaseOrQuote: Currency<I, D>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Pending ( meta: {}, filled_quantity: {})",
            self.meta, self.filled_quantity
        )
    }
}

/// Contains the filled order quantity along with the average fill price.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FilledQuantity<I, const D: u8, BaseOrQuote>
where
    I: Mon<D>,
    BaseOrQuote: Currency<I, D>,
{
    /// All the order quantity has yet to be filled.
    Unfilled,
    /// Some (or all) of the order quantity has been filled.
    Filled {
        /// Cumulative Amount that was filled.
        cumulative_qty: BaseOrQuote,

        /// The average price it was filled at.
        avg_price: QuoteCurrency<I, D>,
    },
}

impl<I, const D: u8, BaseOrQuote> std::fmt::Display for FilledQuantity<I, D, BaseOrQuote>
where
    I: Mon<D>,
    BaseOrQuote: Currency<I, D>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FilledQuantity::Unfilled => write!(f, "Unfilled"),
            FilledQuantity::Filled {
                cumulative_qty,
                avg_price,
            } => write!(
                f,
                "Filled( cumulative_qty: {}, avg_price: {})",
                cumulative_qty, avg_price
            ),
        }
    }
}

/// The order has been fully filled.
/// The executed order quantity is stored elsewhere.
#[derive(Debug, Clone, Eq, PartialEq, Getters, CopyGetters)]
pub struct Filled<I, const D: u8, BaseOrQuote>
where
    I: Mon<D>,
    BaseOrQuote: Currency<I, D>,
{
    /// The now filled in order metadata.
    #[getset(get = "pub")]
    meta: ExchangeOrderMeta,

    /// The timestamp in nanoseconds when the order was executed by the exchange.
    /// Will be the simulated time, not actual computer (OS) time.
    #[getset(get_copy = "pub")]
    ts_ns_executed: TimestampNs,

    /// The average price the order has been filled at.
    #[getset(get_copy = "pub")]
    avg_fill_price: QuoteCurrency<I, D>,

    /// The total filled quantity.
    #[getset(get_copy = "pub")]
    filled_qty: BaseOrQuote,
}

impl<I, const D: u8, BaseOrQuote> Filled<I, D, BaseOrQuote>
where
    I: Mon<D>,
    BaseOrQuote: Currency<I, D>,
{
    /// Create a new instance of `Self`.
    pub(crate) fn new(
        meta: ExchangeOrderMeta,
        ts_ns_executed: TimestampNs,
        avg_fill_price: QuoteCurrency<I, D>,
        filled_qty: BaseOrQuote,
    ) -> Self {
        Self {
            meta,
            ts_ns_executed,
            avg_fill_price,
            filled_qty,
        }
    }
}

impl<I, const D: u8, BaseOrQuote> std::fmt::Display for Filled<I, D, BaseOrQuote>
where
    I: Mon<D>,
    BaseOrQuote: Currency<I, D>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Filled( meta: {}, ts_ns_executed: {}, avg_fill_price: {}, filled_qty: {})",
            self.meta, self.ts_ns_executed, self.avg_fill_price, self.filled_qty
        )
    }
}
