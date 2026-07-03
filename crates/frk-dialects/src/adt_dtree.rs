//! Pattern-match compilation to decision trees — Maranget's algorithm
//! (*Compiling Pattern Matching to Good Decision Trees*; D-025). This is
//! the pass that makes D-031's de-regioned `match` real: a frontend
//! hands over a pattern matrix, this module hands back a decision tree
//! whose nodes are exactly the ops the dialect has — SwitchTag is
//! `tag_of` + `cf.switch`, occurrences are `extract`/`get` chains.
//!
//! v0 scope (D-034): pattern language = variant constructors, product
//! destructuring, integer literals, wildcards, bindings. Compilation is
//! pure (no IR in sight); emission to IR lands with its first consumer
//! (ml_core, M5). Exhaustiveness and usefulness fall out of the tree —
//! a reachable `Fail` node is a counterexample witness, an arm missing
//! from every leaf is redundant — which is complete for this pattern
//! language; rustc_pattern_analysis slots in behind [`PatternAnalysis`]
//! when the language outgrows it (or-patterns, ranges, guards).
//!
//! Column-choice heuristic: leftmost column where the first row has a
//! constructor (Maranget's baseline). D-025: heuristics are free to
//! evolve; the goldens pin the mapping, a heuristic change re-blesses
//! them with an L2 justification.
//!
//! Tree goldens live as literal expected renderings in this module's
//! test suite until a textual matrix format exists (M5) — byte-exact
//! all the same (D-034).

use std::collections::BTreeSet;
use std::fmt;

/// One step of a path from the match scrutinee to a sub-value. Emission
/// maps SumField to `frk_adt.extract {variant, field}` (valid only under
/// that variant's dispatch arm — the tree guarantees it) and
/// ProductField to `frk_adt.get {field}`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Access {
    SumField { variant: usize, field: usize },
    ProductField { field: usize },
}

/// A path from the scrutinee root; empty = the scrutinee itself.
pub type Occurrence = Vec<Access>;

fn render_occurrence(occurrence: &Occurrence) -> String {
    let mut text = String::from("$");
    for access in occurrence {
        match access {
            Access::SumField { variant, field } => {
                text.push_str(&format!(".v{variant}f{field}"));
            }
            Access::ProductField { field } => {
                text.push_str(&format!(".p{field}"));
            }
        }
    }
    text
}

/// The shape of the value a column scrutinizes. Nested freely — the
/// tree layer is representation-agnostic (lowering fences are a
/// different, later concern).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ValueType {
    /// Variants, each a list of field types.
    Sum(Vec<Vec<ValueType>>),
    Product(Vec<ValueType>),
    Int,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Pattern {
    /// Matches anything, binds nothing.
    Wildcard,
    /// Matches anything, binds the value at this position to a name.
    /// Nonlinearity (one name bound twice in a row) is the frontend's
    /// problem; this layer records bindings verbatim.
    Binding(String),
    /// One variant of a sum, with subpatterns for its fields.
    Variant { tag: usize, fields: Vec<Pattern> },
    /// Product destructuring (tuples ARE products).
    Product(Vec<Pattern>),
    /// Integer literal equality.
    Int(i64),
}

impl Pattern {
    fn is_irrefutable(&self) -> bool {
        matches!(self, Self::Wildcard | Self::Binding(_))
    }
}

/// One match arm's row: patterns aligned with the matrix columns.
#[derive(Clone, Debug)]
pub struct Row {
    pub patterns: Vec<Pattern>,
    /// Which source arm this row selects on match.
    pub arm: usize,
}

/// The compilation input: typed columns (occurrence + shape) and rows.
#[derive(Clone, Debug)]
pub struct Matrix {
    pub columns: Vec<(Occurrence, ValueType)>,
    pub rows: Vec<Row>,
}

