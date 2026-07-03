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

/// Rc strategy (D-041, header amended by D-057): a THREE-word header
/// precedes the payload —
/// `[layout: u64 @ ptr-24][size: u64 @ ptr-16][rcword: i64 @ ptr-8]`.
/// The rcword packs Bacon–Rajan bookkeeping: bits 62..63 color
/// (0 black, 1 gray, 2 white, 3 purple), bit 61 buffered, bits 0..60
/// the count. Layout encoding per D-057: bits 0..1 kind (0 WORDMAP,
/// 1 TABLE_SHELL, 2 ARRAY); WORDMAP carries 2-bit per-word codes from
/// bit 4 (0 skip, 1 managed ptr, 2 dyn-tag+payload pair); ARRAY's
/// element code sits in bits 2..3. LEAF = all-zero wordmap.
const COLOR_SHIFT: u32 = 62;
const COLOR_MASK: i64 = 0b11 << COLOR_SHIFT;
const BUFFERED_BIT: i64 = 1 << 61;
const COUNT_MASK: i64 = (1 << 61) - 1;
const BLACK: i64 = 0;
const GRAY: i64 = 1;
const WHITE: i64 = 2;
const PURPLE: i64 = 3;

pub const LAYOUT_LEAF: u64 = 0;
pub const LAYOUT_TABLE_SHELL: u64 = 1;
pub const LAYOUT_ARRAY_LEAF: u64 = 2;
pub const LAYOUT_ARRAY_PTR: u64 = 2 | (1 << 2);
pub const LAYOUT_ARRAY_DYN: u64 = 2 | (2 << 2);

/// Wordmap code for payload word `index` (0 skip, 1 ptr, 2 dyn-tag).
pub fn layout_wordmap(codes: &[u8]) -> u64 {
    let mut layout = 0u64;
    for (index, &code) in codes.iter().enumerate().take(30) {
        layout |= (code as u64 & 0b11) << (4 + 2 * index);
    }
    layout
}

unsafe fn header(payload: *mut u8) -> (*mut u64, *mut u64, *mut i64) {
    unsafe {
        (
            payload.sub(24) as *mut u64, // layout
            payload.sub(16) as *mut u64, // size
            payload.sub(8) as *mut i64,  // rcword
        )
    }
}

fn count_of(rcword: i64) -> i64 {
    rcword & COUNT_MASK
}
fn color_of(rcword: i64) -> i64 {
    // LOGICAL shift: the color occupies the sign bits, and an
    // arithmetic i64 shift smears them to -1 (found by the header
    // probe: purple read back as -1 and never matched).
    ((rcword as u64 & COLOR_MASK as u64) >> COLOR_SHIFT) as i64
}
fn with_color(rcword: i64, color: i64) -> i64 {
    (rcword & !COLOR_MASK) | (color << COLOR_SHIFT)
}

static FREES: AtomicU64 = AtomicU64::new(0);

#[unsafe(no_mangle)]
pub extern "C" fn frk_rt_rc_free_count() -> u64 {
    FREES.load(Ordering::Relaxed)
}

fn candidates() -> &'static std::sync::Mutex<Vec<usize>> {
    static BUFFER: std::sync::OnceLock<std::sync::Mutex<Vec<usize>>> =
        std::sync::OnceLock::new();
    BUFFER.get_or_init(|| std::sync::Mutex::new(Vec::new()))
}

#[unsafe(no_mangle)]
pub extern "C" fn frk_rt_rc_alloc(payload_bytes: u64, layout: u64) -> *mut u8 {
    ALLOCS.fetch_add(1, Ordering::Relaxed);
    let Some(total) = payload_bytes.max(1).checked_add(24) else {
        return std::ptr::null_mut();
    };
    let base = raw_alloc(total);
    if base.is_null() {
        return base;
    }
    unsafe {
        let payload = base.add(24);
        let (l, s, r) = header(payload);
        l.write(layout);
        s.write(payload_bytes.max(1));
        r.write(1); // count 1, black, unbuffered
        payload
    }
}

