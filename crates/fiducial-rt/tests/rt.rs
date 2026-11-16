//! Real-time setup against the real kernel. What an unprivileged CI runner can
//! prove is asserted exactly; what needs `CAP_SYS_NICE` is asserted as either a
//! kernel-read-back success or a structured `EPERM`, never skipped silently.
#![cfg(target_os = "linux")]

use fiducial_rt::{
    apply, is_preempt_rt, pin_to_cpu, prefault_stack, set_fifo, Config, Error, Step,
};

/// Run on a fresh thread: affinity and scheduling are per-thread, so the test
/// runner's own threads are never changed.
fn on_thread<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(4 * 1024 * 1024)
        .spawn(f)
        .unwrap()
        .join()
        .unwrap()
}

fn allowed_cpu() -> u32 {
    // SAFETY: zeroed set is empty; the kernel fills it.
    unsafe {
        let mut set: libc::cpu_set_t = core::mem::zeroed();
        assert_eq!(
            libc::sched_getaffinity(0, core::mem::size_of::<libc::cpu_set_t>(), &mut set),
            0
        );
        (0..1024)
            .find(|c| libc::CPU_ISSET(*c as usize, &set))
            .expect("some cpu is allowed")
    }
}

fn affinity_now() -> Vec<u32> {
    // SAFETY: as above.
    unsafe {
        let mut set: libc::cpu_set_t = core::mem::zeroed();
        assert_eq!(
            libc::sched_getaffinity(0, core::mem::size_of::<libc::cpu_set_t>(), &mut set),
            0
        );
        (0..1024)
            .filter(|c| libc::CPU_ISSET(*c as usize, &set))
            .collect()
    }
}

#[test]
fn bad_values_are_refused_before_anything_is_attempted() {
    for (cfg, _) in [
        (Config::new(0), "priority 0"),
        (Config::new(100), "priority 100"),
        (Config::new(50).cpu(1024), "cpu 1024"),
        (Config::new(50).prefault_stack_kib(8193), "stack 8193"),
    ] {
        assert!(matches!(cfg.validate(), Err(Error::Invalid(_))), "{cfg:?}");
        assert!(
            matches!(on_thread(move || apply(&cfg)), Err(Error::Invalid(_))),
            "{cfg:?}"
        );
    }
    assert_eq!(
        Config::new(99)
            .cpu(1023)
            .prefault_stack_kib(8192)
            .validate(),
        Ok(())
    );
}

#[test]
fn pinning_to_an_allowed_cpu_is_read_back_exactly() {
    let cpu = allowed_cpu();
    let got = on_thread(move || {
        pin_to_cpu(cpu).unwrap();
        affinity_now()
    });
    assert_eq!(got, vec![cpu], "the kernel's mask is exactly the one CPU");
}

#[test]
fn pinning_to_a_cpu_that_does_not_exist_is_einval() {
    // 1000 is valid for the mask but no machine has it: deterministic EINVAL.
    let e = on_thread(|| pin_to_cpu(1000)).unwrap_err();
    assert_eq!(
        e,
        Error::Os {
            step: Step::Affinity,
            errno: libc::EINVAL
        }
    );
}

#[test]
fn prefaulting_touches_stack_that_exists_and_refuses_stack_that_does_not() {
    // The helper thread has 4 MiB: 512 KiB fits, 4000 KiB with margin does not,
    // and the refusal comes back instead of a SIGSEGV from overflowing.
    assert_eq!(on_thread(|| prefault_stack(512)), Ok(()));
    assert_eq!(on_thread(|| prefault_stack(0)), Ok(()));
    match on_thread(|| prefault_stack(4000)) {
        Err(Error::StackTooSmall { have_kib, need_kib }) => {
            assert!(need_kib > have_kib, "{need_kib} vs {have_kib}");
        }
        other => panic!("expected StackTooSmall, got {other:?}"),
    }
}

