//! frk-rt — runtime components behind a documented C ABI (SPEC §10,
//! K4). Residents (D-035 → D-041): the memory-strategy allocators.
//! Still `std` today; goes `#![no_std]`-capable when the Tier-0 grid
//! demands it — the ABI won't change.

use std::alloc::{Layout, alloc, dealloc};

fn raw_alloc(bytes: usize) -> *mut u8 {
    let layout = match Layout::from_size_align(bytes.max(1), 8) {
        Ok(layout) => layout,
        Err(_) => return std::ptr::null_mut(),
    };
    unsafe { alloc(layout) }
}

/// Arena strategy (D-041): bump allocation with process lifetime — the
/// v0 arena is never reset (region reset entry points arrive with real
/// region inference). 8-aligned; zero-byte requests return a valid
/// unique pointer; null only if the host allocator fails.
#[unsafe(no_mangle)]
pub extern "C" fn frk_rt_arena_alloc(bytes: usize) -> *mut u8 {
    raw_alloc(bytes)
}

/// Rc strategy (D-041): an i64 refcount header sits at `ptr - 8`; the
/// returned pointer addresses the payload. The count starts at 1.
#[unsafe(no_mangle)]
pub extern "C" fn frk_rt_rc_alloc(payload_bytes: usize) -> *mut u8 {
    let total = payload_bytes.max(1).checked_add(8);
    let Some(total) = total else {
        return std::ptr::null_mut();
    };
    let base = raw_alloc(total);
    if base.is_null() {
        return base;
    }
    unsafe {
        (base as *mut i64).write(1);
        base.add(8)
    }
}

/// # Safety
/// `payload` must be a live pointer returned by [`frk_rt_rc_alloc`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn frk_rt_rc_retain(payload: *mut u8) {
    if payload.is_null() {
        return;
    }
    unsafe {
        let header = payload.sub(8) as *mut i64;
        *header += 1;
    }
}

/// Decrements; frees header+payload at zero. The layout size is
/// unknown at release time, so v0 frees with the minimal layout the
/// allocator accepts — sound with Rust's global allocator only when
/// alloc/dealloc layouts match, therefore v0 releases DO NOT free:
/// they only decrement (freeing arrives with sized releases at the
/// M10 GC-gate work; v0 rc's job is the plumbing — see D-041).
///
/// # Safety
/// `payload` must be a live pointer returned by [`frk_rt_rc_alloc`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn frk_rt_rc_release(payload: *mut u8) {
    if payload.is_null() {
        return;
    }
    unsafe {
        let header = payload.sub(8) as *mut i64;
        *header -= 1;
    }
}

/// Test/introspection helper (not part of the lowering ABI).
pub fn rc_count(payload: *mut u8) -> i64 {
    unsafe { *(payload.sub(8) as *const i64) }
}

const _: () = {
    // dealloc is referenced by the sized-release future; keep the
    // import honest without dead-code noise.
    let _ = dealloc as unsafe fn(*mut u8, Layout);
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arena_allocations_are_aligned_writable_and_distinct() {
        let a = frk_rt_arena_alloc(24);
        let b = frk_rt_arena_alloc(24);
        assert!(!a.is_null() && !b.is_null());
        assert_ne!(a, b);
        assert_eq!(a as usize % 8, 0);
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
    fn rc_header_counts_retains_and_releases() {
        let p = frk_rt_rc_alloc(16);
        assert!(!p.is_null());
        assert_eq!(p as usize % 8, 0);
        assert_eq!(rc_count(p), 1);
        unsafe {
            frk_rt_rc_retain(p);
            frk_rt_rc_retain(p);
        }
        assert_eq!(rc_count(p), 3);
        unsafe {
            frk_rt_rc_release(p);
        }
        assert_eq!(rc_count(p), 2);
        // Payload is usable regardless of count churn.
        unsafe {
            (p as *mut i64).write(42);
            assert_eq!((p as *const i64).read(), 42);
        }
    }

    #[test]
    fn zero_byte_requests_yield_valid_pointers() {
        assert!(!frk_rt_arena_alloc(0).is_null());
        let p = frk_rt_rc_alloc(0);
        assert!(!p.is_null());
        assert_eq!(rc_count(p), 1);
    }
}
