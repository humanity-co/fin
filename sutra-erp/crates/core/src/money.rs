//! Money value object — stored as paise (i64).
//!
//! All monetary values in SutraERP are represented in paise
//! (1/100 rupee) to avoid floating-point precision issues.
//! The API layer is responsible for display formatting.

use derive_more::{Add, Constructor, Neg, Sub};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::{AddAssign, SubAssign};

/// Money value object stored as paise (1/100 of ₹).
///
/// # Examples
///
/// ```rust
/// use sutra_core::Money;
///
/// // ₹1,000.00 = 100,000 paise
/// let fee = Money::from_paise(100_000);
/// let tax = Money::from_rupees(180.00);
/// let total = fee + tax;
/// assert_eq!(total, Money::from_paise(118_000));
/// ```
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Add,
    Sub,
    Neg,
    Constructor,
    Serialize,
    Deserialize,
)]
#[serde(transparent)]
pub struct Money {
    /// Amount in paise. Always a non-negative i64.
    paise: i64,
}

impl Money {
    /// Zero amount.
    pub const ZERO: Money = Money { paise: 0 };

    /// Create Money from paise.
    pub const fn from_paise(paise: i64) -> Self {
        Money { paise }
    }

    /// Create Money from rupees (f64 — use with caution in tests/demos only).
    ///
    /// # Panics
    ///
    /// Panics if the value is negative or NaN/Infinite.
    pub fn from_rupees(rupees: f64) -> Self {
        assert!(
            rupees >= 0.0 && rupees.is_finite(),
            "rupees must be non-negative and finite"
        );
        let paise = (rupees * 100.0).round() as i64;
        Money { paise }
    }

    /// Create Money from a decimal string like "1234.56".
    ///
    /// # Errors
    ///
    /// Returns an error if the string cannot be parsed or is negative.
    pub fn from_str(s: &str) -> Result<Self, String> {
        let parsed: rust_decimal::Decimal =
            s.parse().map_err(|e| format!("invalid decimal: {e}"))?;
        if parsed.is_sign_negative() {
            return Err("money amount cannot be negative".into());
        }
        let paise = (parsed * rust_decimal::Decimal::from(100)).round();
        Ok(Money {
            paise: paise
                .try_into()
                .map_err(|_| "money value out of range".to_string())?,
        })
    }

    /// Return the amount in paise.
    pub const fn as_paise(self) -> i64 {
        self.paise
    }

    /// Return the amount as a Decimal (rupees).
    pub fn as_rupees_decimal(self) -> rust_decimal::Decimal {
        rust_decimal::Decimal::from(self.paise) / rust_decimal::Decimal::from(100)
    }

    /// Check if this is zero.
    pub const fn is_zero(self) -> bool {
        self.paise == 0
    }
}

impl AddAssign for Money {
    fn add_assign(&mut self, rhs: Self) {
        self.paise += rhs.paise;
    }
}

impl SubAssign for Money {
    fn sub_assign(&mut self, rhs: Self) {
        self.paise -= rhs.paise;
    }
}

impl fmt::Display for Money {
    /// Display in ₹ format — "₹1,23,456.78" (Indian numbering).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let negative = self.paise < 0;
        let abs_paise = self.paise.abs();

        let rupees = abs_paise / 100;
        let fractional = abs_paise % 100;

        // Indian grouping: rightmost 3 digits, then groups of 2
        let rupees_str = format_rupees_indian(rupees);

        if negative {
            write!(f, "-₹{}.{:02}", rupees_str, fractional)
        } else {
            write!(f, "₹{}.{:02}", rupees_str, fractional)
        }
    }
}

/// Format an integer rupee amount with Indian grouping:
/// rightmost 3 digits as a group, then groups of 2.
fn format_rupees_indian(rupees: i64) -> String {
    let s = rupees.to_string();
    let len = s.len();

    if len <= 3 {
        return s;
    }

    let last_three = &s[len - 3..];
    let remaining = &s[..len - 3];

    // Format the remaining portion in groups of 2 from right
    let mut grouped = String::new();
    let rlen = remaining.len();
    let mut i = rlen;

    while i > 0 {
        let start = if i >= 2 { i - 2 } else { 0 };
        if !grouped.is_empty() {
            grouped.insert(0, ',');
        }
        grouped.insert_str(0, &remaining[start..i]);
        i = start;
    }

    format!("{},{}", grouped, last_three)
}

impl From<Money> for i64 {
    fn from(m: Money) -> Self {
        m.paise
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_indian_format() {
        assert_eq!(format!("{}", Money::from_paise(0)), "₹0.00");
        assert_eq!(format!("{}", Money::from_paise(1)), "₹0.01");
        assert_eq!(format!("{}", Money::from_paise(100)), "₹1.00");
        assert_eq!(format!("{}", Money::from_paise(100_000)), "₹1,000.00");
        assert_eq!(format!("{}", Money::from_paise(1_234_567)), "₹12,345.67");
        assert_eq!(format!("{}", Money::from_paise(12_345_678_900)), "₹12,34,56,789.00");
    }

    #[test]
    fn test_from_rupees() {
        assert_eq!(Money::from_rupees(100.00), Money::from_paise(10_000));
        assert_eq!(Money::from_rupees(99.99), Money::from_paise(9_999));
    }

    #[test]
    fn test_from_str() {
        assert_eq!(Money::from_str("100.00").unwrap(), Money::from_paise(10_000));
        assert!(Money::from_str("-1.00").is_err());
        assert!(Money::from_str("abc").is_err());
    }

    #[test]
    fn test_arithmetic() {
        let a = Money::from_paise(100);
        let b = Money::from_paise(200);
        assert_eq!(a + b, Money::from_paise(300));
        assert_eq!(b - a, Money::from_paise(100));
        assert_eq!(-a, Money::from_paise(-100));
    }
}
