//! Typed quantities with tolerance algebra and assertions.
//!
//! Every physical fact is a `Quantity<D>` where `D` is a phantom dimension
//! marker. Tolerance propagates through comparisons; `assert_ge`/`assert_le`
//! evaluate at worst-case bounds so a violated assertion fails CI (§3.1).
//!
//! ## no_std
//!
//! Unconditionally `#![no_std]`. Embedded, WASM and host all use the same type.

#![no_std]
#![deny(unsafe_code)]
#![warn(missing_docs)]

#[cfg(test)]
extern crate std;

use core::marker::PhantomData;

// ── Dimension markers ────────────────────────────────────────────────────────

/// Marker for a length dimension (metres).
pub struct Length;
/// Marker for a frequency dimension (hertz).
pub struct Frequency;
/// Marker for a time dimension (seconds).
pub struct Time;
/// Marker for a mass dimension (grams).
pub struct Mass;
/// Marker for a power dimension (watts).
pub struct Power;
/// Marker for a voltage dimension (volts).
pub struct Voltage;
/// Marker for a current dimension (amperes).
pub struct Current;
/// Marker for a temperature dimension (celsius).
pub struct Temperature;
/// Marker for a dimensionless ratio.
pub struct Dimensionless;

// ── Tolerance ────────────────────────────────────────────────────────────────

/// The tolerance on a [`Quantity`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Tolerance {
    /// No tolerance declared — the value is exact.
    None,
    /// Symmetric absolute tolerance: value ± abs in the same unit.
    Absolute(f64),
    /// Symmetric percentage tolerance: value ± pct% of the nominal.
    Percentage(f64),
}

// ── Quantity ─────────────────────────────────────────────────────────────────

/// A typed quantity with an optional tolerance.
///
/// `D` is a phantom dimension marker (`Length`, `Frequency`, …). The type
/// system prevents adding a length to a frequency; operations within one
/// dimension are natural.
///
/// # Example
///
/// ```rust
/// use fiducial_quantity::{Quantity, Length};
///
/// let board_height = Quantity::<Length>::with_abs(1.6, 0.16); // 1.6 mm ± 0.16
/// let clearance    = Quantity::<Length>::exact(0.5);
///
/// // Worst-case check: board_height.min() >= clearance.max()
/// board_height.assert_ge(&clearance).expect("board must clear the housing");
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quantity<D> {
    /// Nominal value.
    pub value: f64,
    /// Tolerance on the nominal.
    pub tolerance: Tolerance,
    _dim: PhantomData<D>,
}

impl<D> Quantity<D> {
    /// Exact value — no tolerance.
    #[inline]
    pub const fn exact(value: f64) -> Self {
        Self {
            value,
            tolerance: Tolerance::None,
            _dim: PhantomData,
        }
    }

    /// Nominal value with a symmetric absolute tolerance.
    #[inline]
    pub const fn with_abs(value: f64, abs_tol: f64) -> Self {
        Self {
            value,
            tolerance: Tolerance::Absolute(abs_tol),
            _dim: PhantomData,
        }
    }

    /// Nominal value with a symmetric percentage tolerance (0.0–100.0).
    #[inline]
    pub const fn with_pct(value: f64, pct: f64) -> Self {
        Self {
            value,
            tolerance: Tolerance::Percentage(pct),
            _dim: PhantomData,
        }
    }

    /// Worst-case minimum (nominal minus tolerance).
    #[inline]
    pub fn min(&self) -> f64 {
        match self.tolerance {
            Tolerance::None => self.value,
            Tolerance::Absolute(t) => self.value - t,
            Tolerance::Percentage(p) => self.value * (1.0 - p / 100.0),
        }
    }

    /// Worst-case maximum (nominal plus tolerance).
    #[inline]
    pub fn max(&self) -> f64 {
        match self.tolerance {
            Tolerance::None => self.value,
            Tolerance::Absolute(t) => self.value + t,
            Tolerance::Percentage(p) => self.value * (1.0 + p / 100.0),
        }
    }

    /// Assert `self >= rhs` at worst case (self.min() >= rhs.max()).
    ///
    /// Returns [`AssertionError`] when the constraint is violated.
    pub fn assert_ge(&self, rhs: &Quantity<D>) -> Result<(), AssertionError> {
        let lhs_min = self.min();
        let rhs_max = rhs.max();
        if lhs_min >= rhs_max {
            Ok(())
        } else {
            Err(AssertionError {
                lhs: lhs_min,
                rhs: rhs_max,
            })
        }
    }

    /// Assert `self <= rhs` at worst case (self.max() <= rhs.min()).
    pub fn assert_le(&self, rhs: &Quantity<D>) -> Result<(), AssertionError> {
        let lhs_max = self.max();
        let rhs_min = rhs.min();
        if lhs_max <= rhs_min {
            Ok(())
        } else {
            Err(AssertionError {
                lhs: lhs_max,
                rhs: rhs_min,
            })
        }
    }
}

// ── AssertionError ────────────────────────────────────────────────────────────

/// Returned when a quantity assertion is violated.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AssertionError {
    /// The worst-case value of the left-hand side that failed.
    pub lhs: f64,
    /// The worst-case value of the right-hand side that failed.
    pub rhs: f64,
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_quantity_min_max_equal() {
        let q = Quantity::<Length>::exact(1.6);
        assert_eq!(q.min(), 1.6);
        assert_eq!(q.max(), 1.6);
    }

    #[test]
    fn absolute_tolerance() {
        let q = Quantity::<Length>::with_abs(1.6, 0.1);
        assert!((q.min() - 1.5).abs() < 1e-10);
        assert!((q.max() - 1.7).abs() < 1e-10);
    }

    #[test]
    fn percentage_tolerance() {
        let q = Quantity::<Frequency>::with_pct(868.0, 10.0);
        assert!((q.min() - 781.2).abs() < 1e-10);
        assert!((q.max() - 954.8).abs() < 1e-10);
    }

    #[test]
    fn assert_ge_passes_when_clear() {
        let board = Quantity::<Length>::with_abs(1.6, 0.1); // 1.5–1.7
        let clearance = Quantity::<Length>::exact(1.4); // exactly 1.4
        assert!(board.assert_ge(&clearance).is_ok());
    }

    #[test]
    fn assert_ge_fails_when_overlapping() {
        let board = Quantity::<Length>::with_abs(1.6, 0.2); // 1.4–1.8
        let clearance = Quantity::<Length>::with_abs(1.5, 0.1); // 1.4–1.6
                                                                // board.min() = 1.4, clearance.max() = 1.6 → 1.4 < 1.6 → fail
        assert!(board.assert_ge(&clearance).is_err());
    }

    #[test]
    fn assert_le_passes() {
        let actual = Quantity::<Mass>::with_abs(50.0, 5.0); // 45–55
        let budget = Quantity::<Mass>::exact(60.0);
        assert!(actual.assert_le(&budget).is_ok());
    }

    #[test]
    fn assert_le_fails() {
        let actual = Quantity::<Mass>::with_abs(58.0, 5.0); // 53–63
        let budget = Quantity::<Mass>::exact(60.0);
        // actual.max() = 63 > budget.min() = 60 → fail
        assert!(actual.assert_le(&budget).is_err());
    }
}
