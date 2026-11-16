# fiducial-rt

**Real-time setup for a Linux control loop, from the declaration.**

[![crates.io](https://img.shields.io/crates/v/fiducial-rt.svg)](https://crates.io/crates/fiducial-rt)
[![docs.rs](https://docs.rs/fiducial-rt/badge.svg)](https://docs.rs/fiducial-rt)

Part of the [Fiducial](https://github.com/AleksaZCodes/fiducial) platform —
*declare each fact once, derive every artifact from it.*

---

## What this is

A host node in `topology.toml` can declare how its loop runs:

```toml
[[node]]
id = "ctl"
kind = "host"
# …
[node.rt]
priority = 80        # SCHED_FIFO, 1 to 99
cpu = 2              # optional
lock_memory = true   # default
prefault_stack_kib = 512
```

`fid derive` checks the values and writes them as `NODE_CTL_RT_*` constants.
This crate applies them: memory locked and stack touched first, then the CPU,
then `SCHED_FIFO` last, so a step that fails leaves the thread at normal
priority. It returns what the kernel reads back, and a refusal names the step
and the errno.

## Use

```rust
use fiducial_rt::{apply, Config};

let cfg = Config::new(80).cpu(2);
match apply(&cfg) {
    Ok(done) => assert_eq!(done.priority, 80),
    Err(e) => eprintln!("running at normal priority: {e}"),
}
```

## What it does not do

It does not make the kernel real-time. `SCHED_FIFO` bounds when the loop is
picked to run; worst-case latency also depends on the kernel being `PREEMPT_RT`
(`fiducial_rt::is_preempt_rt()`), and has to be measured on it. On other
operating systems `apply` returns `Error::Unsupported`.

## License

MIT