impl Matrix {
    /// The usual entry: one column (the scrutinee), one row per arm.
    pub fn over_scrutinee(scrutinee: ValueType, arms: Vec<Pattern>) -> Self {
        Self {
            columns: vec![(Vec::new(), scrutinee)],
            rows: arms
                .into_iter()
                .enumerate()
                .map(|(arm, pattern)| Row { patterns: vec![pattern], arm })
                .collect(),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum DecisionTree {
    /// No row matches the values reaching here: the match was
    /// inexhaustive and this path is its counterexample.
    Fail,
    /// Arm selected; bindings give each bound name its occurrence.
    Leaf {
        arm: usize,
        bindings: Vec<(String, Occurrence)>,
    },
    /// Dispatch on a sum tag (emission: tag_of + cf.switch). `default`
    /// is present exactly when `cases` doesn't cover every variant.
    SwitchTag {
        occurrence: Occurrence,
        cases: Vec<(usize, DecisionTree)>,
        default: Option<Box<DecisionTree>>,
    },
    /// Dispatch on integer literals (emission: cf.switch).
    SwitchInt {
        occurrence: Occurrence,
        cases: Vec<(i64, DecisionTree)>,
        default: Box<DecisionTree>,
    },
}

impl DecisionTree {
    fn render(&self, indent: usize, out: &mut String) {
        let pad = "  ".repeat(indent);
        match self {
            Self::Fail => out.push_str(&format!("{pad}FAIL\n")),
            Self::Leaf { arm, bindings } => {
                out.push_str(&format!("{pad}leaf arm={arm}"));
                for (name, occurrence) in bindings {
                    out.push_str(&format!(" {name}={}", render_occurrence(occurrence)));
                }
                out.push('\n');
            }
            Self::SwitchTag { occurrence, cases, default } => {
                out.push_str(&format!(
                    "{pad}switch-tag {}\n",
                    render_occurrence(occurrence)
                ));
                for (tag, subtree) in cases {
                    out.push_str(&format!("{pad}  case v{tag}:\n"));
                    subtree.render(indent + 2, out);
                }
                if let Some(subtree) = default {
                    out.push_str(&format!("{pad}  default:\n"));
                    subtree.render(indent + 2, out);
                }
            }
            Self::SwitchInt { occurrence, cases, default } => {
                out.push_str(&format!(
                    "{pad}switch-int {}\n",
                    render_occurrence(occurrence)
                ));
                for (literal, subtree) in cases {
                    out.push_str(&format!("{pad}  case {literal}:\n"));
                    subtree.render(indent + 2, out);
                }
                out.push_str(&format!("{pad}  default:\n"));
                default.render(indent + 2, out);
            }
        }
    }
}

impl fmt::Display for DecisionTree {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut out = String::new();
        self.render(0, &mut out);
        f.write_str(out.trim_end_matches('\n'))
    }
}

/// A counterexample path: constraints on occurrences that reach FAIL.
#[derive(Debug, PartialEq, Eq)]
pub struct Witness(pub Vec<(Occurrence, String)>);

impl fmt::Display for Witness {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_empty() {
            return f.write_str("any value");
        }
        let parts: Vec<String> = self
            .0
            .iter()
            .map(|(occurrence, constraint)| {
                format!("{} {}", render_occurrence(occurrence), constraint)
            })
            .collect();
        f.write_str(&parts.join(", "))
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct MatchDiagnostics {
    /// Some(witness) iff a FAIL node is reachable — the match misses
    /// the described values.
    pub inexhaustive: Option<Witness>,
    /// Arms that appear in no leaf: no value can select them.
    pub redundant_arms: Vec<usize>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct CompiledMatch {
    pub tree: DecisionTree,
    pub diagnostics: MatchDiagnostics,
}

#[derive(Debug, PartialEq, Eq)]
pub enum DtreeError {
    RaggedRow { row: usize, expected: usize, got: usize },
    /// A pattern that cannot scrutinize its column's type.
    TypeMismatch { row: usize, column: usize, message: String },
}

impl fmt::Display for DtreeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RaggedRow { row, expected, got } => write!(
                f,
                "row {row} has {got} pattern(s), the matrix has {expected} column(s)"
            ),
            Self::TypeMismatch { row, column, message } => {
                write!(f, "row {row}, column {column}: {message}")
            }
        }
    }
}

impl std::error::Error for DtreeError {}

/// The exhaustiveness/usefulness seam (SPEC §4.1, amended by D-034):
/// the type kit consumes this trait; [`TreeDerived`] is the v0 oracle,
/// rustc_pattern_analysis becomes an alternative impl when the pattern
/// language outgrows the tree-derived one.
pub trait PatternAnalysis {
    fn analyze(&self, matrix: &Matrix) -> Result<MatchDiagnostics, DtreeError>;
}

pub struct TreeDerived;

impl PatternAnalysis for TreeDerived {
    fn analyze(&self, matrix: &Matrix) -> Result<MatchDiagnostics, DtreeError> {
        Ok(compile(matrix.clone())?.diagnostics)
    }
}

/// Compiles a pattern matrix to a decision tree and its diagnostics.
pub fn compile(matrix: Matrix) -> Result<CompiledMatch, DtreeError> {
    validate(&matrix)?;
    let arms: Vec<usize> = matrix.rows.iter().map(|row| row.arm).collect();

    let tree = compile_matrix(WorkMatrix::seed(matrix))?;

    let mut leaf_arms = BTreeSet::new();
    collect_leaf_arms(&tree, &mut leaf_arms);
    let redundant_arms = arms
        .iter()
        .copied()
        .filter(|arm| !leaf_arms.contains(arm))
        .collect();

    let mut witness_path = Vec::new();
    let inexhaustive = find_fail(&tree, &mut witness_path).then(|| Witness(witness_path));

    Ok(CompiledMatch {
        tree,
        diagnostics: MatchDiagnostics { inexhaustive, redundant_arms },
    })
}

fn validate(matrix: &Matrix) -> Result<(), DtreeError> {
    for (row_index, row) in matrix.rows.iter().enumerate() {
        if row.patterns.len() != matrix.columns.len() {
            return Err(DtreeError::RaggedRow {
                row: row_index,
                expected: matrix.columns.len(),
                got: row.patterns.len(),
            });
        }
        for (column_index, (_, column_type)) in matrix.columns.iter().enumerate() {
            check_pattern(&row.patterns[column_index], column_type).map_err(|message| {
                DtreeError::TypeMismatch { row: row_index, column: column_index, message }
            })?;
        }
    }
    Ok(())
}

fn check_pattern(pattern: &Pattern, column_type: &ValueType) -> Result<(), String> {
    match (pattern, column_type) {
        (Pattern::Wildcard | Pattern::Binding(_), _) => Ok(()),
        (Pattern::Variant { tag, fields }, ValueType::Sum(variants)) => {
            let Some(field_types) = variants.get(*tag) else {
                return Err(format!(
                    "variant {tag} out of range ({} variant(s))",
                    variants.len()
                ));
            };
            if fields.len() != field_types.len() {
                return Err(format!(
                    "variant {tag} has {} field(s), pattern has {}",
                    field_types.len(),
                    fields.len()
                ));
            }
            for (field, field_type) in fields.iter().zip(field_types) {
                check_pattern(field, field_type)?;
            }
            Ok(())
        }
        (Pattern::Product(fields), ValueType::Product(field_types)) => {
            if fields.len() != field_types.len() {
                return Err(format!(
                    "product has {} field(s), pattern has {}",
                    field_types.len(),
                    fields.len()
                ));
            }
            for (field, field_type) in fields.iter().zip(field_types) {
                check_pattern(field, field_type)?;
            }
            Ok(())
        }
        (Pattern::Int(_), ValueType::Int) => Ok(()),
        (pattern, column_type) => {
            Err(format!("pattern {pattern:?} cannot match {column_type:?}"))
        }
    }
}

/// Internal working state: rows accrue bindings as irrefutable patterns
/// are consumed by specialization.
struct WorkRow {
    patterns: Vec<Pattern>,
    arm: usize,
    bindings: Vec<(String, Occurrence)>,
}

struct WorkMatrix {
    columns: Vec<(Occurrence, ValueType)>,
    rows: Vec<WorkRow>,
}

impl WorkMatrix {
    fn seed(matrix: Matrix) -> Self {
        Self {
            columns: matrix.columns,
            rows: matrix
                .rows
                .into_iter()
                .map(|row| WorkRow { patterns: row.patterns, arm: row.arm, bindings: Vec::new() })
                .collect(),
        }
    }
}

/// Consumes an irrefutable pattern at a removed/expanded column,
/// recording a binding when it names one.
fn absorb_irrefutable(
    pattern: &Pattern,
    occurrence: &Occurrence,
    bindings: &mut Vec<(String, Occurrence)>,
) {
    if let Pattern::Binding(name) = pattern {
        bindings.push((name.clone(), occurrence.clone()));
    }
}

fn compile_matrix(matrix: WorkMatrix) -> Result<DecisionTree, DtreeError> {
    let Some(first) = matrix.rows.first() else {
        return Ok(DecisionTree::Fail);
    };

    // First row irrefutable across all columns → it matches; done.
    if first.patterns.iter().all(Pattern::is_irrefutable) {
        let mut bindings = first.bindings.clone();
        for (pattern, (occurrence, _)) in first.patterns.iter().zip(&matrix.columns) {
            absorb_irrefutable(pattern, occurrence, &mut bindings);
        }
        return Ok(DecisionTree::Leaf { arm: first.arm, bindings });
    }

    // Maranget baseline heuristic: leftmost column where the first row
    // holds a constructor.
    let column = first
        .patterns
        .iter()
        .position(|pattern| !pattern.is_irrefutable())
        .expect("guarded by the irrefutable check above");

    let (occurrence, column_type) = matrix.columns[column].clone();
    match column_type {
        ValueType::Product(field_types) => {
            // Single-constructor type: specialize in place, no node.
            compile_matrix(specialize_product(matrix, column, &occurrence, &field_types))
        }
        ValueType::Sum(variants) => {
            let mut head_tags = BTreeSet::new();
            for row in &matrix.rows {
                if let Pattern::Variant { tag, .. } = &row.patterns[column] {
                    head_tags.insert(*tag);
                }
            }

            let mut cases = Vec::with_capacity(head_tags.len());
            for tag in &head_tags {
                let specialized =
                    specialize_variant(&matrix, column, &occurrence, *tag, &variants[*tag]);
                cases.push((*tag, compile_matrix(specialized)?));
            }

            let default = if head_tags.len() == variants.len() {
                None
            } else {
                Some(Box::new(compile_matrix(default_matrix(&matrix, column, &occurrence))?))
            };

            Ok(DecisionTree::SwitchTag { occurrence, cases, default })
        }
        ValueType::Int => {
            let mut literals = BTreeSet::new();
            for row in &matrix.rows {
                if let Pattern::Int(value) = &row.patterns[column] {
                    literals.insert(*value);
                }
            }

            let mut cases = Vec::with_capacity(literals.len());
            for literal in &literals {
                let specialized = specialize_int(&matrix, column, &occurrence, *literal);
                cases.push((*literal, compile_matrix(specialized)?));
            }

            let default =
                Box::new(compile_matrix(default_matrix(&matrix, column, &occurrence))?);
            Ok(DecisionTree::SwitchInt { occurrence, cases, default })
        }
    }
}

fn expanded_columns(
    columns: &[(Occurrence, ValueType)],
    column: usize,
    sub_columns: Vec<(Occurrence, ValueType)>,
) -> Vec<(Occurrence, ValueType)> {
    let mut next = Vec::with_capacity(columns.len() - 1 + sub_columns.len());
    next.extend_from_slice(&columns[..column]);
    next.extend(sub_columns);
    next.extend_from_slice(&columns[column + 1..]);
    next
}

fn expanded_patterns(patterns: &[Pattern], column: usize, sub: Vec<Pattern>) -> Vec<Pattern> {
    let mut next = Vec::with_capacity(patterns.len() - 1 + sub.len());
    next.extend_from_slice(&patterns[..column]);
    next.extend(sub);
    next.extend_from_slice(&patterns[column + 1..]);
    next
}

fn specialize_product(
    matrix: WorkMatrix,
    column: usize,
    occurrence: &Occurrence,
    field_types: &[ValueType],
) -> WorkMatrix {
    let sub_columns: Vec<(Occurrence, ValueType)> = field_types
        .iter()
        .enumerate()
        .map(|(field, field_type)| {
            let mut path = occurrence.clone();
            path.push(Access::ProductField { field });
            (path, field_type.clone())
        })
        .collect();

    let columns = expanded_columns(&matrix.columns, column, sub_columns);
    let rows = matrix
        .rows
        .into_iter()
        .map(|mut row| {
            let pattern = row.patterns[column].clone();
            let sub = match pattern {
                Pattern::Product(fields) => fields,
                irrefutable => {
                    absorb_irrefutable(&irrefutable, occurrence, &mut row.bindings);
                    vec![Pattern::Wildcard; field_types.len()]
                }
            };
            WorkRow {
                patterns: expanded_patterns(&row.patterns, column, sub),
                arm: row.arm,
                bindings: row.bindings,
            }
        })
        .collect();

    WorkMatrix { columns, rows }
}

fn specialize_variant(
    matrix: &WorkMatrix,
    column: usize,
    occurrence: &Occurrence,
    tag: usize,
    field_types: &[ValueType],
) -> WorkMatrix {
    let sub_columns: Vec<(Occurrence, ValueType)> = field_types
        .iter()
        .enumerate()
        .map(|(field, field_type)| {
            let mut path = occurrence.clone();
            path.push(Access::SumField { variant: tag, field });
            (path, field_type.clone())
        })
        .collect();

    let columns = expanded_columns(&matrix.columns, column, sub_columns);
    let rows = matrix
        .rows
        .iter()
        .filter_map(|row| {
            let mut bindings = row.bindings.clone();
            let sub = match &row.patterns[column] {
                Pattern::Variant { tag: row_tag, fields } if *row_tag == tag => fields.clone(),
                Pattern::Variant { .. } => return None,
                irrefutable => {
                    absorb_irrefutable(irrefutable, occurrence, &mut bindings);
                    vec![Pattern::Wildcard; field_types.len()]
                }
            };
            Some(WorkRow {
                patterns: expanded_patterns(&row.patterns, column, sub),
                arm: row.arm,
                bindings,
            })
        })
        .collect();

    WorkMatrix { columns, rows }
}

fn specialize_int(
    matrix: &WorkMatrix,
    column: usize,
    occurrence: &Occurrence,
    literal: i64,
) -> WorkMatrix {
    let columns = expanded_columns(&matrix.columns, column, Vec::new());
    let rows = matrix
        .rows
        .iter()
        .filter_map(|row| {
            let mut bindings = row.bindings.clone();
            match &row.patterns[column] {
                Pattern::Int(value) if *value == literal => {}
                Pattern::Int(_) => return None,
                irrefutable => absorb_irrefutable(irrefutable, occurrence, &mut bindings),
            }
            Some(WorkRow {
                patterns: expanded_patterns(&row.patterns, column, Vec::new()),
                arm: row.arm,
                bindings,
            })
        })
        .collect();
    WorkMatrix { columns, rows }
}

/// Maranget's default matrix: rows irrefutable at `column`, column
/// removed.
fn default_matrix(matrix: &WorkMatrix, column: usize, occurrence: &Occurrence) -> WorkMatrix {
    let columns = expanded_columns(&matrix.columns, column, Vec::new());
    let rows = matrix
        .rows
        .iter()
        .filter_map(|row| {
            if !row.patterns[column].is_irrefutable() {
                return None;
            }
            let mut bindings = row.bindings.clone();
            absorb_irrefutable(&row.patterns[column], occurrence, &mut bindings);
            Some(WorkRow {
                patterns: expanded_patterns(&row.patterns, column, Vec::new()),
                arm: row.arm,
                bindings,
            })
        })
        .collect();
    WorkMatrix { columns, rows }
}

fn collect_leaf_arms(tree: &DecisionTree, arms: &mut BTreeSet<usize>) {
    match tree {
        DecisionTree::Fail => {}
        DecisionTree::Leaf { arm, .. } => {
            arms.insert(*arm);
        }
        DecisionTree::SwitchTag { cases, default, .. } => {
            for (_, subtree) in cases {
                collect_leaf_arms(subtree, arms);
            }
            if let Some(subtree) = default {
                collect_leaf_arms(subtree, arms);
            }
        }
        DecisionTree::SwitchInt { cases, default, .. } => {
            for (_, subtree) in cases {
                collect_leaf_arms(subtree, arms);
            }
            collect_leaf_arms(default, arms);
        }
    }
}

/// DFS to the first FAIL, recording branch constraints; returns whether
/// one was found (the path is then a concrete counterexample class).
fn find_fail(tree: &DecisionTree, path: &mut Vec<(Occurrence, String)>) -> bool {
    match tree {
        DecisionTree::Fail => true,
        DecisionTree::Leaf { .. } => false,
        DecisionTree::SwitchTag { occurrence, cases, default } => {
            for (tag, subtree) in cases {
                path.push((occurrence.clone(), format!("is variant {tag}")));
                if find_fail(subtree, path) {
                    return true;
                }
                path.pop();
            }
            if let Some(subtree) = default {
                let covered: Vec<String> =
                    cases.iter().map(|(tag, _)| format!("v{tag}")).collect();
                path.push((
                    occurrence.clone(),
                    format!("is any variant not in {{{}}}", covered.join(", ")),
                ));
                if find_fail(subtree, path) {
                    return true;
                }
                path.pop();
            }
            false
        }
        DecisionTree::SwitchInt { occurrence, cases, default } => {
            for (literal, subtree) in cases {
                path.push((occurrence.clone(), format!("= {literal}")));
                if find_fail(subtree, path) {
                    return true;
                }
                path.pop();
            }
            let covered: Vec<String> =
                cases.iter().map(|(literal, _)| literal.to_string()).collect();
            path.push((
                occurrence.clone(),
                format!("is any integer not in {{{}}}", covered.join(", ")),
            ));
            if find_fail(default, path) {
                return true;
            }
            path.pop();
            false
        }
    }
}