/// Children of `payload` per its layout word (D-057). The visitor
/// receives each traced managed child pointer (nonzero).
unsafe fn for_each_child(payload: *mut u8, mut visit: impl FnMut(*mut u8)) {
    unsafe {
        let (l, s, _) = header(payload);
        let layout = *l;
        let size = *s;
        let mut visit_word = |word: u64| {
            if word != 0 {
                visit(word as usize as *mut u8);
            }
        };
        match layout & 0b11 {
            1 => {
                // TABLE_SHELL: [cap, count, slots*, meta]; slots hold
                // {state, ktag, kpay, vtag, vpay}; keys AND values.
                let fields = payload as *const i64;
                let cap = *fields;
                let slots = *fields.add(2) as usize as *const i64;
                let meta = *fields.add(3);
                visit_word(meta as u64);
                if !slots.is_null() {
                    for index in 0..cap {
                        let slot = slots.add((index * 5) as usize);
                        if *slot != 1 {
                            continue; // empty or tombstone
                        }
                        for (tag, pay) in [(*slot.add(1), *slot.add(2)), (*slot.add(3), *slot.add(4))] {
                            if tag == 4 || tag == 5 {
                                visit_word(pay as u64);
                            }
                        }
                    }
                }
            }
            2 => {
                // ARRAY: [len, elems...]; element code in bits 2..3.
                let code = (layout >> 2) & 0b11;
                if code == 0 {
                    return;
                }
                let words = payload as *const i64;
                let length = *words;
                match code {
                    1 => {
                        for index in 0..length {
                            visit_word(*words.add((1 + index) as usize) as u64);
                        }
                    }
                    2 => {
                        // dyn pairs would occupy 2 words each — not
                        // yet emitted by any frontend; trace defensively.
                        let mut index = 0;
                        while index + 1 < length * 2 {
                            let tag = *words.add((1 + index) as usize);
                            let pay = *words.add((2 + index) as usize);
                            if tag == 4 || tag == 5 {
                                visit_word(pay as u64);
                            }
                            index += 2;
                        }
                    }
                    _ => {}
                }
            }
            _ => {
                // WORDMAP over payload words 0..min(30, size/8).
                let words = payload as *const i64;
                let word_count = ((size / 8) as usize).min(30);
                let mut index = 0;
                while index < word_count {
                    let code = (layout >> (4 + 2 * index)) & 0b11;
                    match code {
                        1 => {
                            visit_word(*words.add(index) as u64);
                            index += 1;
                        }
                        2 => {
                            let tag = *words.add(index);
                            if index + 1 < word_count && (tag == 4 || tag == 5) {
                                visit_word(*words.add(index + 1) as u64);
                            }
                            index += 2;
                        }
                        _ => index += 1,
                    }
                }
            }
        }
    }
}

unsafe fn free_object(payload: *mut u8) {
    unsafe {
        let (l, s, _) = header(payload);
        if *l & 0b11 == 1 {
            // Table shells own their malloc'd slot array (D-056 debt).
            let fields = payload as *const i64;
            let slots = *fields.add(2) as usize as *mut u8;
            if !slots.is_null() {
                libc_free(slots);
            }
        }
        let total = (*s + 24) as usize;
        let base = payload.sub(24);
        std::alloc::dealloc(base, Layout::from_size_align(total, 8).expect("layout"));
        FREES.fetch_add(1, Ordering::Relaxed);
    }
}

/// The C twin's tables malloc their slots; the Rust twin mirrors with
/// the global allocator through the same 8-aligned discipline. The
/// table code in this twin uses raw_alloc, so "free" must match it.
unsafe fn libc_free(pointer: *mut u8) {
    // raw_alloc has no size record; table slot arrays record their
    // capacity in the shell, but the shell is already being freed —
    // recompute from the slot layout is not possible here, so slot
    // arrays are allocated with a size prefix (see table_grow).
    unsafe {
        let base = pointer.sub(8);
        let bytes = *(base as *const u64);
        std::alloc::dealloc(
            base,
            Layout::from_size_align((bytes + 8) as usize, 8).expect("layout"),
        );
    }
}

