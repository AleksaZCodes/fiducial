//! The Linux implementation. Every `unsafe` block is one libc call with its
//! arguments built on the line above it.

use crate::{Applied, Config, Error, Step, MAX_CPUS};

fn errno() -> i32 {
    std::io::Error::last_os_error().raw_os_error().unwrap_or(0)
}

/// `mlockall(MCL_CURRENT | MCL_FUTURE)`: every page now and later stays resident.
pub fn lock_memory() -> Result<(), Error> {
    // SAFETY: no pointers; flags only.
    if unsafe { libc::mlockall(libc::MCL_CURRENT | libc::MCL_FUTURE) } != 0 {
        return Err(Error::Os {
            step: Step::LockMemory,
            errno: errno(),
        });
    }
    Ok(())
}

/// Pin the calling thread to one CPU.
pub fn pin_to_cpu(cpu: u32) -> Result<(), Error> {
    if cpu >= MAX_CPUS {
        return Err(Error::Invalid(
            "cpu is beyond the 1024 an affinity mask holds",
        ));
    }
    // SAFETY: a zeroed cpu_set_t is the empty set; CPU_SET is bounds-checked above.
    let mut set: libc::cpu_set_t = unsafe { core::mem::zeroed() };
    unsafe { libc::CPU_SET(cpu as usize, &mut set) };
    // SAFETY: pid 0 is the calling thread; `set` is live and its size is passed.
    let r = unsafe { libc::sched_setaffinity(0, core::mem::size_of::<libc::cpu_set_t>(), &set) };
    if r != 0 {
        return Err(Error::Os {
            step: Step::Affinity,
            errno: errno(),
        });
    }
    Ok(())
}

/// Move the calling thread to `SCHED_FIFO` at `priority`.
pub fn set_fifo(priority: u8) -> Result<(), Error> {
    if !(1..=99).contains(&priority) {
        return Err(Error::Invalid("priority must be 1 to 99"));
    }
    let param = libc::sched_param {
        sched_priority: i32::from(priority),
    };
    // SAFETY: pid 0 is the calling thread; `param` is live.
    if unsafe { libc::sched_setscheduler(0, libc::SCHED_FIFO, &param) } != 0 {
        return Err(Error::Os {
            step: Step::Scheduler,
            errno: errno(),
        });
    }
    Ok(())
}

/// The stack the calling thread has, in bytes.
fn stack_bytes() -> Result<usize, Error> {
    // SAFETY: pthread_getattr_np fills an attr we then read and destroy.
    unsafe {
        let mut attr: libc::pthread_attr_t = core::mem::zeroed();
        if libc::pthread_getattr_np(libc::pthread_self(), &mut attr) != 0 {
            return Err(Error::Os {
                step: Step::StackBounds,
                errno: errno(),
            });
        }
        let mut addr: *mut libc::c_void = core::ptr::null_mut();
        let mut size: libc::size_t = 0;
        let r = libc::pthread_attr_getstack(&attr, &mut addr, &mut size);
        libc::pthread_attr_destroy(&mut attr);
        if r != 0 {
            return Err(Error::Os {
                step: Step::StackBounds,
                errno: r,
            });
        }
        Ok(size)
    }
}

/// Stack kept free beyond what is prefaulted, for the frames already live.
const MARGIN_KIB: u32 = 128;
const PAGE: usize = 4096;

#[inline(never)]
fn touch(pages: usize) {
    let mut page = [0u8; PAGE];
    // A volatile write to every page keeps the compiler from eliding the frame.
    // SAFETY: index 0 of a live array.
    unsafe { core::ptr::write_volatile(page.as_mut_ptr(), pages as u8) };
    if pages > 1 {
        touch(pages - 1);
    }
    core::hint::black_box(&page);
}

/// Touch `kib` of the calling thread's stack so its pages are mapped before the
/// loop needs them. Refuses, rather than overflows, if the stack is too small.
pub fn prefault_stack(kib: u32) -> Result<(), Error> {
    if kib == 0 {
        return Ok(());
    }
    let have = (stack_bytes()? / 1024) as u32;
    let need = kib + MARGIN_KIB;
    if need > have {
        return Err(Error::StackTooSmall {
            have_kib: have,
            need_kib: need,
        });
    }
    touch((kib as usize * 1024).div_ceil(PAGE));
    Ok(())
}

/// Apply `cfg` to the calling thread (and, for memory locking, the process).
///
/// Order matters: memory is locked and the stack touched **before** the thread
/// becomes real-time, so setup's own page faults happen at normal priority, and
/// the scheduler is set last so a failure earlier leaves the thread unchanged.
/// What is returned is read back from the kernel, not echoed from `cfg`.
pub fn apply(cfg: &Config) -> Result<Applied, Error> {
    cfg.validate()?;
    if cfg.lock_memory {
        lock_memory()?;
    }
    prefault_stack(cfg.prefault_stack_kib)?;
    if let Some(c) = cfg.cpu {
        pin_to_cpu(c)?;
    }
    set_fifo(cfg.priority)?;
    Ok(read_back(cfg))
}

fn read_back(cfg: &Config) -> Applied {
    // SAFETY: pid 0 is the calling thread; out-params are live locals.
    let (policy, prio) = unsafe {
        let mut p = libc::sched_param { sched_priority: 0 };
        let policy = libc::sched_getscheduler(0);
        libc::sched_getparam(0, &mut p);
        (policy, p.sched_priority)
    };
    let cpu = cfg.cpu.and_then(|_| single_cpu());
    Applied {
        priority: if policy == libc::SCHED_FIFO {
            prio as u8
        } else {
            0
        },
        cpu,
        memory_locked: cfg.lock_memory,
        stack_prefaulted_kib: cfg.prefault_stack_kib,
    }
}

/// The CPU the thread is allowed on, if exactly one.
pub(crate) fn single_cpu() -> Option<u32> {
    // SAFETY: zeroed cpu_set_t is empty; the kernel fills it for pid 0.
    let mut set: libc::cpu_set_t = unsafe { core::mem::zeroed() };
    let r =
        unsafe { libc::sched_getaffinity(0, core::mem::size_of::<libc::cpu_set_t>(), &mut set) };
    if r != 0 {
        return None;
    }
    let mut found = None;
    for c in 0..MAX_CPUS {
        // SAFETY: c is within the set.
        if unsafe { libc::CPU_ISSET(c as usize, &set) } {
            if found.is_some() {
                return None;
            }
            found = Some(c);
        }
    }
    found
}

/// Whether the running kernel is `PREEMPT_RT` (`/sys/kernel/realtime` reads `1`).
pub fn is_preempt_rt() -> bool {
    std::fs::read_to_string("/sys/kernel/realtime").is_ok_and(|s| s.trim() == "1")
}
