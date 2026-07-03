//! Matrix→tree goldens for the decision-tree pass (D-025: "its own
//! goldens over the matrix→tree mapping"; format ruled in D-034). The
//! expected renderings are byte-exact: a heuristic or rendering change
//! re-blesses these strings with an L2 justification in the commit.

use frk_dialects::adt_dtree::{
    CompiledMatch, Matrix, Pattern, PatternAnalysis, Row, TreeDerived, ValueType, compile,
};

fn option_i64() -> ValueType {
    ValueType::Sum(vec![vec![], vec![ValueType::Int]])
}

fn bind(name: &str) -> Pattern {
    Pattern::Binding(name.to_string())
}

fn compiled(scrutinee: ValueType, arms: Vec<Pattern>) -> CompiledMatch {
    compile(Matrix::over_scrutinee(scrutinee, arms)).expect("compilation must succeed")
}

#[test]
fn option_unwrap_maps_to_a_complete_tag_switch() {
    let result = compiled(
        option_i64(),
        vec![
            Pattern::Variant { tag: 1, fields: vec![bind("x")] },
            Pattern::Variant { tag: 0, fields: vec![] },
        ],
    );
    assert_eq!(
        result.tree.to_string(),
        "\
switch-tag $
  case v0:
    leaf arm=1
  case v1:
    leaf arm=0 x=$.v1f0"
    );
    assert!(result.diagnostics.inexhaustive.is_none());
    assert!(result.diagnostics.redundant_arms.is_empty());
}

#[test]
fn nested_options_switch_on_the_inner_occurrence() {
    let option_option = ValueType::Sum(vec![vec![], vec![option_i64()]]);
    let result = compiled(
        option_option,
        vec![
            // Some(Some(x)) → 0; Some(None) → 1; None → 2
            Pattern::Variant {
                tag: 1,
                fields: vec![Pattern::Variant { tag: 1, fields: vec![bind("x")] }],
            },
            Pattern::Variant {
                tag: 1,
                fields: vec![Pattern::Variant { tag: 0, fields: vec![] }],
            },
            Pattern::Variant { tag: 0, fields: vec![] },
        ],
    );
    assert_eq!(
        result.tree.to_string(),
        "\
switch-tag $
  case v0:
    leaf arm=2
  case v1:
    switch-tag $.v1f0
      case v0:
        leaf arm=1
      case v1:
        leaf arm=0 x=$.v1f0.v1f0"
    );
    assert!(result.diagnostics.inexhaustive.is_none());
    assert!(result.diagnostics.redundant_arms.is_empty());
}

#[test]
fn integer_literals_switch_with_a_default_and_expose_redundancy() {
    let result = compiled(
        ValueType::Int,
        vec![
            Pattern::Int(0),
            Pattern::Int(42),
            Pattern::Wildcard,
            // Duplicate literal: unreachable, must be reported.
            Pattern::Int(42),
        ],
    );
    assert_eq!(
        result.tree.to_string(),
        "\
switch-int $
  case 0:
    leaf arm=0
  case 42:
    leaf arm=1
  default:
    leaf arm=2"
    );
    assert!(result.diagnostics.inexhaustive.is_none());
    assert_eq!(result.diagnostics.redundant_arms, vec![3]);
}

#[test]
fn products_unpack_without_a_switch_node() {
    let pair = ValueType::Product(vec![ValueType::Int, ValueType::Int]);
    let result = compiled(
        pair,
        vec![
            Pattern::Product(vec![Pattern::Int(42), bind("y")]),
            Pattern::Product(vec![bind("a"), bind("b")]),
        ],
    );
    assert_eq!(
        result.tree.to_string(),
        "\
switch-int $.p0
  case 42:
    leaf arm=0 y=$.p1
  default:
    leaf arm=1 a=$.p0 b=$.p1"
    );
    assert!(result.diagnostics.inexhaustive.is_none());
    assert!(result.diagnostics.redundant_arms.is_empty());
}

#[test]
fn incomplete_tag_coverage_yields_a_default_and_a_witness() {
    let result = compiled(
        option_i64(),
        vec![Pattern::Variant { tag: 1, fields: vec![Pattern::Wildcard] }],
    );
    assert_eq!(
        result.tree.to_string(),
        "\
switch-tag $
  case v1:
    leaf arm=0
  default:
    FAIL"
    );
    let witness = result.diagnostics.inexhaustive.expect("must be inexhaustive");
    assert_eq!(witness.to_string(), "$ is any variant not in {v1}");
}