/// # Safety
/// `payload` must be a live pointer from [`frk_rt_rc_alloc`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn frk_rt_rc_retain(payload: *mut u8) {
    if payload.is_null() {
        return;
    }
    unsafe {
        let (_, _, r) = header(payload);
        *r = with_color(*r + 1, BLACK);
    }
}

/// # Safety
/// `payload` must be a live pointer from [`frk_rt_rc_alloc`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn frk_rt_rc_release(payload: *mut u8) {
    if payload.is_null() {
        return;
    }
    RELEASES.fetch_add(1, Ordering::Relaxed);
    unsafe { release_inner(payload) }
}

unsafe fn release_inner(payload: *mut u8) {
    unsafe {
        let (l, _, r) = header(payload);
        let rcword = *r;
        let count = count_of(rcword) - 1;
        if count == 0 {
            // Release cascade, then free — unless the collector holds
            // a buffered reference (it will free at markRoots).
            *r = with_color(count | (rcword & BUFFERED_BIT), BLACK);
            if rcword & BUFFERED_BIT == 0 {
                for_each_child(payload, |child| release_inner(child));
                free_object(payload);
            }
        } else {
            // Possibly a cycle root: buffer non-leaf objects purple.
            let leaf = *l == LAYOUT_LEAF;
            let mut next = with_color(count | (rcword & BUFFERED_BIT), PURPLE);
            if leaf {
                *r = with_color(count | (rcword & BUFFERED_BIT), BLACK);
            } else {
                if rcword & BUFFERED_BIT == 0 {
                    next |= BUFFERED_BIT;
                    candidates().lock().expect("buffer").push(payload as usize);
                }
                *r = next;
            }
        }
    }
}

/// The explicit cycle collection entry (D-057: deterministic trigger;
/// automatic thresholds are the deferred last rung).
///
/// # Safety
/// All buffered pointers originate from this runtime.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn frk_rt_rc_collect() {
    unsafe {
        let roots: Vec<usize> = std::mem::take(&mut *candidates().lock().expect("buffer"));
        // markRoots
        let mut live_roots = Vec::new();
        for &root in &roots {
            let payload = root as *mut u8;
            let (_, _, r) = header(payload);
            let rcword = *r;
            if color_of(rcword) == PURPLE && count_of(rcword) > 0 {
                mark_gray(payload);
                live_roots.push(root);
            } else {
                *r &= !BUFFERED_BIT;
                // Bacon–Rajan's guard: only BLACK count-0 objects are
                // deferred frees; a GRAY count-0 here is a TRIAL zero
                // from a sibling root's mark phase — not ours to free.
                if color_of(rcword) == BLACK && count_of(rcword) == 0 {
                    for_each_child(payload, |child| release_inner(child));
                    free_object(payload);
                }
            }
        }
        // scanRoots
        for &root in &live_roots {
            scan(root as *mut u8);
        }
        // collectRoots
        for &root in &live_roots {
            let payload = root as *mut u8;
            let (_, _, r) = header(payload);
            *r &= !BUFFERED_BIT;
            collect_white(payload);
        }
    }
}

unsafe fn mark_gray(payload: *mut u8) {
    unsafe {
        let (_, _, r) = header(payload);
        if color_of(*r) != GRAY {
            *r = with_color(*r, GRAY);
            for_each_child(payload, |child| {
                let (_, _, cr) = header(child);
                *cr -= 1; // trial decrement
                mark_gray(child);
            });
        }
    }
}

