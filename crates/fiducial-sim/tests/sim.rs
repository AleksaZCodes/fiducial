//! `fiducial-sim` integration tests.
//!
//! "Done when: simulation runs native and in WASM" — Phase 17.
//!
//! These tests run on the host.  The WASM path is exercised by the spine CI
//! matrix (`cargo check --target wasm32-unknown-unknown --no-default-features`).
//! A separate `wasm-pack test` could assert numerical equivalence in Node;
//! that is deferred until a product ships a WASM sim page.

use fiducial_sim::*;

// ── ODE: Simple harmonic oscillator ──────────────────────────────────────────
//
// Exact solution: x(t) = A cos(ωt), dx/dt = -Aω sin(ωt)
// Period: T = 2π/ω

struct Oscillator {
    omega: f64,
}

impl OdeSystem<2> for Oscillator {
    fn derivatives(&self, _t: f64, y: &[f64; 2]) -> [f64; 2] {
        // [x, v] → [v, -ω²x]
        [y[1], -self.omega * self.omega * y[0]]
    }
}

#[test]
fn oscillator_returns_to_start_after_one_period() {
    // ω=1 → T=2π. Integrate for exactly one period.
    let omega = 1.0f64;
    let period = 2.0 * std::f64::consts::PI;
    let integrator: Integrator<Oscillator, 2> = Integrator::new(Oscillator { omega }, 0.001);
    let snaps = integrator.run(0.0, period, [1.0, 0.0]);
    let last = snaps.last().unwrap();
    // Position should be back near 1.0; velocity near 0.0.
    assert!(
        (last.1[0] - 1.0).abs() < 1e-4,
        "x after one period: {} (expected ≈ 1.0)",
        last.1[0]
    );
    assert!(
        last.1[1].abs() < 5e-4,
        "v after one period: {} (expected ≈ 0.0)",
        last.1[1]
    );
}

#[test]
fn rk4_step_single_step_matches_euler_direction() {
    // The first RK4 step must move in the correct direction.
    let osc = Oscillator { omega: 1.0 };
    let y0 = [1.0f64, 0.0]; // x=1, v=0 → dx/dt=0, dv/dt=-1
    let (_t1, y1) = rk4_step(&osc, 0.0, &y0, 0.1);
    // x should decrease (v starts at 0, becomes negative), v should become negative
    assert!(
        y1[0] < y0[0] || (y1[0] - y0[0]).abs() < 1e-3,
        "x moved wrong"
    );
    assert!(y1[1] < 0.0, "v should be negative after one step");
}

#[test]
fn batch_produces_same_result_as_sequential() {
    // run_batch over one IC must equal run for the same IC.
    let integrator: Integrator<Oscillator, 2> = Integrator::new(Oscillator { omega: 1.0 }, 0.01);
    let y0 = [1.0f64, 0.0];
    let sequential = integrator.run(0.0, 1.0, y0);
    let mut batch = integrator.run_batch(0.0, 1.0, vec![y0]);
    let parallel_first = batch.pop().unwrap();
    assert_eq!(sequential.len(), parallel_first.len(), "step counts differ");
    for ((t_s, y_s), (t_p, y_p)) in sequential.iter().zip(parallel_first.iter()) {
        assert!((t_s - t_p).abs() < 1e-12, "time mismatch");
        assert!((y_s[0] - y_p[0]).abs() < 1e-12, "state mismatch");
    }
}

// ── Thermal model ─────────────────────────────────────────────────────────────

#[test]
fn thermal_steady_state_matches_analytical() {
    // R_th=10 °C/W, C_th=1 J/°C, P=2 W, T_amb=25 °C
    // Steady state: 25 + 2 × 10 = 45 °C
    // Time constant: 10 × 1 = 10 s
    // Simulate for 5τ: should be within 1% of steady state.
    let model = ThermalModel {
        r_th: 10.0,
        c_th: 1.0,
        t_amb: 25.0,
        power: 2.0,
    };
    let expected_ss = model.steady_state(); // 45.0
    let tau = model.time_constant(); // 10.0
    let snaps = model.simulate(5.0 * tau, 0.01, model.t_amb);
    let last_temp = snaps.last().unwrap().temperature;
    assert!(
        (last_temp - expected_ss).abs() / expected_ss < 0.01,
        "after 5τ, temperature is {last_temp:.3} °C; expected ≈ {expected_ss:.3} °C"
    );
}

#[test]
fn thermal_starts_at_initial_temperature() {
    let model = ThermalModel {
        r_th: 5.0,
        c_th: 2.0,
        t_amb: 20.0,
        power: 1.0,
    };
    let snaps = model.simulate(10.0, 0.1, 20.0);
    assert_eq!(snaps[0].t, 0.0, "first snapshot must be at t=0");
    assert_eq!(
        snaps[0].temperature, 20.0,
        "first snapshot must equal initial temperature"
    );
}

#[test]
fn thermal_temperature_is_monotone_when_starting_below_steady_state() {
    // Starting at ambient, power is positive → temperature only rises.
    let model = ThermalModel {
        r_th: 8.0,
        c_th: 3.0,
        t_amb: 25.0,
        power: 3.0,
    };
    let snaps = model.simulate(60.0, 0.05, model.t_amb);
    let mut prev = f64::NEG_INFINITY;
    for s in &snaps {
        assert!(
            s.temperature >= prev - 1e-9,
            "temperature decreased at t={}: {} < {}",
            s.t,
            s.temperature,
            prev
        );
        prev = s.temperature;
    }
}

#[test]
fn thermal_snapshot_serialises_and_deserialises() {
    let snap = ThermalSnapshot {
        t: 1.5,
        temperature: 42.0,
    };
    let json = serde_json::to_string(&snap).expect("serialise");
    let back: ThermalSnapshot = serde_json::from_str(&json).expect("deserialise");
    assert!((back.t - snap.t).abs() < 1e-12);
    assert!((back.temperature - snap.temperature).abs() < 1e-12);
}
