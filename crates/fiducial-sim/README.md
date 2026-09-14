# fiducial-sim

**One simulation, running native and in the browser.**

[![crates.io](https://img.shields.io/crates/v/fiducial-sim.svg)](https://crates.io/crates/fiducial-sim)
[![docs.rs](https://docs.rs/fiducial-sim/badge.svg)](https://docs.rs/fiducial-sim)

Part of the [Fiducial](https://github.com/AleksaZCodes/fiducial) platform —
*declare each fact once, derive every artifact from it.*

---

## What this is

Numerical simulation built on one trait. `OdeSystem<N>` declares a system of `N`
ordinary differential equations; `rk4_step` and `Integrator` advance it with
4th-order Runge-Kutta.

The point is the **single API across two very different runtimes**. On the host,
`run_batch` fans parameter sweeps across cores with rayon. On `wasm32` there are
no threads, so the same call runs sequentially — and the signature does not
change, so the calling code does not either. A simulation written for the design
desk runs unmodified inside the marketing page.

## Install

```sh
cargo add fiducial-sim
```

For WASM, opt out of the host defaults:

```toml
fiducial-sim = { version = "0.1", default-features = false }
```

## Use

```rust
use fiducial_sim::{Integrator, OdeSystem};

// Exponential decay: dy/dt = -k·y
struct Decay { k: f64 }

impl OdeSystem<1> for Decay {
    fn derivatives(&self, _t: f64, y: &[f64; 1]) -> [f64; 1] {
        [-self.k * y[0]]
    }
}

let integrator = Integrator::new(Decay { k: 1.0 }, 0.01);
let trace = integrator.run(0.0, 1.0, [1.0]);

// y(1) = e⁻¹ ≈ 0.3679, and RK4 at dt=0.01 is well inside 1e-6 of it.
let (_, final_state) = trace.last().unwrap();
assert!((final_state[0] - std::f64::consts::E.recip()).abs() < 1e-6);
```

## Included models

`ThermalModel` — a single-node RC thermal simulation with `steady_state()`,
`time_constant()` and `simulate()`. `ThermalSnapshot` round-trips through serde,
so a trace computed on the host renders in the browser without a second model.

## Features

| Feature | Default | Effect |
|---|---|---|
| `std` | ✅ | Host build. |
| `parallel` | ✅ | `run_batch` fans out with rayon. |
| *(none)* | | `wasm32` — single-threaded, identical API. |

## License

MIT