unsafe fn scan(payload: *mut u8) {
    unsafe {
        let (_, _, r) = header(payload);
        if color_of(*r) == GRAY {
            if count_of(*r) > 0 {
                scan_black(payload);
            } else {
                *r = with_color(*r, WHITE);
                for_each_child(payload, |child| scan(child));
            }
        }
    }
}

unsafe fn scan_black(payload: *mut u8) {
    unsafe {
        let (_, _, r) = header(payload);
        *r = with_color(*r, BLACK);
        for_each_child(payload, |child| {
            let (_, _, cr) = header(child);
            *cr += 1; // restore
            if color_of(*cr) != BLACK {
                scan_black(child);
            }
        });
    }
}

unsafe fn collect_white(payload: *mut u8) {
    unsafe {
        let (_, _, r) = header(payload);
        if color_of(*r) == WHITE && (*r & BUFFERED_BIT) == 0 {
            *r = with_color(*r, BLACK);
            for_each_child(payload, |child| collect_white(child));
            free_object(payload);
        }
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

/// Lua runtime errors (D-056 helpers): print and abort — the native
/// analog of the interpreter's located traps. Codes are the fenced
/// operations (1 = tostring on table/function, 2 = concat on a
/// non-concatenable, 3 = length of a non-string/table, 4 = arithmetic
/// coercion, 5 = attempt to index a non-table).
#[unsafe(no_mangle)]
pub extern "C" fn frk_rt_lua_error(code: i64) {
    eprintln!("frk: lua runtime error {code} (D-056)");
    std::process::abort();
}

// ---- control effects (M15; κ_frk, D-060): the result-passing
// carrier for escape continuations. NO unwinder (Tier-0/wasm) — abort
// sets a process-global "pending cell", every non-tail caller checks
// it after a call and returns, until the matching prompt resolves it.
// Single-threaded per run (like every other twin global). A well-formed
// program leaves pending=0 and the prompt stack empty, so state never
// leaks between goldens sharing a JIT process. ----

/// Monotonic prompt-token source — never reused within a run, so a
/// stale escape can never alias a fresh prompt (no ABA).
static CTL_NEXT_TOKEN: AtomicU64 = AtomicU64::new(1);
/// The single in-flight abort: flag, target token, and the 2-word dyn
/// value {tag, payload}. Only one abort unwinds at a time.
static CTL_PENDING: AtomicU64 = AtomicU64::new(0);
static CTL_TARGET: AtomicU64 = AtomicU64::new(0);
static CTL_VALUE_TAG: AtomicU64 = AtomicU64::new(0);
static CTL_VALUE_PAYLOAD: AtomicU64 = AtomicU64::new(0);

fn ctl_prompts() -> &'static std::sync::Mutex<Vec<i64>> {
    static PROMPTS: std::sync::OnceLock<std::sync::Mutex<Vec<i64>>> =
        std::sync::OnceLock::new();
    PROMPTS.get_or_init(|| std::sync::Mutex::new(Vec::new()))
}

/// Installs a fresh prompt and returns its token (κ_frk §2).
#[unsafe(no_mangle)]
pub extern "C" fn frk_rt_ctl_prompt_enter() -> i64 {
    let token = CTL_NEXT_TOKEN.fetch_add(1, Ordering::Relaxed) as i64;
    ctl_prompts().lock().unwrap().push(token);
    token
}

/// Removes `token` and anything nested above it (LIFO; the truncate is
/// defensive — a well-typed run pops the exact top).
#[unsafe(no_mangle)]
pub extern "C" fn frk_rt_ctl_prompt_exit(token: i64) {
    let mut prompts = ctl_prompts().lock().unwrap();
    if let Some(position) = prompts.iter().rposition(|&t| t == token) {
        prompts.truncate(position);
    }
}

/// Raises an abort toward `token`. A dead token is the "escape past
/// extent" trap (the native analog of the interpreter's Trap). Live:
/// park the value and set pending; the caller chain propagates.
#[unsafe(no_mangle)]
pub extern "C" fn frk_rt_ctl_abort(token: i64, tag: i64, payload: i64) {
    if !ctl_prompts().lock().unwrap().contains(&token) {
        eprintln!("frk: escape past extent (κ_frk, D-060)");
        std::process::abort();
    }
    CTL_TARGET.store(token as u64, Ordering::Relaxed);
    CTL_VALUE_TAG.store(tag as u64, Ordering::Relaxed);
    CTL_VALUE_PAYLOAD.store(payload as u64, Ordering::Relaxed);
    CTL_PENDING.store(1, Ordering::Relaxed);
}

/// The result-passing carrier read after a call: is an abort pending?
#[unsafe(no_mangle)]
pub extern "C" fn frk_rt_ctl_pending() -> i64 {
    CTL_PENDING.load(Ordering::Relaxed) as i64
}

/// The prompt's catch test: if an abort targets `token`, clear pending,
/// write the parked dyn to `out` (out[0]=tag, out[1]=payload), and
/// return 1; otherwise return 0 (leave pending for an outer prompt).
///
/// # Safety
/// `out` must point to two writable i64 slots when the return is 1.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn frk_rt_ctl_resolve(token: i64, out: *mut i64) -> i64 {
    if CTL_PENDING.load(Ordering::Relaxed) == 0
        || CTL_TARGET.load(Ordering::Relaxed) as i64 != token
    {
        return 0;
    }
    CTL_PENDING.store(0, Ordering::Relaxed);
    unsafe {
        *out = CTL_VALUE_TAG.load(Ordering::Relaxed) as i64;
        *out.add(1) = CTL_VALUE_PAYLOAD.load(Ordering::Relaxed) as i64;
    }
    1
}

