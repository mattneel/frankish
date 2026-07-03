//! frk-harness — golden runner, differential runner, stage dumps, and the
//! conformance dashboard emitter (SPEC §7). Goldens are byte-exact after
//! canonicalization (law L2); the contract is docs/canon.md and its only
//! implementation is [`canon`].

pub mod canon;