#[test]
fn wildcard_first_row_short_circuits_and_shadows_everything() {
    let result = compiled(option_i64(), vec![bind("whole"), Pattern::Variant { tag: 0, fields: vec![] }]);
    assert_eq!(result.tree.to_string(), "leaf arm=0 whole=$");
    assert_eq!(result.diagnostics.redundant_arms, vec![1]);
}

#[test]
fn heuristic_picks_the_first_rows_leftmost_constructor_column() {
    // Two columns; the first row is irrefutable in column 0 but
    // constructs in column 1 — the switch must land on column 1.
    let matrix = Matrix {
        columns: vec![(vec![], ValueType::Int), (vec![], option_i64())],
        rows: vec![
            Row {
                patterns: vec![bind("n"), Pattern::Variant { tag: 0, fields: vec![] }],
                arm: 0,
            },
            Row { patterns: vec![Pattern::Wildcard, Pattern::Wildcard], arm: 1 },
        ],
    };
    let result = compile(matrix).unwrap();
    assert_eq!(
        result.tree.to_string(),
        "\
switch-tag $
  case v0:
    leaf arm=0 n=$
  default:
    leaf arm=1"
    );
}

#[test]
fn deep_mixed_nesting_composes_every_mechanism() {
    // Sum over a product payload: variant 1 carries (Int, Option<Int>).
    let scrutinee = ValueType::Sum(vec![
        vec![],
        vec![ValueType::Product(vec![ValueType::Int, option_i64()])],
    ]);
    let result = compiled(
        scrutinee,
        vec![
            // V1((0, Some(x))) → 0
            Pattern::Variant {
                tag: 1,
                fields: vec![Pattern::Product(vec![
                    Pattern::Int(0),
                    Pattern::Variant { tag: 1, fields: vec![bind("x")] },
                ])],
            },
            // V1((n, _)) → 1
            Pattern::Variant {
                tag: 1,
                fields: vec![Pattern::Product(vec![bind("n"), Pattern::Wildcard])],
            },
            // V0 → 2
            Pattern::Variant { tag: 0, fields: vec![] },
        ],
    );
    // Note the inner switch: only v1 appears as a head constructor
    // there, so Maranget covers v0 through the default rather than
    // enumerating it — fewer cases, same semantics.
    assert_eq!(
        result.tree.to_string(),
        "\
switch-tag $
  case v0:
    leaf arm=2
  case v1:
    switch-int $.v1f0.p0
      case 0:
        switch-tag $.v1f0.p1
          case v1:
            leaf arm=0 x=$.v1f0.p1.v1f0
          default:
            leaf arm=1 n=$.v1f0.p0
      default:
        leaf arm=1 n=$.v1f0.p0"
    );
    assert!(result.diagnostics.inexhaustive.is_none());
    assert!(result.diagnostics.redundant_arms.is_empty());
}

#[test]
fn malformed_matrices_are_errors() {
    // Ragged row.
    let ragged = Matrix {
        columns: vec![(vec![], ValueType::Int)],
        rows: vec![Row { patterns: vec![], arm: 0 }],
    };
    assert!(compile(ragged).is_err());

    // Variant tag out of range.
    let bad_tag = Matrix::over_scrutinee(
        option_i64(),
        vec![Pattern::Variant { tag: 7, fields: vec![] }],
    );
    assert!(compile(bad_tag).is_err());

    // Int pattern against a sum column.
    let wrong_kind = Matrix::over_scrutinee(option_i64(), vec![Pattern::Int(1)]);
    assert!(compile(wrong_kind).is_err());

    // Wrong constructor arity.
    let bad_arity = Matrix::over_scrutinee(
        option_i64(),
        vec![Pattern::Variant { tag: 1, fields: vec![] }],
    );
    assert!(compile(bad_arity).is_err());
}

#[test]
fn the_analysis_trait_boundary_reports_without_a_tree_in_hand() {
    let matrix = Matrix::over_scrutinee(
        option_i64(),
        vec![Pattern::Variant { tag: 0, fields: vec![] }],
    );
    let diagnostics = TreeDerived.analyze(&matrix).unwrap();
    assert!(diagnostics.inexhaustive.is_some());
    assert!(diagnostics.redundant_arms.is_empty());
}