// ---- tables (M11 bar 3; D-056): pure-hash dyn-keyed maps. The
// 4-word object shell [cap, count, slots, meta] is STRATEGY-allocated
// by the lowering (rc headers work); slots are rt-malloc'd until the
// layout-descriptor rung adds destructors. All-i64 ABI; dyn results
// return through a caller-provided out pointer (D-056.3). ----

const TABLE_EMPTY: i64 = 0;
const TABLE_FULL: i64 = 1;
const TABLE_TOMB: i64 = 2;

#[repr(C)]
#[derive(Clone, Copy)]
struct TableSlot {
    state: i64,
    ktag: i64,
    kpay: i64,
    vtag: i64,
    vpay: i64,
}

unsafe fn table_fields<'a>(shell: i64) -> &'a mut [i64; 4] {
    unsafe { &mut *(shell as usize as *mut [i64; 4]) }
}

unsafe fn table_slots<'a>(fields: &[i64; 4]) -> &'a mut [TableSlot] {
    unsafe {
        std::slice::from_raw_parts_mut(fields[2] as usize as *mut TableSlot, fields[0] as usize)
    }
}

fn table_hash(ktag: i64, kpay: i64) -> u64 {
    // splitmix-style scramble over both words.
    let mut h = (ktag as u64).wrapping_mul(0x9E3779B97F4A7C15) ^ (kpay as u64);
    h ^= h >> 30;
    h = h.wrapping_mul(0xBF58476D1CE4E5B9);
    h ^= h >> 27;
    h
}

#[unsafe(no_mangle)]
pub extern "C" fn frk_rt_table_init(shell: i64) {
    unsafe {
        *table_fields(shell) = [0, 0, 0, 0];
    }
}

