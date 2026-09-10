//! Single-node RC thermal model.
//!
//! Models the junction temperature of a component over time given a power
//! dissipation profile and an ambient temperature.  This is the smallest
//! useful simulation on the platform: it has a physical meaning, a known
//! analytical solution (exponential decay to steady state), and a real
//! "done when" criterion — the simulated temperature at t→∞ must match
//! `T_amb + P * R_th` to within numerical tolerance.
//!
//! # Model
//!
//! ```text
//!   dT/dt = (P - (T - T_amb) / R_th) / C_th
//! ```
//!
//! | Parameter | Meaning |
//! |---|---|
//! | `T` | Junction temperature (°C) |
//! | `P` | Power dissipation (W) |
//! | `R_th` | Thermal resistance (°C/W) |
//! | `C_th` | Thermal capacitance (J/°C) |
//! | `T_amb` | Ambient temperature (°C) |

use crate::{Integrator, OdeSystem};
use serde::{Deserialize, Serialize};

/// Parameters for the RC thermal model.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ThermalModel {
    /// Thermal resistance (°C/W).
    pub r_th: f64,
    /// Thermal capacitance (J/°C).
    pub c_th: f64,
    /// Ambient temperature (°C).
    pub t_amb: f64,
    /// Power dissipation (W).
    pub power: f64,
}

/// A single point in the simulated temperature time series.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ThermalSnapshot {
    /// Time (s).
    pub t: f64,
    /// Junction temperature (°C).
    pub temperature: f64,
}

impl ThermalModel {
    /// Steady-state junction temperature: `T_amb + P * R_th`.
    pub fn steady_state(&self) -> f64 {
        self.t_amb + self.power * self.r_th
    }

    /// Thermal time constant τ = R_th × C_th.
    pub fn time_constant(&self) -> f64 {
        self.r_th * self.c_th
    }

    /// Simulate from `t=0` to `t_end` with time step `dt`, starting at
    /// `t_initial`.
    pub fn simulate(&self, t_end: f64, dt: f64, t_initial: f64) -> Vec<ThermalSnapshot> {
        let integrator: Integrator<ThermalModel, 1> = Integrator::new(*self, dt);
        integrator
            .run(0.0, t_end, [t_initial])
            .into_iter()
            .map(|(t, y)| ThermalSnapshot {
                t,
                temperature: y[0],
            })
            .collect()
    }
}

// State: [T]
impl OdeSystem<1> for ThermalModel {
    fn derivatives(&self, _t: f64, y: &[f64; 1]) -> [f64; 1] {
        let temp = y[0];
        let dt = (self.power - (temp - self.t_amb) / self.r_th) / self.c_th;
        [dt]
    }
}
