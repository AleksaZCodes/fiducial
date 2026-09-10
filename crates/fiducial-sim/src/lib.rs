//! `fiducial-sim` — numerical simulation for the Fiducial platform.
//!
//! # Shape
//!
//! Simulation is just a pipeline: a declared initial state, a step function,
//! and a sequence of snapshots.  The same pipeline runs natively (with rayon
//! parallelism when the `parallel` feature is enabled) and in WASM
//! (single-threaded, `--no-default-features`).
//!
//! # The one real constraint
//!
//! `rayon` does not parallelise in WASM by default — it needs
//! `wasm-bindgen-rayon` plus COOP/COEP headers on the serving origin.
//! Single-threaded WASM is the platform default; a product can opt into
//! parallel WASM by enabling those headers and the `wasm-bindgen-rayon`
//! feature.  The `parallel` feature gate is the declaration; WASM targets
//! disable it.
//!
//! # Provided simulations
//!
//! | Module | What it models |
//! |---|---|
//! | [`rk4`] | General-purpose 4th-order Runge-Kutta ODE integrator |
//! | [`thermal`] | Single-node RC thermal model (junction temperature over time) |
//! | [`ode`] | Trait definition for ODE systems |

#![cfg_attr(not(feature = "std"), no_std)]

pub mod ode;
pub mod rk4;
pub mod thermal;

// Re-export the most-used surface.
pub use ode::OdeSystem;
pub use rk4::{rk4_step, Integrator};
pub use thermal::{ThermalModel, ThermalSnapshot};