#[test]
fn the_scheduler_is_changed_or_refused_with_a_named_errno() {
    let r = on_thread(|| {
        let r = set_fifo(1);
        // SAFETY: reads this thread's policy.
        let policy = unsafe { libc::sched_getscheduler(0) };
        (r, policy)
    });
    match r {
        (Ok(()), policy) => assert_eq!(policy, libc::SCHED_FIFO, "kernel agrees it took effect"),
        (
            Err(Error::Os {
                step: Step::Scheduler,
                errno,
            }),
            policy,
        ) => {
            assert_eq!(
                errno,
                libc::EPERM,
                "the only expected refusal is missing privilege"
            );
            assert_eq!(
                policy,
                libc::SCHED_OTHER,
                "a refusal leaves the thread unchanged"
            );
        }
        (other, _) => panic!("{other:?}"),
    }
}

#[test]
fn apply_returns_what_the_kernel_holds_or_a_structured_refusal() {
    let cpu = allowed_cpu();
    let cfg = Config::new(1).cpu(cpu).unlocked().prefault_stack_kib(256);
    match on_thread(move || apply(&cfg)) {
        Ok(a) => {
            assert_eq!(a.priority, 1);
            assert_eq!(a.cpu, Some(cpu));
            assert!(!a.memory_locked);
            assert_eq!(a.stack_prefaulted_kib, 256);
        }
        Err(Error::Os { step, errno }) => {
            assert_eq!((step, errno), (Step::Scheduler, libc::EPERM));
        }
        Err(e) => panic!("{e}"),
    }
}

#[test]
fn a_failure_before_the_scheduler_leaves_the_thread_at_normal_priority() {
    // A CPU that does not exist fails at Affinity, which comes before the
    // scheduler step: so the thread must still be SCHED_OTHER.
    let (r, policy) = on_thread(|| {
        let r = apply(&Config::new(1).cpu(1000).unlocked());
        // SAFETY: reads this thread's policy.
        (r, unsafe { libc::sched_getscheduler(0) })
    });
    assert_eq!(
        r,
        Err(Error::Os {
            step: Step::Affinity,
            errno: libc::EINVAL
        })
    );
    assert_eq!(policy, libc::SCHED_OTHER);
}

#[test]
fn memory_locking_is_read_back_from_proc_or_refused_as_enomem_or_eperm() {
    let r = on_thread(|| {
        let r = fiducial_rt::lock_memory();
        let locked = std::fs::read_to_string("/proc/self/status")
            .unwrap()
            .lines()
            .find(|l| l.starts_with("VmLck:"))
            .and_then(|l| {
                l.split_whitespace()
                    .nth(1)
                    .and_then(|n| n.parse::<u64>().ok())
            })
            .unwrap_or(0);
        // SAFETY: undo, so later tests in this process allocate normally.
        unsafe { libc::munlockall() };
        (r, locked)
    });
    match r {
        (Ok(()), kib) => assert!(kib > 0, "mlockall succeeded but VmLck is {kib} kB"),
        (
            Err(Error::Os {
                step: Step::LockMemory,
                errno,
            }),
            _,
        ) => {
            assert!(
                errno == libc::ENOMEM || errno == libc::EPERM,
                "errno {errno}"
            );
        }
        (other, _) => panic!("{other:?}"),
    }
}

#[test]
fn preempt_rt_detection_agrees_with_the_kernel_file() {
    let file = std::fs::read_to_string("/sys/kernel/realtime").map(|s| s.trim() == "1");
    assert_eq!(is_preempt_rt(), file.unwrap_or(false));
}

#[test]
fn errors_say_what_to_do() {
    let e = Error::Os {
        step: Step::Scheduler,
        errno: libc::EPERM,
    }
    .to_string();
    assert!(e.contains("CAP_SYS_NICE"), "{e}");
    let e = Error::Os {
        step: Step::LockMemory,
        errno: libc::ENOMEM,
    }
    .to_string();
    assert!(e.contains("RLIMIT_MEMLOCK"), "{e}");
}
