//! 4th-order Runge-Kutta integrator.
//!
//! `rk4_step` is a single step; `Integrator` drives the loop and collects
//! snapshots.  Both are generic over `OdeSystem<N>`.
//!
//! # Parallel batches
//!
//! When the `parallel` feature is enabled, `Integrator::run_batch` integrates
//! multiple independent initial conditions in parallel with rayon.  The API is
//! identical on WASM; the implementation falls back to a sequential iterator.

use crate::OdeSystem;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

// ── Single step ───────────────────────────────────────────────────────────────

/// Advance state `y` by one RK4 step of size `h` at time `t`.
///
/// Returns `(t + h, new_y)`.
pub fn rk4_step<S: OdeSystem<N>, const N: usize>(
    system: &S,
    t: f64,
    y: &[f64; N],
    h: f64,
) -> (f64, [f64; N]) {
    let k1 = system.derivatives(t, y);
    let y2 = add_scaled(y, &k1, h / 2.0);
    let k2 = system.derivatives(t + h / 2.0, &y2);
    let y3 = add_scaled(y, &k2, h / 2.0);
    let k3 = system.derivatives(t + h / 2.0, &y3);
    let y4 = add_scaled(y, &k3, h);
    let k4 = system.derivatives(t + h, &y4);

    let mut out = [0.0f64; N];
    for i in 0..N {
        out[i] = y[i] + (h / 6.0) * (k1[i] + 2.0 * k2[i] + 2.0 * k3[i] + k4[i]);
    }
    (t + h, out)
}

fn add_scaled<const N: usize>(y: &[f64; N], k: &[f64; N], s: f64) -> [f64; N] {
    let mut out = [0.0f64; N];
    for i in 0..N {
        out[i] = y[i] + s * k[i];
    }
    out
}

// ── Integrator ────────────────────────────────────────────────────────────────

/// Drives an ODE integration loop and collects time-series snapshots.
pub struct Integrator<S: OdeSystem<N>, const N: usize> {
    pub system: S,
    pub dt: f64,
}

impl<S: OdeSystem<N>, const N: usize> Integrator<S, N> {
    pub fn new(system: S, dt: f64) -> Self {
        Self { system, dt }
    }

    /// Integrate from `t0` to `t_end` starting at `y0`.
    ///
    /// Returns a `Vec` of `(t, state)` snapshots, one per step including the
    /// initial condition.
    pub fn run(&self, t0: f64, t_end: f64, y0: [f64; N]) -> Vec<(f64, [f64; N])> {
        let steps = ((t_end - t0) / self.dt).ceil() as usize;
        let mut snapshots = Vec::with_capacity(steps + 1);
        let mut t = t0;
        let mut y = y0;
        snapshots.push((t, y));
        while t < t_end - self.dt * 0.5 {
            let (tn, yn) = rk4_step(&self.system, t, &y, self.dt);
            t = tn;
            y = yn;
            snapshots.push((t, y));
        }
        snapshots
    }

    /// Run independent initial conditions in parallel (rayon when available,
    /// sequential otherwise — the API is identical on WASM).
    pub fn run_batch(
        &self,
        t0: f64,
        t_end: f64,
        initial_conditions: Vec<[f64; N]>,
    ) -> Vec<Vec<(f64, [f64; N])>>
    where
        S: Sync,
        [f64; N]: Send,
    {
        #[cfg(feature = "parallel")]
        {
            initial_conditions
                .into_par_iter()
                .map(|y0| self.run(t0, t_end, y0))
                .collect()
        }
        #[cfg(not(feature = "parallel"))]
        {
            initial_conditions
                .into_iter()
                .map(|y0| self.run(t0, t_end, y0))
                .collect()
        }
    }
}
