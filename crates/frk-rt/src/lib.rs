//! frk-rt — the runtime component library behind a documented C ABI,
//! freestanding-first so Tier 0 stays as wide as every LLVM triple
//! (SPEC §10, contract point K4).
//!
//! First real component lands with frk.mem (M7). The crate builds as
//! rlib + staticlib from day one so linking is exercised early; it goes
//! `#![no_std]` when the first Tier-0 component arrives.
