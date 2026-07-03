//! frk-rt — runtime components behind a documented C ABI (SPEC §10,
//! K4). Residents (D-035 → D-041): the memory-strategy allocators.
//! Still `std` today; goes `#![no_std]`-capable when the Tier-0 grid
//! demands it — the ABI won't change.

use std::alloc::{Layout, alloc, dealloc};
use std::sync::atomic::{AtomicU64, Ordering};

/// Total allocations across both strategies — the measurable target
/// the M10 release/liveness pass will be held against (requested at
/// the D-041 ratification; the leak-canary golden becomes writable
/// the day releases land).
static ALLOCS: AtomicU64 = AtomicU64::new(0);

#[unsafe(no_mangle)]
pub extern "C" fn frk_rt_alloc_count() -> u64 {
    ALLOCS.load(Ordering::Relaxed)
}

/// Releases executed (GC ladder step 1, D-053): with alloc_count,
/// the live-object measure the leak canary asserts against.
static RELEASES: AtomicU64 = AtomicU64::new(0);

#[unsafe(no_mangle)]
pub extern "C" fn frk_rt_rc_release_count() -> u64 {
    RELEASES.load(Ordering::Relaxed)
}

fn raw_alloc(bytes: u64) -> *mut u8 {
    let layout = match Layout::from_size_align((bytes.max(1)) as usize, 8) {
        Ok(layout) => layout,
        Err(_) => return std::ptr::null_mut(),
    };
    unsafe { alloc(layout) }
}

/// Arena strategy (D-041): bump allocation with process lifetime — the
/// v0 arena is never reset (region reset entry points arrive with real
/// region inference). 8-aligned; zero-byte requests return a valid
/// unique pointer; null only if the host allocator fails. Sizes are
/// u64 on every target (D-042 — 32-bit-word targets enforce exact
/// signatures; the c mirror matches).
#[unsafe(no_mangle)]
pub extern "C" fn frk_rt_arena_alloc(bytes: u64) -> *mut u8 {
    ALLOCS.fetch_add(1, Ordering::Relaxed);
    raw_alloc(bytes)
}

/// Rc strategy (D-041): an i64 refcount header sits at `ptr - 8`; the
/// returned pointer addresses the payload. The count starts at 1.
#[unsafe(no_mangle)]
pub extern "C" fn frk_rt_rc_alloc(payload_bytes: u64) -> *mut u8 {
    ALLOCS.fetch_add(1, Ordering::Relaxed);
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
    RELEASES.fetch_add(1, Ordering::Relaxed);
    unsafe {
        let header = payload.sub(8) as *mut i64;
        *header -= 1;
    }
}

/// JS-faithful f64 → text (D-047): Rust's Display IS shortest
/// round-trip, and its integer-value trimming matches JS ToString for
/// the corpus range (canon rule: values are 0 or |v| ∈ [1e-4, 1e15),
/// finite — outside that JS switches to exponent spellings we fence).
pub fn format_f64(value: f64) -> String {
    format!("{value}")
}

/// Prints one number, newline-terminated (console.log protocol).
#[unsafe(no_mangle)]
pub extern "C" fn frk_rt_print_f64(value: f64) {
    println!("{}", format_f64(value));
}

/// Prints a boolean as JS spells it. i1 arrives zero-extended.
#[unsafe(no_mangle)]
pub extern "C" fn frk_rt_print_bool(value: u8) {
    println!("{}", if value != 0 { "true" } else { "false" });
}

/// The dyn tag check (D-051/D-054): straight-line native total
/// semantics — mismatch prints and aborts. Corpus law keeps
/// mismatches out of in-process (JIT) golden runs; AOT verifies the
/// abort path as a subprocess.
#[unsafe(no_mangle)]
pub extern "C" fn frk_rt_dyn_check(actual: i64, expected: i64) {
    if actual != expected {
        eprintln!("frk: dyn tag mismatch: expected {expected}, got {actual} (D-051)");
        std::process::abort();
    }
}

// ---- strings (M9, D-049): rt-owned immutable UTF-16 values. Layout
// {len: u64, units: u16 × len}, one allocation, plain malloc-domain
// (strategy-independent; revisit at the M10 GC gate). ----

fn str_alloc(len_units: u64) -> *mut u8 {
    let bytes = 8u64.saturating_add(len_units.saturating_mul(2));
    let base = raw_alloc(bytes);
    if !base.is_null() {
        unsafe { (base as *mut u64).write(len_units) };
    }
    base
}

unsafe fn str_parts<'a>(s: *const u8) -> (u64, &'a [u16]) {
    unsafe {
        let len = *(s as *const u64);
        let units = std::slice::from_raw_parts(s.add(8) as *const u16, len as usize);
        (len, units)
    }
}

/// # Safety
/// `units` must point at `len` valid u16s (or be unused when len=0).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn frk_rt_str_from_units(units: *const u16, len: u64) -> *mut u8 {
    let base = str_alloc(len);
    if base.is_null() || len == 0 {
        return base;
    }
    unsafe {
        std::ptr::copy_nonoverlapping(units, base.add(8) as *mut u16, len as usize);
    }
    base
}

