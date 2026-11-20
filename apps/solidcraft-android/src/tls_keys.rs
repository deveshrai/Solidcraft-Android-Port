//! Thread-local keys that do not run out.
//!
//! Rust's std on Android keeps each `thread_local!` under a pthread key of its own, made the first
//! time the thread-local is used and never given back. SolidCraft has about 150 of them, and
//! Android gives a process about 128 keys, shared with ART, the graphics driver and the system's
//! libraries. When none is left, std aborts ("out of TLS keys"): on a Galaxy Z Fold (Adreno 840)
//! that was the chamfer preview on a cone, in 0.45.0a.
//!
//! The link (`build.rs`) sends this library's key calls here. Every key made in it is one of ours:
//! an index into a table that each thread keeps under a single system key.

use std::ffi::{c_int, c_void};
use std::ptr;
use std::sync::atomic::{AtomicPtr, AtomicU8, AtomicUsize, Ordering};

use libc::pthread_key_t;

type Dtor = unsafe extern "C" fn(*mut c_void);
type Table = Vec<*mut c_void>;

/// The most keys we hand out: std makes one per thread-local.
const KEYS: usize = 4096;
/// Our keys: this bit and an index below `KEYS`. Bionic's keys have the top bit set and macOS's
/// are small numbers, so neither is taken for one of ours.
const OURS: usize = 0x4000_0000;
/// The rounds of destructors at a thread's exit, as POSIX and bionic run them.
const ROUNDS: usize = 4;

static NEXT: AtomicUsize = AtomicUsize::new(0);
static DTORS: [AtomicPtr<()>; KEYS] = [const { AtomicPtr::new(ptr::null_mut()) }; KEYS];
/// The system key the tables live under: `HUB_STATE` says whether it is made yet.
static HUB: AtomicUsize = AtomicUsize::new(0);
static HUB_STATE: AtomicU8 = AtomicU8::new(UNMADE);
const UNMADE: u8 = 0;
const MAKING: u8 = 1;
const MADE: u8 = 2;
const REFUSED: u8 = 3;

/// The system's own calls. On Android the link's `--wrap` names them `__real_…`.
#[cfg(target_os = "android")]
#[allow(unsafe_code)]
mod real {
    use super::{Dtor, c_int, c_void, pthread_key_t};

    unsafe extern "C" {
        #[link_name = "__real_pthread_key_create"]
        pub fn pthread_key_create(key: *mut pthread_key_t, dtor: Option<Dtor>) -> c_int;
        #[link_name = "__real_pthread_key_delete"]
        pub fn pthread_key_delete(key: pthread_key_t) -> c_int;
        #[link_name = "__real_pthread_getspecific"]
        pub fn pthread_getspecific(key: pthread_key_t) -> *mut c_void;
        #[link_name = "__real_pthread_setspecific"]
        pub fn pthread_setspecific(key: pthread_key_t, value: *const c_void) -> c_int;
    }
}
#[cfg(not(target_os = "android"))]
use libc as real;

/// The system key for the tables, made by the first call.
#[allow(unsafe_code)]
fn hub() -> Option<pthread_key_t> {
    loop {
        match HUB_STATE.compare_exchange(UNMADE, MAKING, Ordering::AcqRel, Ordering::Acquire) {
            Ok(_) => {
                let mut key: pthread_key_t = 0;
                // SAFETY: `key` is a place for the new key, and `exit` is a destructor of the
                // type pthreads calls.
                let made = unsafe { real::pthread_key_create(&mut key, Some(exit)) } == 0;
                HUB.store(key as usize, Ordering::Relaxed);
                HUB_STATE.store(if made { MADE } else { REFUSED }, Ordering::Release);
            }
            Err(MADE) => return Some(HUB.load(Ordering::Relaxed) as pthread_key_t),
            Err(REFUSED) => return None,
            Err(_) => std::hint::spin_loop(),
        }
    }
}

/// The system key for the tables if it is made: no table exists before it.
fn hub_made() -> Option<pthread_key_t> {
    (HUB_STATE.load(Ordering::Acquire) == MADE).then(|| HUB.load(Ordering::Relaxed) as pthread_key_t)
}

