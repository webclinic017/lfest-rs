//! This module contains order filtering related code

use getset::CopyGetters;

use crate::prelude::{ConfigError, CurrencyMarker, Mon, OrderError, QuoteCurrency};

/// The `SizeFilter` defines the quantity rules that each order needs to follow
/// The generic currency `S` is always the `PairedCurrency` of the margin
/// currency
#[derive(Debug, Clone, CopyGetters)]
pub struct QuantityFilter<I, const DB: u8, const DQ: u8, BaseOrQuote>
where
    I: Mon<DB> + Mon<DQ>,
    BaseOrQuote: CurrencyMarker<I, DB, DQ>,
{
    /// Defines the optional minimum `quantity` of any order
    #[getset(get_copy = "pub")]
    min_quantity: Option<BaseOrQuote>,

    /// Defines the optional maximum `quantity` of any order
    #[getset(get_copy = "pub")]
    max_quantity: Option<BaseOrQuote>,

    /// Defines the intervals that a `quantity` can be increased / decreased by.
    /// For the filter to pass,
    /// (quantity - min_qty) % tick_size == 0
    #[getset(get_copy = "pub")]
    tick_size: BaseOrQuote,

    _quote: std::marker::PhantomData<QuoteCurrency<I, DB, DQ>>,
}

impl<I, const DB: u8, const DQ: u8, BaseOrQuote> Default for QuantityFilter<I, DB, DQ, BaseOrQuote>
where
    I: Mon<DB> + Mon<DQ>,
    BaseOrQuote: CurrencyMarker<I, DB, DQ>,
{
    fn default() -> Self {
        Self {
            min_quantity: None,
            max_quantity: None,
            tick_size: BaseOrQuote::one(),
            _quote: std::marker::PhantomData,
        }
    }
}

impl<I, const DB: u8, const DQ: u8, BaseOrQuote> QuantityFilter<I, DB, DQ, BaseOrQuote>
where
    I: Mon<DB> + Mon<DQ>,
    BaseOrQuote: CurrencyMarker<I, DB, DQ>,
{
    /// Create a new instance of the QuantityFilter.
    /// Make sure the `min_quantity` is a multiple of `tick_size`.
    pub fn new(
        min_quantity: Option<BaseOrQuote>,
        max_quantity: Option<BaseOrQuote>,
        tick_size: BaseOrQuote,
    ) -> Result<Self, ConfigError> {
        if let Some(min_qty) = min_quantity {
            if (min_qty % tick_size) != BaseOrQuote::zero() {
                return Err(ConfigError::InvalidMinQuantity);
            }
        }
        if tick_size == BaseOrQuote::zero() {
            return Err(ConfigError::InvalidTickSize);
        }

        Ok(Self {
            min_quantity,
            max_quantity,
            tick_size,
            _quote: std::marker::PhantomData,
        })
    }

    pub(crate) fn validate_order_quantity(
        &self,
        quantity: BaseOrQuote,
    ) -> std::result::Result<(), OrderError<I, DB, DQ>> {
        if quantity == BaseOrQuote::zero() {
            return Err(OrderError::QuantityTooLow);
        }

        if let Some(max_qty) = self.max_quantity {
            if quantity > max_qty {
                return Err(OrderError::QuantityTooHigh);
            }
        }

        let min_qty = if let Some(min_qty) = self.min_quantity {
            if quantity < min_qty {
                return Err(OrderError::QuantityTooLow);
            }
            min_qty
        } else {
            BaseOrQuote::zero()
        };

        if ((quantity - min_qty) % self.tick_size) != BaseOrQuote::zero() {
            return Err(OrderError::InvalidQuantityStepSize);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use num_traits::{One, Zero};

    use super::*;
    use crate::prelude::*;

    #[test]
    fn quantity_filter() {
        let filter = QuantityFilter {
            min_quantity: Some(QuoteCurrency::<i32, 4, 2>::new(10, 0)),
            max_quantity: Some(QuoteCurrency::new(1000, 0)),
            tick_size: QuoteCurrency::one(),
            _quote: std::marker::PhantomData,
        };

        assert_eq!(
            filter.validate_order_quantity(QuoteCurrency::zero()),
            Err(OrderError::QuantityTooLow)
        );

        filter
            .validate_order_quantity(QuoteCurrency::new(50, 0))
            .unwrap();

        assert_eq!(
            filter.validate_order_quantity(QuoteCurrency::new(5, 0)),
            Err(OrderError::QuantityTooLow)
        );

        assert_eq!(
            filter.validate_order_quantity(QuoteCurrency::new(5000, 0)),
            Err(OrderError::QuantityTooHigh)
        );

        assert_eq!(
            filter.validate_order_quantity(QuoteCurrency::new(505, 1)),
            Err(OrderError::InvalidQuantityStepSize)
        );
    }

    #[test]
    fn quantity_filter_2() {
        let filter = QuantityFilter {
            min_quantity: None,
            max_quantity: None,
            tick_size: QuoteCurrency::one(),
            _quote: std::marker::PhantomData::<QuoteCurrency<i32, 4, 2>>::default(),
        };
        assert_eq!(
            filter.validate_order_quantity(QuoteCurrency::zero()),
            Err(OrderError::QuantityTooLow)
        );
        assert_eq!(
            filter.validate_order_quantity(QuoteCurrency::new(5, 1)),
            Err(OrderError::InvalidQuantityStepSize)
        );
    }
}