/// # Safety
/// Both operands must be live strings from this runtime.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn frk_rt_str_concat(a: *const u8, b: *const u8) -> *mut u8 {
    unsafe {
        let (alen, aunits) = str_parts(a);
        let (blen, bunits) = str_parts(b);
        let base = str_alloc(alen + blen);
        if base.is_null() {
            return base;
        }
        let data = base.add(8) as *mut u16;
        std::ptr::copy_nonoverlapping(aunits.as_ptr(), data, alen as usize);
        std::ptr::copy_nonoverlapping(bunits.as_ptr(), data.add(alen as usize), blen as usize);
        base
    }
}

/// # Safety
/// Both operands must be live strings from this runtime.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn frk_rt_str_eq(a: *const u8, b: *const u8) -> i64 {
    unsafe {
        let (alen, aunits) = str_parts(a);
        let (blen, bunits) = str_parts(b);
        (alen == blen && aunits == bunits) as i64
    }
}

/// # Safety
/// The operand must be a live string from this runtime.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn frk_rt_str_len(s: *const u8) -> u64 {
    unsafe { str_parts(s).0 }
}

/// # Safety
/// The operand must be a live string from this runtime.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn frk_rt_print_str(s: *const u8) {
    let text = unsafe { String::from_utf16_lossy(str_parts(s).1) };
    println!("{text}");
}

// ---- byte strings (M11 bar 3; D-052/D-056): interned, identity-
// equal. Layout {u64 len, bytes}; the intern table owns canonical
// pointers for the process lifetime (strings are rt values, outside
// the strategy axis until the tracer — D-049 precedent). ----

fn intern_table() -> &'static std::sync::Mutex<std::collections::HashMap<Vec<u8>, usize>> {
    static TABLE: std::sync::OnceLock<
        std::sync::Mutex<std::collections::HashMap<Vec<u8>, usize>>,
    > = std::sync::OnceLock::new();
    TABLE.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
}

fn bstr_intern_bytes(bytes: &[u8]) -> *mut u8 {
    let mut table = intern_table().lock().expect("intern table");
    if let Some(&canonical) = table.get(bytes) {
        return canonical as *mut u8;
    }
    let base = raw_alloc(8 + bytes.len() as u64);
    if base.is_null() {
        return base;
    }
    unsafe {
        (base as *mut u64).write(bytes.len() as u64);
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), base.add(8), bytes.len());
    }
    table.insert(bytes.to_vec(), base as usize);
    base
}

unsafe fn bstr_parts<'a>(s: *const u8) -> &'a [u8] {
    unsafe {
        let len = *(s as *const u64) as usize;
        std::slice::from_raw_parts(s.add(8), len)
    }
}

/// # Safety
/// `bytes` must point at `len` valid bytes (unused when len = 0).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn frk_rt_bstr_intern(bytes: *const u8, len: u64) -> *mut u8 {
    let slice = if len == 0 {
        &[][..]
    } else {
        unsafe { std::slice::from_raw_parts(bytes, len as usize) }
    };
    bstr_intern_bytes(slice)
}

/// # Safety
/// Both operands must be canonical byte strings from this runtime.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn frk_rt_bstr_concat(a: *const u8, b: *const u8) -> *mut u8 {
    let mut bytes = unsafe { bstr_parts(a).to_vec() };
    bytes.extend_from_slice(unsafe { bstr_parts(b) });
    bstr_intern_bytes(&bytes)
}

/// %.14g into an interned byte string — tostring and ..-coercion ride
/// the SAME formatter the parity rig proved (D-056).
#[unsafe(no_mangle)]
pub extern "C" fn frk_rt_bstr_from_num(value: f64) -> *mut u8 {
    bstr_intern_bytes(format_lua_num(value).as_bytes())
}

/// Raw bytes + newline (Lua print of a string; 8-bit clean).
///
/// # Safety
/// The operand must be a canonical byte string from this runtime.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn frk_rt_print_lua_str(s: *const u8) {
    use std::io::Write;
    let bytes = unsafe { bstr_parts(s) };
    let mut stdout = std::io::stdout().lock();
    let _ = stdout.write_all(bytes);
    let _ = stdout.write_all(b"\n");
}

// ---- Lua printing (M11 bar 4; D-052/D-055): %.14g semantics. ----