/// The index of one of our keys; `None` for a system key.
fn index(key: pthread_key_t) -> Option<usize> {
    let k = key as usize;
    (k & !(KEYS - 1) == OURS).then_some(k & (KEYS - 1))
}

/// A new key with `dtor` for each thread's value at its exit; `None` when all are taken.
pub(crate) fn make(dtor: Option<Dtor>) -> Option<pthread_key_t> {
    hub()?;
    let i = NEXT.fetch_add(1, Ordering::Relaxed);
    DTORS.get(i)?.store(dtor.map_or(ptr::null_mut(), |d| d as *mut ()), Ordering::Release);
    Some((OURS | i) as pthread_key_t)
}

/// This thread's value for `key`.
#[allow(unsafe_code)]
pub(crate) fn get(key: pthread_key_t) -> *mut c_void {
    let Some(i) = index(key) else {
        // SAFETY: a system key, passed on as given.
        return unsafe { real::pthread_getspecific(key) };
    };
    let Some(hub) = hub_made() else { return ptr::null_mut() };
    // SAFETY: the hub's value is null or this thread's table, which only this thread touches.
    let table = unsafe { real::pthread_getspecific(hub) }.cast::<Table>();
    unsafe { table.as_ref() }.and_then(|t| t.get(i).copied()).unwrap_or(ptr::null_mut())
}

/// Sets this thread's value for `key`: 0, or an error number.
#[allow(unsafe_code)]
pub(crate) fn set(key: pthread_key_t, value: *const c_void) -> c_int {
    let Some(i) = index(key) else {
        // SAFETY: a system key, passed on as given.
        return unsafe { real::pthread_setspecific(key, value) };
    };
    let Some(hub) = hub_made() else { return libc::EINVAL };
    // SAFETY: as in `get`.
    let mut table = unsafe { real::pthread_getspecific(hub) }.cast::<Table>();
    if table.is_null() {
        if value.is_null() {
            return 0;
        }
        table = Box::into_raw(Box::new(Table::new()));
        // SAFETY: the hub is a system key; the table is this thread's until `exit` frees it.
        if unsafe { real::pthread_setspecific(hub, table.cast()) } != 0 {
            drop(unsafe { Box::from_raw(table) });
            return libc::ENOMEM;
        }
    }
    // SAFETY: this thread's table, not borrowed elsewhere while this runs.
    let table = unsafe { &mut *table };
    if table.len() <= i {
        table.resize(i + 1, ptr::null_mut());
    }
    if let Some(slot) = table.get_mut(i) {
        *slot = value.cast_mut();
    }
    0
}

/// Forgets `key`: its values stay where they are, and no destructor runs for them.
fn delete(key: pthread_key_t) -> c_int {
    let Some(i) = index(key) else { return real_delete(key) };
    DTORS.get(i).map_or(libc::EINVAL, |d| {
        d.store(ptr::null_mut(), Ordering::Release);
        0
    })
}

#[allow(unsafe_code)]
fn real_delete(key: pthread_key_t) -> c_int {
    // SAFETY: a system key, passed on as given.
    unsafe { real::pthread_key_delete(key) }
}

/// A thread's exit: the destructors of its values, in rounds while they set new ones, then the
/// table.
#[allow(unsafe_code)]
unsafe extern "C" fn exit(table: *mut c_void) {
    let Some(hub) = hub_made() else { return };
    let table = table.cast::<Table>();
    // Back under the hub while the destructors run: they may read and set other thread-locals.
    // SAFETY: the table this thread's hub held, handed back by pthreads.
    unsafe { real::pthread_setspecific(hub, table.cast()) };
    for _ in 0..ROUNDS {
        let mut ran = false;
        let mut i = 0;
        // SAFETY: this thread's table; each borrow ends before a destructor runs.
        while let Some(slot) = (unsafe { &mut *table }).get_mut(i) {
            let value = std::mem::replace(slot, ptr::null_mut());
            let dtor = DTORS.get(i).map_or(ptr::null_mut(), |d| d.load(Ordering::Acquire));
            i += 1;
            if !value.is_null() && !dtor.is_null() {
                // SAFETY: `dtor` came from a `Dtor` in `make`; the value is the one set for it.
                unsafe { std::mem::transmute::<*mut (), Dtor>(dtor)(value) };
                ran = true;
            }
        }
        if !ran {
            break;
        }
    }
    // SAFETY: the hub is a system key; the table came from `Box::into_raw` in `set`.
    unsafe { real::pthread_setspecific(hub, ptr::null()) };
    drop(unsafe { Box::from_raw(table) });
}

