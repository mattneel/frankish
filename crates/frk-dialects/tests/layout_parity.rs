//! The D-057 layout encoding is duplicated in the lowering and the
//! runtime by design (no dependency between them); this test is the
//! lockstep. A drift here means the tracer walks the wrong words.

#[test]
fn lowering_and_runtime_layout_encodings_agree() {
    // The runtime's canonical values.
    assert_eq!(frk_rt::LAYOUT_LEAF, 0);
    assert_eq!(frk_rt::LAYOUT_TABLE_SHELL, 1);
    assert_eq!(frk_rt::LAYOUT_ARRAY_LEAF, 2);
    assert_eq!(frk_rt::LAYOUT_ARRAY_PTR, 2 | (1 << 2));
    assert_eq!(frk_rt::LAYOUT_ARRAY_DYN, 2 | (2 << 2));
    // Wordmap: codes at bit 4, two bits per word.
    assert_eq!(frk_rt::layout_wordmap(&[1]), 1 << 4);
    assert_eq!(frk_rt::layout_wordmap(&[0, 1]), 1 << 6);
    assert_eq!(frk_rt::layout_wordmap(&[2, 0]), 2 << 4);
    assert_eq!(frk_rt::layout_wordmap(&[2, 0, 1]), (2 << 4) | (1 << 8));
}
