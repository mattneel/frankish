//! frk-rt — the runtime component library behind a documented C ABI,
//! freestanding-first so Tier 0 stays as wide as every LLVM triple
//! (SPEC §10, contract point K4).
//!
//! First resident (M4, D-035): [`frk_rt_alloc`], the closure-environment
//! allocator. The crate builds as rlib + staticlib; the JIT runner
//! links the rlib and registers the symbol with the execution engine,
//! AOT (M7) links the staticlib. Still `std` today; goes `#![no_std]`
//! when the Tier-0 grid arrives (M7) — the ABI won't change.

use std::alloc::{Layout, alloc};

/// Allocates `bytes` of memory aligned to 8, for closure environments
/// and (later) every kernel-dialect runtime allocation.
///
/// # v0 contract (D-035)
///
/// **Leaks by design.** There is no free: environments live for the
/// process. The arena/rc memory discipline (frk.mem, M7) replaces this
/// implementation behind the same symbol; callers never change.
/// Zero-byte requests still return a valid unique pointer. Returns null
/// only if the host allocator fails.
///
/// # Safety
///
/// Callable from C/JIT code with any `bytes`; the returned region is
/// uninitialized.
#[unsafe(no_mangle)]
pub extern "C" fn frk_rt_alloc(bytes: usize) -> *mut u8 {
    let layout = match Layout::from_size_align(bytes.max(1), 8) {
        Ok(layout) => layout,
        Err(_) => return std::ptr::null_mut(),
    };
    unsafe { alloc(layout) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocations_are_aligned_writable_and_distinct() {
        let a = frk_rt_alloc(24);
        let b = frk_rt_alloc(24);
        assert!(!a.is_null() && !b.is_null());
        assert_ne!(a, b, "leaking bump must still hand out unique blocks");
        assert_eq!(a as usize % 8, 0);
        assert_eq!(b as usize % 8, 0);
        unsafe {
            for offset in 0..24 {
                a.add(offset).write(offset as u8);
            }
            for offset in 0..24 {
                assert_eq!(a.add(offset).read(), offset as u8);
            }
        }
    }

    #[test]
    fn zero_byte_requests_yield_valid_pointers() {
        let p = frk_rt_alloc(0);
        assert!(!p.is_null());
        assert_eq!(p as usize % 8, 0);
    }
}
