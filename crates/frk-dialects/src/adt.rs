//! frk.adt — sums, products, tuples as parametric `!frk_adt` types with
//! pure value ops (SPEC §4.1 as amended by D-031). The dialect namespace
//! is `frk_adt`; SPEC prose writes "frk.adt" for the same thing.
//!
//! Ops:
//! - `make_sum(fields...) {variant} -> !frk_adt.sum<...>`
//! - `tag_of(sum) -> i64`
//! - `extract(sum) {variant, field} -> <field type>`
//! - `make_product(fields...) -> !frk_adt.product<...>`
//! - `get(product) {field} -> <field type>`
//!
//! Type parameter encoding: a sum's single parameter is an array of
//! variants, each an array of field types — `Option<i64>` is
//! `!frk_adt.sum<[[], [i64]]>`; a product's parameter is its field-type
//! array — `!frk_adt.product<[i64, i64]>`. Tuples ARE products.
//!
//! There is deliberately no `match` op (D-031): dispatch compiles to
//! `tag_of` + upstream `cf.switch` + per-arm `extract`, produced by the
//! decision-tree pass from the frontend's pattern matrix.
//!
//! IRDL enforces shape here — operand/result base types (sum vs product
//! vs anything), attribute kinds, arity, i64 tags. What IRDL cannot say
//! (variant/field indices in range, extract's result = the named field's
//! type, make_sum operand types = the variant's shape) is the frk
//! verification pass's job (K1 second half; runs before execution or
//! lowering).
//!
//! IRDL landmines, learned against mlir-opt 22.1.8:
//! - A reused constraint variable unifies *values*: one `%idx` shared by
//!   `variant` and `field` would demand variant == field. Every
//!   independently-valued attribute gets its own variable.
//! - `irdl.is i64` means "the attribute equals the type i64";
//!   `irdl.base "#builtin.integer"` means "any integer attribute".
//! - `irdl.parametric`/`irdl.base` symbol refs must be fully nested
//!   (`@frk_adt::@sum`).

/// The dialect definition, loaded at registration time (D-031) by
/// [`crate::register`]. (Raw string uses ##: the IRDL text itself
/// contains `"#` in `irdl.base "#builtin.integer"`.)
pub const IRDL: &str = r##"
irdl.dialect @frk_adt {
  irdl.type @sum {
    %variants = irdl.any
    irdl.parameters(variants: %variants)
  }
  irdl.type @product {
    %fields = irdl.any
    irdl.parameters(fields: %fields)
  }
  irdl.operation @make_sum {
    %any = irdl.any
    %sum = irdl.base @frk_adt::@sum
    %idx = irdl.base "#builtin.integer"
    irdl.operands(fields: variadic %any)
    irdl.results(sum: %sum)
    irdl.attributes { "variant" = %idx }
  }
  irdl.operation @tag_of {
    %sum = irdl.base @frk_adt::@sum
    %tag = irdl.is i64
    irdl.operands(sum: %sum)
    irdl.results(tag: %tag)
  }
  irdl.operation @extract {
    %sum = irdl.base @frk_adt::@sum
    %any = irdl.any
    %vidx = irdl.base "#builtin.integer"
    %fidx = irdl.base "#builtin.integer"
    irdl.operands(sum: %sum)
    irdl.results(value: %any)
    irdl.attributes { "variant" = %vidx, "field" = %fidx }
  }
  irdl.operation @make_product {
    %any = irdl.any
    %prod = irdl.base @frk_adt::@product
    irdl.operands(fields: variadic %any)
    irdl.results(product: %prod)
  }
  irdl.operation @get {
    %prod = irdl.base @frk_adt::@product
    %any = irdl.any
    %idx = irdl.base "#builtin.integer"
    irdl.operands(product: %prod)
    irdl.results(value: %any)
    irdl.attributes { "field" = %idx }
  }
}
"##;
