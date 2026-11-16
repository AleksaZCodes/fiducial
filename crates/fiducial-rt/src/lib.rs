//! Real-time setup for a Linux control loop.
//!
//! A node declared in `topology.toml` with an `[node.rt]` block names how its
//! loop must run: a `SCHED_FIFO` priority, whether memory is locked, which CPU it
//! is pinned to, how much stack is touched up front. `fid derive` turns those
//! into `NODE_<ID>_RT_*` constants; this crate is what applies them.
//!
//! ```
//! use fiducial_rt::Config;
//!
//! // The values the declaration derived: priority 80, CPU 2.
//! let cfg = Config::new(80).cpu(2);
//! assert_eq!(cfg.validate(), Ok(()));
//! assert!(Config::new(0).validate().is_err());
//! ```
//!
//! Applying it ([`apply`]) is a request to the kernel, and the kernel may say no:
//! `SCHED_FIFO` needs `CAP_SYS_NICE` (or an `RLIMIT_RTPRIO`), and
//! `mlockall` is bounded by `RLIMIT_MEMLOCK`. So `apply` returns what **was**
//! done, read back from the kernel, and a refusal is an [`Error`] naming the step
//! and the errno rather than a loop that quietly runs at normal priority.
//!
//! Real-time scheduling bounds how soon the loop runs, not how fast the kernel
//! path under it is: for hard latency bounds the kernel must also be `PREEMPT_RT`
//! ([`is_preempt_rt`]). That this crate runs is not evidence that it does.

#![deny(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]

use core::fmt;

/// The largest CPU number a Linux affinity mask (`cpu_set_t`) holds.
pub const MAX_CPUS: u32 = 1024;
/// The most stack that may be prefaulted, in KiB: a thread's default is 8 MiB.
pub const MAX_PREFAULT_STACK_KIB: u32 = 8192;

/// How the loop must run. Mirrors the `rt` block in `topology.toml`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Config {
    /// `SCHED_FIFO` priority, 1 to 99. Higher runs first.
    pub priority: u8,
    /// Pin the loop to this CPU.
    pub cpu: Option<u32>,
    /// `mlockall`, so a page fault cannot stall the loop.
    pub lock_memory: bool,
    /// Touch this much stack up front, in KiB.
    pub prefault_stack_kib: u32,
}

impl Config {
    /// A priority, with the declaration's defaults: lock memory, 512 KiB of stack.
    pub const fn new(priority: u8) -> Self {
        Self {
            priority,
            cpu: None,
            lock_memory: true,
            prefault_stack_kib: 512,
        }
    }
    /// Pin to a CPU.
    pub const fn cpu(mut self, cpu: u32) -> Self {
        self.cpu = Some(cpu);
        self
    }
    /// Do not lock memory.
    pub const fn unlocked(mut self) -> Self {
        self.lock_memory = false;
        self
    }
    /// Prefault this much stack, in KiB.
    pub const fn prefault_stack_kib(mut self, kib: u32) -> Self {
        self.prefault_stack_kib = kib;
        self
    }

    /// Whether the values are ones the kernel could accept. Nothing is applied.
    pub fn validate(&self) -> Result<(), Error> {
        if !(1..=99).contains(&self.priority) {
            return Err(Error::Invalid("priority must be 1 to 99"));
        }
        if self.cpu.is_some_and(|c| c >= MAX_CPUS) {
            return Err(Error::Invalid(
                "cpu is beyond the 1024 an affinity mask holds",
            ));
        }
        if self.prefault_stack_kib > MAX_PREFAULT_STACK_KIB {
            return Err(Error::Invalid(
                "prefault_stack_kib is more than a default stack",
            ));
        }
        Ok(())
    }
}

/// What [`apply`] set, as read back from the kernel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Applied {
    /// The `SCHED_FIFO` priority the thread now has.
    pub priority: u8,
    /// The one CPU the thread is pinned to, if it was asked to be.
    pub cpu: Option<u32>,
    /// Whether the process's memory is locked.
    pub memory_locked: bool,
    /// Stack touched, in KiB.
    pub stack_prefaulted_kib: u32,
}

/// The step that failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// `mlockall`.
    LockMemory,
    /// `sched_setaffinity`.
    Affinity,
    /// `sched_setscheduler`.
    Scheduler,
    /// Reading the thread's own stack bounds.
    StackBounds,
}

/// Why real-time setup did not happen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// The values are not ones the kernel accepts. Nothing was attempted.
    Invalid(&'static str),
    /// The kernel refused a step. `EPERM` is a missing privilege; `ENOMEM` on
    /// `LockMemory` is `RLIMIT_MEMLOCK`.
    Os {
        /// The step.
        step: Step,
        /// `errno`.
        errno: i32,
    },
    /// The thread's stack is too small to prefault that much without overflowing.
    StackTooSmall {
        /// The stack the thread has, in KiB.
        have_kib: u32,
        /// What was asked for, plus the margin kept free, in KiB.
        need_kib: u32,
    },
    /// This is not Linux.
    Unsupported,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(why) => write!(f, "invalid real-time setting: {why}"),
            Self::Os { step, errno } => {
                write!(f, "{step:?} refused by the kernel (errno {errno})")?;
                match (*step, *errno) {
                    (Step::Scheduler, 1) => write!(f, ": needs CAP_SYS_NICE or an rtprio rlimit"),
                    (Step::LockMemory, 12) => write!(f, ": over RLIMIT_MEMLOCK"),
                    _ => Ok(()),
                }
            }
            Self::StackTooSmall { have_kib, need_kib } => write!(
                f,
                "stack is {have_kib} KiB; prefaulting needs {need_kib} KiB with margin"
            ),
            Self::Unsupported => write!(f, "real-time setup needs Linux"),
        }
    }
}

impl std::error::Error for Error {}

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::{apply, is_preempt_rt, lock_memory, pin_to_cpu, prefault_stack, set_fifo};

#[cfg(not(target_os = "linux"))]
mod other {
    use super::{Applied, Config, Error};
    /// Not Linux: always [`Error::Unsupported`].
    pub fn apply(_: &Config) -> Result<Applied, Error> {
        Err(Error::Unsupported)
    }
    /// Not Linux: never.
    pub fn is_preempt_rt() -> bool {
        false
    }
}
#[cfg(not(target_os = "linux"))]
pub use other::{apply, is_preempt_rt};
