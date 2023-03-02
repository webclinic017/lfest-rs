use std::{
    convert::TryFrom,
    ops::{Add, Div, Mul, Sub},
};

use derive_more::{Add, AddAssign, Display, Div, From, Into, Mul, Sub, SubAssign};
use fpdec::Decimal;

use crate::types::{BaseCurrency, Currency, Fee};

/// Allows the quick construction of `QuoteCurrency`
#[macro_export]
macro_rules! quote {
    ( $a:expr ) => {{
        QuoteCurrency::from_f64($a)
    }};
}

/// The markets QUOTE currency, e.g.: BTCUSD -> USD is the quote currency
#[derive(
    Default,
    Debug,
    Clone,
    Copy,
    PartialEq,
    PartialOrd,
    Add,
    Sub,
    Mul,
    Div,
    AddAssign,
    SubAssign,
    Display,
    Into,
    From,
)]
#[mul(forward)]
#[div(forward)]
pub struct QuoteCurrency(Decimal);

impl Currency for QuoteCurrency {
    type PairedCurrency = BaseCurrency;

    #[inline(always)]
    fn new(val: Decimal) -> Self {
        Self(val)
    }

    #[inline]
    fn from_f64(val: f64) -> Self {
        Self(Decimal::try_from(val).expect("Unable to create Decimal from f64"))
    }

    #[inline(always)]
    fn inner(self) -> Decimal {
        self.0
    }

    #[inline(always)]
    fn new_zero() -> Self {
        Self::new(Decimal::ZERO)
    }

    #[inline(always)]
    fn is_zero(&self) -> bool {
        self.0.eq(&Decimal::ZERO)
    }

    #[inline(always)]
    fn is_finite(&self) -> bool {
        // self.0.is_finite()
        todo!()
    }

    #[inline(always)]
    fn abs(self) -> Self {
        Self(self.0.abs())
    }

    #[inline(always)]
    fn fee_portion(&self, fee: Fee) -> Self {
        Self(self.0 * fee.inner())
    }

    #[inline(always)]
    fn convert(&self, rate: QuoteCurrency) -> Self::PairedCurrency {
        BaseCurrency::new(self.0 / rate.0)
    }

    #[inline(always)]
    fn into_negative(self) -> Self {
        Self(-self.0)
    }
}

/// ### Arithmetic with `Rational` on the right hand side
impl Add<Decimal> for QuoteCurrency {
    type Output = Self;

    fn add(self, rhs: Decimal) -> Self::Output {
        Self(self.0 + rhs)
    }
}

impl Sub<Decimal> for QuoteCurrency {
    type Output = Self;

    fn sub(self, rhs: Decimal) -> Self::Output {
        Self(self.0 - rhs)
    }
}

impl Mul<Decimal> for QuoteCurrency {
    type Output = Self;

    fn mul(self, rhs: Decimal) -> Self::Output {
        Self(self.0 * rhs)
    }
}

impl Div<Decimal> for QuoteCurrency {
    type Output = Self;

    fn div(self, rhs: Decimal) -> Self::Output {
        Self(self.0 / rhs)
    }
}

/// ### Arithmetic with `&Self` on the right hand side
impl<'a> Add<&'a Self> for QuoteCurrency {
    type Output = Self;

    fn add(self, rhs: &'a Self) -> Self::Output {
        Self(self.0 + &rhs.0)
    }
}

impl<'a> Sub<&'a Self> for QuoteCurrency {
    type Output = Self;

    fn sub(self, rhs: &'a Self) -> Self::Output {
        Self(self.0 - &rhs.0)
    }
}

impl<'a> Mul<&'a Self> for QuoteCurrency {
    type Output = Self;

    fn mul(self, rhs: &'a Self) -> Self::Output {
        Self(self.0 * &rhs.0)
    }
}

impl<'a> Div<&'a Self> for QuoteCurrency {
    type Output = Self;

    fn div(self, rhs: &'a Self) -> Self::Output {
        Self(self.0 / &rhs.0)
    }
}

/// ### Arithmetic assignment with `&Self` on the right hand side
impl<'a> std::ops::AddAssign<&'a Self> for QuoteCurrency {
    fn add_assign(&mut self, rhs: &'a Self) {
        self.0 = &self.0 + &rhs.0;
    }
}

impl<'a> std::ops::SubAssign<&'a Self> for QuoteCurrency {
    fn sub_assign(&mut self, rhs: &'a Self) {
        self.0 = &self.0 - &rhs.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quote_display() {
        println!("{}", quote!(0.5));
    }
}