unsafe fn table_grow(shell: i64) {
    unsafe {
        let fields = table_fields(shell);
        let old_cap = fields[0] as usize;
        let old_slots_ptr = fields[2];
        let new_cap = if old_cap == 0 { 8 } else { old_cap * 2 };
        let new_bytes = new_cap * std::mem::size_of::<TableSlot>();
        // Size-prefixed (8 bytes) so the collector's free_object can
        // dealloc slot arrays without a side record (D-057).
        let base = raw_alloc(new_bytes as u64 + 8);
        (base as *mut u64).write(new_bytes as u64);
        let new_ptr = base.add(8);
        std::ptr::write_bytes(new_ptr, 0, new_bytes);
        let old_fields = *fields;
        fields[0] = new_cap as i64;
        fields[1] = 0;
        fields[2] = new_ptr as i64;
        if old_cap > 0 {
            let old =
                std::slice::from_raw_parts(old_slots_ptr as usize as *const TableSlot, old_cap);
            for slot in old {
                if slot.state == TABLE_FULL {
                    frk_rt_table_raw_set(shell, slot.ktag, slot.kpay, slot.vtag, slot.vpay);
                }
            }
            libc_free(old_slots_ptr as usize as *mut u8);
        }
        let _ = old_fields;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn frk_rt_table_raw_set(shell: i64, ktag: i64, kpay: i64, vtag: i64, vpay: i64) {
    unsafe {
        let fields = table_fields(shell);
        if fields[0] == 0 || fields[1] * 10 >= fields[0] * 7 {
            table_grow(shell);
        }
        let fields = table_fields(shell);
        let cap = fields[0] as u64;
        let slots = table_slots(fields);
        let mask = cap - 1;
        let mut index = (table_hash(ktag, kpay) & mask) as usize;
        let mut first_tomb: Option<usize> = None;
        loop {
            let slot = slots[index];
            match slot.state {
                s if s == TABLE_FULL && slot.ktag == ktag && slot.kpay == kpay => {
                    if vtag == 0 {
                        slots[index].state = TABLE_TOMB; // nil deletes
                    } else {
                        slots[index].vtag = vtag;
                        slots[index].vpay = vpay;
                    }
                    return;
                }
                s if s == TABLE_TOMB => {
                    if first_tomb.is_none() {
                        first_tomb = Some(index);
                    }
                }
                s if s == TABLE_EMPTY => {
                    if vtag == 0 {
                        return; // deleting an absent key: no-op
                    }
                    let target = first_tomb.unwrap_or(index);
                    slots[target] = TableSlot { state: TABLE_FULL, ktag, kpay, vtag, vpay };
                    fields[1] += 1;
                    return;
                }
                _ => {}
            }
            index = (index + 1) & mask as usize;
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn frk_rt_table_raw_get(shell: i64, ktag: i64, kpay: i64, out: *mut i64) {
    unsafe {
        let fields = table_fields(shell);
        if fields[0] != 0 {
            let cap = fields[0] as u64;
            let slots = table_slots(fields);
            let mask = cap - 1;
            let mut index = (table_hash(ktag, kpay) & mask) as usize;
            loop {
                let slot = slots[index];
                if slot.state == TABLE_EMPTY {
                    break;
                }
                if slot.state == TABLE_FULL && slot.ktag == ktag && slot.kpay == kpay {
                    out.write(slot.vtag);
                    out.add(1).write(slot.vpay);
                    return;
                }
                index = (index + 1) & mask as usize;
            }
        }
        out.write(0); // nil
        out.add(1).write(0);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn frk_rt_table_len(shell: i64) -> i64 {
    // The border probe (D-056): # = largest n with t[1..n] present.
    let mut out = [0i64; 2];
    let mut length: i64 = 0;
    loop {
        let probe = ((length + 1) as f64).to_bits() as i64;
        frk_rt_table_raw_get(shell, 2, probe, out.as_mut_ptr());
        if out[0] == 0 {
            return length;
        }
        length += 1;
    }
}

/// Iteration for pairs/next (D-058): scans slots from the given
/// key's position + 1 (or 0 for a nil start key, tag 0). Slot order —
/// deterministic for OUR tables, implementation-defined per Lua and
/// canon. Writes {ktag, kpay, vtag, vpay} to out; nil key at end.
#[unsafe(no_mangle)]
pub extern "C" fn frk_rt_table_next(
    shell: i64,
    ktag: i64,
    kpay: i64,
    out: *mut i64,
) {
    unsafe {
        let fields = table_fields(shell);
        let cap = fields[0] as usize;
        let start = if ktag == 0 {
            0
        } else {
            let slots = table_slots(fields);
            let mask = cap as u64 - 1;
            let mut index = (table_hash(ktag, kpay) & mask) as usize;
            let mut found = None;
            loop {
                let slot = slots[index];
                if slot.state == TABLE_EMPTY {
                    break;
                }
                if slot.state == TABLE_FULL && slot.ktag == ktag && slot.kpay == kpay {
                    found = Some(index + 1);
                    break;
                }
                index = (index + 1) & mask as usize;
            }
            match found {
                Some(next) => next,
                None => cap, // invalid key: end (lua errors; we end)
            }
        };
        if cap > 0 {
            let slots = table_slots(fields);
            for index in start..cap {
                let slot = slots[index];
                if slot.state == TABLE_FULL {
                    out.write(slot.ktag);
                    out.add(1).write(slot.kpay);
                    out.add(2).write(slot.vtag);
                    out.add(3).write(slot.vpay);
                    return;
                }
            }
        }
        out.write(0);
        out.add(1).write(0);
        out.add(2).write(0);
        out.add(3).write(0);
    }
}

/// Lua string.sub (D-058): 1-based, negative-from-end, clamped.
///
/// # Safety
/// `s` must be a canonical byte string from this runtime.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn frk_rt_bstr_sub(s: *const u8, from: i64, to: i64) -> *mut u8 {
    let bytes = unsafe { bstr_parts(s) };
    let length = bytes.len() as i64;
    let mut i = if from < 0 { length + from + 1 } else { from };
    let mut j = if to < 0 { length + to + 1 } else { to };
    if i < 1 {
        i = 1;
    }
    if j > length {
        j = length;
    }
    if i > j {
        return bstr_intern_bytes(&[]);
    }
    bstr_intern_bytes(&bytes[(i - 1) as usize..j as usize])
}

/// # Safety
/// `s` must be a canonical byte string from this runtime.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn frk_rt_bstr_rep(s: *const u8, count: i64) -> *mut u8 {
    let bytes = unsafe { bstr_parts(s) };
    let count = count.max(0) as usize;
    let mut out = Vec::with_capacity(bytes.len() * count);
    for _ in 0..count {
        out.extend_from_slice(bytes);
    }
    bstr_intern_bytes(&out)
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
    unsafe { count_of(*(payload.sub(8) as *const i64)) }
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
        let p = frk_rt_rc_alloc(16, LAYOUT_LEAF);
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
        unsafe {
            frk_rt_rc_release(p);
            frk_rt_rc_release(p); // to zero: leaf frees immediately
        }
    }

    #[test]
    fn release_cascade_frees_transitively() {
        // inner <- outer (outer's word 0 is a managed pointer).
        let frees_before = frk_rt_rc_free_count();
        let inner = frk_rt_rc_alloc(8, LAYOUT_LEAF);
        let outer = frk_rt_rc_alloc(8, layout_wordmap(&[1]));
        unsafe {
            (outer as *mut u64).write(inner as u64);
            frk_rt_rc_release(outer); // count 1 -> 0: cascade
        }
        assert_eq!(
            frk_rt_rc_free_count() - frees_before,
            2,
            "outer AND inner freed by the cascade"
        );
    }

    #[test]
    fn trial_deletion_dead_and_live_cycles() {
        // The candidate buffer and counters are process-global; this
        // test owns the whole cycle story SEQUENTIALLY (cargo runs
        // tests in parallel — two collect() calls would steal each
        // other's roots).
        unsafe { frk_rt_rc_collect() }; // drain any prior candidates

        // Dead cycle: a <-> b, externals dropped — pure rc never
        // frees it; trial deletion must.
        let frees_before = frk_rt_rc_free_count();
        let a = frk_rt_rc_alloc(8, layout_wordmap(&[1]));
        let b = frk_rt_rc_alloc(8, layout_wordmap(&[1]));
        unsafe {
            (a as *mut u64).write(b as u64);
            frk_rt_rc_retain(b);
            (b as *mut u64).write(a as u64);
            frk_rt_rc_retain(a);
            frk_rt_rc_release(a);
            frk_rt_rc_release(b);
        }
        assert_eq!(frk_rt_rc_free_count(), frees_before, "cycle survives rc");
        assert_eq!(rc_count(a), 1);
        unsafe { frk_rt_rc_collect() };
        assert_eq!(
            frk_rt_rc_free_count() - frees_before,
            2,
            "trial deletion collects the dead cycle"
        );

        // Live cycle: same shape, ONE external stays — collect must
        // restore counts and free nothing.
        let frees_mid = frk_rt_rc_free_count();
        let c = frk_rt_rc_alloc(8, layout_wordmap(&[1]));
        let d = frk_rt_rc_alloc(8, layout_wordmap(&[1]));
        unsafe {
            (c as *mut u64).write(d as u64);
            frk_rt_rc_retain(d);
            (d as *mut u64).write(c as u64);
            frk_rt_rc_retain(c);
            frk_rt_rc_release(d); // d's external gone; c keeps its own
        }
        unsafe { frk_rt_rc_collect() };
        assert_eq!(frk_rt_rc_free_count(), frees_mid, "live cycle survives");
        assert_eq!(rc_count(c), 2, "counts restored by scan_black");
        assert_eq!(rc_count(d), 1);
        unsafe {
            frk_rt_rc_release(c);
            frk_rt_rc_collect();
        }
        assert_eq!(frk_rt_rc_free_count() - frees_mid, 2, "then it dies");
    }

    #[test]
    fn tables_upsert_delete_probe_and_border() {
        // A shell on the heap, as the lowering would allocate it.
        let shell_ptr = frk_rt_arena_alloc(32) as i64;
        frk_rt_table_init(shell_ptr);
        let mut out = [0i64; 2];
        let key = |n: f64| (2i64, n.to_bits() as i64);

        // Missing → nil.
        let (kt, kp) = key(1.0);
        frk_rt_table_raw_get(shell_ptr, kt, kp, out.as_mut_ptr());
        assert_eq!(out[0], 0);

        // 1..=3 present → border 3; delete 2 → border 1.
        for n in 1..=3 {
            let (kt, kp) = key(n as f64);
            frk_rt_table_raw_set(shell_ptr, kt, kp, 2, (n as f64 * 10.0).to_bits() as i64);
        }
        assert_eq!(frk_rt_table_len(shell_ptr), 3);
        let (kt, kp) = key(2.0);
        frk_rt_table_raw_get(shell_ptr, kt, kp, out.as_mut_ptr());
        assert_eq!(f64::from_bits(out[1] as u64), 20.0);
        frk_rt_table_raw_set(shell_ptr, kt, kp, 0, 0); // nil deletes
        frk_rt_table_raw_get(shell_ptr, kt, kp, out.as_mut_ptr());
        assert_eq!(out[0], 0);
        assert_eq!(frk_rt_table_len(shell_ptr), 1);

        // Growth: 100 string-ish keys (tag 3, fake ptrs) survive.
        for i in 0..100i64 {
            frk_rt_table_raw_set(shell_ptr, 3, 0x1000 + i, 1, 1);
        }
        frk_rt_table_raw_get(shell_ptr, 3, 0x1050, out.as_mut_ptr());
        assert_eq!(out[0], 1);
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
        let _ = frk_rt_rc_alloc(8, LAYOUT_LEAF);
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
        let p = frk_rt_rc_alloc(0, LAYOUT_LEAF);
        assert!(!p.is_null());
        assert_eq!(rc_count(p), 1);
    }
}
