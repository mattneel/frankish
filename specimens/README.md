# Specimen discipline

Specimens are idiom carriers, not products. Each is a **named, versioned,
frozen subset** of a real language, pinned against a specific upstream
release, with a vendored conformance corpus and an explicit fence list.
The MANIFEST in each directory is that specimen's law (constitution L5).

Rules:
- **Admission rule.** No feature enters a subset unless it carries an idiom
  the kernel dialect library lacks. Fence lists are not TODO lists.
- **Pin the oracle.** Upstream version is frozen in the MANIFEST; vendored
  tests carry their upstream license file; never edit vendored corpora —
  exclusions live in the manifest, not in the files.
- **Triangulate.** upstream ↔ derived interpreter ↔ JIT/AOT, pairwise, over
  the corpus, through the canonicalization filter (SPEC §7.4), per commit.
  Conformance % per runner per target is the dashboard row.
- **Extraction loop.** v0 may cheat (private ops, ad-hoc lowering) to go
  green. The deliverable is the promotion pass: extract into kernel dialects
  under the §3 contract, re-base the specimen, conformance not worse. Write
  the extraction report in STATE.md at milestone close.
- **Parsers are scaffolding.** Borrowed grammars via tree-sitter or upstream
  parsers (D-019); do not spend research effort here.

MANIFEST skeleton: Identity & pin · Role (idioms carried → dialects
exercised) · Scope grammar · Fence list · Conformance sources & exclusions ·
Oracles & canonicalization notes · Exit bars per stage · Status.
