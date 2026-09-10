//! ODE system trait — the declaration every simulation implements.
//!
//! A simulation is a system `dy/dt = f(t, y)`.  Implementors declare:
//! - `N`: the number of state variables (const generic)
//! - `derivatives`: the right-hand side function
//!
//! The integrators in this crate are generic over `OdeSystem`.

/// A system of ordinary differential equations with `N` state variables.
///
/// # Example
///
/// ```rust
/// use fiducial_sim::OdeSystem;
///
/// /// Simple harmonic oscillator: d²x/dt² = -ω²x
/// /// State: [x, dx/dt]
/// struct Oscillator { omega: f64 }
///
/// impl OdeSystem<2> for Oscillator {
///     fn derivatives(&self, _t: f64, y: &[f64; 2]) -> [f64; 2] {
///         [y[1], -self.omega * self.omega * y[0]]
///     }
/// }
/// ```
pub trait OdeSystem<const N: usize> {
    /// Compute `dy/dt` at time `t` given state `y`.
    fn derivatives(&self, t: f64, y: &[f64; N]) -> [f64; N];
}