// The calls std makes, which the link's `--wrap` sends here.
#[cfg(target_os = "android")]
#[allow(unsafe_code)]
mod wrap {
    use super::{Dtor, c_int, c_void, pthread_key_t};

    #[unsafe(no_mangle)]
    unsafe extern "C" fn __wrap_pthread_key_create(key: *mut pthread_key_t, dtor: Option<Dtor>) -> c_int {
        if key.is_null() {
            return libc::EINVAL;
        }
        match super::make(dtor) {
            // SAFETY: the caller's place for the key, checked not null.
            Some(k) => unsafe { *key = k },
            None => return libc::EAGAIN,
        }
        0
    }

    #[unsafe(no_mangle)]
    extern "C" fn __wrap_pthread_key_delete(key: pthread_key_t) -> c_int {
        super::delete(key)
    }

    #[unsafe(no_mangle)]
    extern "C" fn __wrap_pthread_getspecific(key: pthread_key_t) -> *mut c_void {
        super::get(key)
    }

    #[unsafe(no_mangle)]
    extern "C" fn __wrap_pthread_setspecific(key: pthread_key_t, value: *const c_void) -> c_int {
        super::set(key, value)
    }
}

#[cfg(test)]
#[allow(unsafe_code)]
mod tests {
    use super::*;

    static DROPPED: AtomicUsize = AtomicUsize::new(0);
    static LATE: AtomicUsize = AtomicUsize::new(usize::MAX);

    unsafe extern "C" fn count(_: *mut c_void) {
        DROPPED.fetch_add(1, Ordering::SeqCst);
    }

    /// Sets the key `LATE` names in the next round, as a std destructor touching a thread-local.
    unsafe extern "C" fn set_late(_: *mut c_void) {
        set(LATE.load(Ordering::SeqCst) as pthread_key_t, ptr::without_provenance(1));
    }

    #[test]
    fn keys_hold_a_value_per_thread_past_the_systems_limit() {
        // macOS gives a process 512 keys; ours go past them.
        let keys: Vec<_> = (0..600).filter_map(|_| make(None)).collect();
        assert_eq!(keys.len(), 600);
        for (n, k) in keys.iter().enumerate() {
            assert!(index(*k).is_some());
            assert!(get(*k).is_null());
            assert_eq!(set(*k, ptr::without_provenance(n + 1)), 0);
        }
        let last = keys.last().copied().unwrap_or(0);
        let other = std::thread::spawn(move || {
            let before = get(last) as usize;
            set(last, ptr::without_provenance(7));
            (before, get(last) as usize)
        });
        assert_eq!(other.join().ok(), Some((0, 7)));
        assert!(keys.iter().enumerate().all(|(n, k)| get(*k) as usize == n + 1));
        assert!(make(None).is_some_and(|k| delete(k) == 0));
    }

    #[test]
    fn a_threads_exit_runs_the_destructors_in_rounds() {
        let counted = make(Some(count)).unwrap_or(0);
        let late = make(Some(count)).unwrap_or(0);
        // Made after `late`, so its destructor runs after `late`'s turn: the value it sets for
        // `late` waits for the next round.
        let setter = make(Some(set_late)).unwrap_or(0);
        LATE.store(late as usize, Ordering::SeqCst);
        let before = DROPPED.load(Ordering::SeqCst);
        std::thread::spawn(move || {
            set(counted, ptr::without_provenance(1));
            set(setter, ptr::without_provenance(1));
        })
        .join()
        .ok();
        assert_eq!(DROPPED.load(Ordering::SeqCst) - before, 2);
        // A system key passes through untouched.
        let mut sys: pthread_key_t = 0;
        assert_eq!(unsafe { real::pthread_key_create(&mut sys, None) }, 0);
        assert!(index(sys).is_none());
        assert_eq!(set(sys, ptr::without_provenance(3)), 0);
        assert_eq!(get(sys) as usize, 3);
        assert_eq!(real_delete(sys), 0);
    }
}