/// C's %.14g inside the canon fence (positional values: 0 or |v| in
/// [1e-4, 1e14), finite, and clear of the round-to-1e14 boundary).
/// %.14g ROUNDS — 14 significant digits, half-to-even — unlike the
/// TS-0 printers' shortest-round-trip (D-055.2). Rust's fixed-
/// precision formatting is correctly rounded with the same tie rule,
/// so parity with the C twin is exact; the cross-twin rig proves it.
pub fn format_lua_num(value: f64) -> String {
    if value == 0.0 {
        return if value.is_sign_negative() { "-0".into() } else { "0".into() };
    }
    let sci = format!("{value:.13e}"); // 14 significant digits
    let (mantissa, exponent) = sci.split_once('e').expect("sci form");
    let exponent: i32 = exponent.parse().expect("exponent");
    let negative = mantissa.starts_with('-');
    let digits: String = mantissa.chars().filter(char::is_ascii_digit).collect();
    debug_assert_eq!(digits.len(), 14);

    let point = exponent + 1; // digit count before the decimal point
    let mut out = String::new();
    if negative {
        out.push('-');
    }
    if point <= 0 {
        // 0.000ddd — %g strips trailing zeros.
        out.push_str("0.");
        for _ in 0..(-point) {
            out.push('0');
        }
        out.push_str(digits.trim_end_matches('0'));
    } else if point as usize >= digits.len() {
        out.push_str(&digits);
        for _ in 0..(point as usize - digits.len()) {
            out.push('0');
        }
    } else {
        let (integer, fraction) = digits.split_at(point as usize);
        out.push_str(integer);
        let fraction = fraction.trim_end_matches('0');
        if !fraction.is_empty() {
            out.push('.');
            out.push_str(fraction);
        }
    }
    out
}

#[unsafe(no_mangle)]
pub extern "C" fn frk_rt_print_lua_num(value: f64) {
    println!("{}", format_lua_num(value));
}

#[unsafe(no_mangle)]
pub extern "C" fn frk_rt_print_lua_bool(value: u8) {
    println!("{}", if value != 0 { "true" } else { "false" });
}

#[unsafe(no_mangle)]
pub extern "C" fn frk_rt_print_lua_nil() {
    println!("nil");
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
    fn byte_strings_intern_to_identical_pointers() {
        unsafe {
            let a = frk_rt_bstr_intern(b"hello".as_ptr(), 5);
            let b = frk_rt_bstr_intern(b"hello".as_ptr(), 5);
            assert_eq!(a, b, "interning canonicalizes");
            let c = frk_rt_bstr_concat(
                frk_rt_bstr_intern(b"hel".as_ptr(), 3),
                frk_rt_bstr_intern(b"lo".as_ptr(), 2),
            );
            assert_eq!(a, c, "concat interns to the same canonical pointer");
            assert_ne!(a, frk_rt_bstr_intern(b"world".as_ptr(), 5));
            assert_eq!(*(a as *const u64), 5);
        }
        let n = frk_rt_bstr_from_num(42.0);
        unsafe {
            assert_eq!(bstr_parts(n), b"42");
        }
    }

    #[test]
    fn lua_formatting_is_percent_14g_with_half_even_ties() {
        assert_eq!(format_lua_num(42.0), "42");
        assert_eq!(format_lua_num(-0.0), "-0");
        assert_eq!(format_lua_num(0.1), "0.1");
        assert_eq!(format_lua_num(1.0 / 3.0), "0.33333333333333");
        assert_eq!(format_lua_num(0.0001), "0.0001");
        assert_eq!(format_lua_num(2.5), "2.5");
        // THE TIE PAIR (D-055.2): binary-exact values whose 15th
        // significant digit is exactly 5, nothing after — round half
        // to even at digit 14. ...34.5 stays (4 even), ...33.5 goes
        // up (to 4).
        assert_eq!(format_lua_num(12345678901234.5), "12345678901234");
        assert_eq!(format_lua_num(12345678901233.5), "12345678901234");
    }

    #[test]
    fn f64_formatting_is_js_faithful_in_the_fenced_range() {
        assert_eq!(format_f64(832040.0), "832040");
        assert_eq!(format_f64(0.5), "0.5");
        assert_eq!(format_f64(0.1 + 0.2), "0.30000000000000004");
        assert_eq!(format_f64(1.0 / 3.0), "0.3333333333333333");
        assert_eq!(format_f64(-0.0), "-0");
        assert_eq!(format_f64(0.0001), "0.0001");
    }

    #[test]
    fn the_allocation_counter_counts_both_strategies() {
        let before = frk_rt_alloc_count();
        let _ = frk_rt_arena_alloc(8);
        let _ = frk_rt_rc_alloc(8);
        assert!(frk_rt_alloc_count() >= before + 2);
    }

    #[test]
    fn strings_roundtrip_concat_and_compare() {
        let hello: Vec<u16> = "héllo".encode_utf16().collect();
        let world: Vec<u16> = " wörld".encode_utf16().collect();
        unsafe {
            let a = frk_rt_str_from_units(hello.as_ptr(), hello.len() as u64);
            let b = frk_rt_str_from_units(world.as_ptr(), world.len() as u64);
            let ab = frk_rt_str_concat(a, b);
            assert_eq!(frk_rt_str_len(ab), (hello.len() + world.len()) as u64);
            let again = frk_rt_str_concat(a, b);
            assert_eq!(frk_rt_str_eq(ab, again), 1);
            assert_eq!(frk_rt_str_eq(ab, a), 0);
            // Surrogate pairs count 2 (JS .length semantics).
            let emoji: Vec<u16> = "😀".encode_utf16().collect();
            assert_eq!(emoji.len(), 2);
            let e = frk_rt_str_from_units(emoji.as_ptr(), 2);
            assert_eq!(frk_rt_str_len(e), 2);
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
