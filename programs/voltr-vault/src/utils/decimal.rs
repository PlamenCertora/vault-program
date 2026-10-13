use crate::error::VaultError;
use crate::utils::SafeMath;
use anchor_lang::{solana_program::msg, Result};
use fixed::traits::{FromFixed, ToFixed};
pub use fixed::types::U80F48 as Decimal;
use std::panic::Location;

#[allow(clippy::assign_op_pattern)]
#[allow(clippy::reversed_empty_ranges)]
mod uint_types {
    use uint::construct_uint;
    construct_uint! {
               pub struct U256(4);
    }
    construct_uint! {
               pub struct U128(2);
    }
}

pub use uint_types::{U128, U256};

pub const FRACTION_ONE_SCALED: u128 = Decimal::ONE.to_bits();

pub fn pow_decimal(decimal: Decimal, power: u32) -> Option<Decimal> {
    if power == 0 {
        return Some(Decimal::ONE);
    }

    let mut x = decimal;
    let mut y = Decimal::ONE;
    let mut n = power;

    while n > 1 {
        if n % 2 == 1 {
            y = x.checked_mul(y)?;
        }
        x = x.checked_mul(x)?;
        n /= 2;
    }

    x.checked_mul(y)
}

pub trait DecimalExtra {
    fn to_percent<Dst: FromFixed>(&self) -> Option<Dst>;
    fn to_bps<Dst: FromFixed>(&self) -> Option<Dst>;
    fn from_percent<Src: ToFixed>(percent: Src) -> Self;
    fn from_bps<Src: ToFixed>(bps: Src) -> Self;
    fn checked_pow(&self, power: u32) -> Option<Self>
    where
        Self: std::marker::Sized;

    fn mul_int_ratio(&self, numerator: impl Into<u128>, denominator: impl Into<u128>) -> Self;
    fn full_mul_int_ratio(&self, numerator: impl Into<U256>, denominator: impl Into<U256>) -> Self;

    fn to_floor<Dst: FromFixed>(&self) -> Dst;
    fn to_ceil<Dst: FromFixed>(&self) -> Dst;
    fn to_round<Dst: FromFixed>(&self) -> Dst;

    fn to_decimal_bits(&self) -> u128;
    fn from_decimal_bits(decimal: u128) -> Self;

    fn to_display(&self) -> DecimalDisplay;
}

impl DecimalExtra for Decimal {
    #[inline]
    fn to_percent<Dst: FromFixed>(&self) -> Option<Dst> {
        (self * 100).round().checked_to_num()
    }

    #[inline]
    fn to_bps<Dst: FromFixed>(&self) -> Option<Dst> {
        (self * 10_000).round().checked_to_num()
    }

    #[inline]
    fn from_percent<Src: ToFixed>(percent: Src) -> Self {
        let percent = Decimal::from_num(percent);
        percent / 100
    }

    #[inline]
    fn from_bps<Src: ToFixed>(bps: Src) -> Self {
        let bps = Decimal::from_num(bps);
        bps / 10_000
    }

    #[inline]
    fn checked_pow(&self, power: u32) -> Option<Self>
    where
        Self: std::marker::Sized,
    {
        pow_decimal(*self, power)
    }

    #[inline]
    fn mul_int_ratio(&self, numerator: impl Into<u128>, denominator: impl Into<u128>) -> Self {
        let numerator = numerator.into();
        let denominator = denominator.into();
        (*self * numerator) / denominator
    }

    #[inline]
    fn full_mul_int_ratio(&self, numerator: impl Into<U256>, denominator: impl Into<U256>) -> Self {
        let numerator = numerator.into();
        let denominator = denominator.into();

        if denominator.is_zero() {
            panic!("Division by zero in full_mul_int_ratio");
        }

        let big_sf = U256::from(self.to_bits());
        let big_sf_res = (big_sf * numerator) / denominator;
        let sf_res: u128 = big_sf_res
            .try_into()
            .expect("Denominator is not big enough, the result doesn't fit in a Fraction.");
        Decimal::from_bits(sf_res)
    }

    #[inline]
    fn to_floor<Dst: FromFixed>(&self) -> Dst {
        self.floor().to_num()
    }

    #[inline]
    fn to_ceil<Dst: FromFixed>(&self) -> Dst {
        self.ceil().to_num()
    }

    #[inline]
    fn to_round<Dst: FromFixed>(&self) -> Dst {
        self.round().to_num()
    }

    #[inline]
    fn to_decimal_bits(&self) -> u128 {
        self.to_bits()
    }

    #[inline]
    fn from_decimal_bits(decimal: u128) -> Self {
        Decimal::from_bits(decimal)
    }

    #[inline]
    fn to_display(&self) -> DecimalDisplay {
        DecimalDisplay(self)
    }
}

pub fn to_decimal_bits<Src: ToFixed>(src: Src) -> u128 {
    Decimal::from_num(src).to_bits()
}

impl SafeMath for Decimal {
    #[track_caller]
    #[inline(always)]
    fn safe_add(self, rhs: Self) -> Result<Self> {
        self.checked_add(rhs).ok_or_else(|| {
            let caller = Location::caller();
            msg!("Math error thrown at {}:{}", caller.file(), caller.line());
            VaultError::MathOverflow.into()
        })
    }

    #[track_caller]
    #[inline(always)]
    fn safe_sub(self, rhs: Self) -> Result<Self> {
        self.checked_sub(rhs).ok_or_else(|| {
            let caller = Location::caller();
            msg!("Math error thrown at {}:{}", caller.file(), caller.line());
            VaultError::MathOverflow.into()
        })
    }

    #[track_caller]
    #[inline(always)]
    fn safe_mul(self, rhs: Self) -> Result<Self> {
        self.checked_mul(rhs).ok_or_else(|| {
            let caller = Location::caller();
            msg!("Math error thrown at {}:{}", caller.file(), caller.line());
            VaultError::MathOverflow.into()
        })
    }

    #[track_caller]
    #[inline(always)]
    fn safe_div(self, rhs: Self) -> Result<Self> {
        self.checked_div(rhs).ok_or_else(|| {
            let caller = Location::caller();
            msg!("Math error thrown at {}:{}", caller.file(), caller.line());
            VaultError::MathOverflow.into()
        })
    }

    #[track_caller]
    #[inline(always)]
    fn safe_div_ceil(self, rhs: Self) -> Result<Self> {
        // First perform checked division; then, if successful, round up.
        self.checked_div(rhs)
            .map(|result| result.ceil())
            .ok_or_else(|| {
                let caller = Location::caller();
                msg!("Math error thrown at {}:{}", caller.file(), caller.line());
                VaultError::MathOverflow.into()
            })
    }
}

pub struct DecimalDisplay<'a>(&'a Decimal);

use std::fmt::Display;

impl Display for DecimalDisplay<'_> {
    fn fmt(&self, formater: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let decimal = self.0.to_bits();

        const ROUND_COMP: u128 = (1 << Decimal::FRAC_NBITS) / (10_000 * 2);
        let decimal = decimal + ROUND_COMP;

        let i = decimal >> Decimal::FRAC_NBITS;

        const FRAC_MASK: u128 = (1 << Decimal::FRAC_NBITS) - 1;
        let f_p = u64::try_from(decimal & FRAC_MASK).unwrap();
        let f_p = ((f_p >> 30) * 10_000) >> 30;
        write!(formater, "{i}.{f_p:0>4}")
    }
}

impl std::fmt::Debug for DecimalDisplay<'_> {
    fn fmt(&self, formater: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formater, "{}", self)
    }
}
