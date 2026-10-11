# Steering amendment: Vector syntax is a first-class language surface

## 1. Direction

Developer experience takes precedence over implementation vocabulary. Ordinary vectors need literals, indexing, slices, updates, patterns, and comprehensions. Explicit SIMD needs arithmetic operators and contextual literals, not a catalog of primitive calls.

This amendment supersedes earlier examples that require `vector.from([...])` for ordinary construction. It preserves the concurrency and CPU amendment's safety and numerical semantics.

### Immediate actions

- Read the current specification, freeze log, active amendments, and implementation before changes.
- Record this amendment and its acceptance rows through the existing integrity mechanism.
- Preserve the original goals and historical bootstrap snapshots.
- Amend conflicting syntax decisions explicitly.
- Make the following surface part of the language release contract.
- Implement ordinary vector syntax during core collection work.
- Implement SIMD syntax with the first admitted SIMD operations.
- Do not defer ergonomics until after a public intrinsic-only API hardens.

## 2. Three representations, concise spellings

| Type spelling | Meaning |
|---|---|
| `[T]` | The immutable, persistent `Vector(T)` collection. |
| `[N]T` | A fixed-length inline array with `N` elements. |
| `vec[N]T` | An explicit SIMD value with `N` logical lanes. |

`Vector(T)` remains the underlying generic collection name and a compatible explicit spelling. The formatter and ordinary examples prefer `[T]`. A SIMD value does not imply a heap collection or one particular machine register.

### Construction rules

- Make `[a, b, c]` the ordinary collection literal.
- Default an unconstrained literal to `[T]`, subject to existing element inference and numeric-literal rules.
- Let an expected `[N]T` or `vec[N]T` type construct that representation directly.
- Do not construct a persistent vector before a contextually typed array or SIMD literal.
- Require exactly `N` elements for fixed-length array and SIMD literals.
- Diagnose heterogeneous literals unless a legitimate common element type exists.
- Do not introduce an implicit dynamic value box to reconcile incompatible elements.
- Infer an empty literal's element type from context.
- If context cannot determine that type, request an annotation.
- Support trailing commas, multiline layout, comments, and source-mapped element errors.
- Keep explicit views distinct from owning collection and array values.
- Preserve existing fixed-array ABI and lifetime rules.

```calque
const scores: [i64] = [10, 20, 30];
const empty: [User] = [];
const matrixRow: [4]f32 = [1.0, 2.0, 3.0, 4.0];
const lanes: vec[4]f32 = [1.0, 2.0, 3.0, 4.0];
```

## 3. Everyday collection syntax

```calque
const scores: [i64] = [10, 20, 30];
const extended = [0, ..scores, 40];
const joined = scores ++ [40, 50];
const middle = scores[1..3];
const lastTwo = scores[1..];
const firstTwo = scores[..2];

var edited = scores;
edited[1] = 99;

assert scores == [10, 20, 30];
assert edited == [10, 99, 30];
assert middle == [20, 30];
```

### Access, spread, and concatenation

- Implement checked `values[index]` access.
- Use half-open ranges for slices.
- Support omitted range endpoints under the same bounds semantics.
- Require slice boundaries to satisfy `0 <= start <= end <= length`.
- Reject invalid constant indices during compilation when their invalidity is established.
- Preserve deterministic runtime checks for remaining accesses.
- Do not silently wrap negative indices or truncate invalid ranges.
- Make persistent-vector slices return immutable persistent-vector values.
- Keep any backing-storage sharing invisible to value semantics.
- Document retention and allocation costs without universal zero-copy promises.
- Keep fixed-array views subject to their explicit managed-lifetime contract.
- Implement `[..left, value, ..right]` spread construction for admitted collection inputs.
- Evaluate entries and spread sources once, in written order.
- Implement `left ++ right` as concatenation of compatible persistent vectors.
- Keep length arithmetic checked before allocation.
- Do not implement persistent-vector `+` as hidden elementwise arithmetic.
- Do not implicitly convert an existing collection into SIMD merely because an operator requires SIMD.

### Mutation syntax with value semantics

- Permit `values[index] = replacement` only through a valid mutable place.
- For persistent vectors, define that operation as replacement followed by rebinding of the mutable place.
- Preserve old aliases and earlier immutable snapshots.
- Reject assignments through `const` bindings.
- Evaluate the mutable place, index, and replacement once under a documented left-to-right rule.
- Preserve bounds failures and effect order in expanded update operations.
- Preserve existing capture and concurrency restrictions for mutable bindings.
- Do not treat element-update syntax as an implicit atomic operation.
- Permit internal reuse only when uniqueness and ownership analysis justify it.
- Keep explicit builders for bulk work. Do not require them for ordinary literals.

## 4. Comprehensions rather than construction ceremonies

```calque
const names: [Str] = [
    for user in users
    if user.active
    => user.name
];

const squares: [i64] = [for n in 0..10 => n * n];
```

### Comprehension contract

- Implement `[for binding in source => expression]` as an eager vector comprehension.
- Support an optional `if predicate` filter before the result expression.
- Keep the parenthesized statement-loop syntax compatible with existing source where required.
- Evaluate the source once.
- Evaluate the predicate once per input element in traversal order.
- Evaluate the result only for accepted elements.
- Keep a half-open numeric range iterable without an intermediate vector.
- Preserve `try`, explicit effects, and checked arithmetic inside comprehension expressions.
- Allocate or share output through the normal verified collection implementation.
- Lower the comprehension to one builder or equivalent construction path when valid.
- Avoid intermediate map/filter collections that the surface never requested.
- Do not introduce hidden tasks, unordered evaluation, or fast-math.
- Preserve existing functional pipeline syntax as another composition tool.
- Do not require `seq.iter()` solely to express an ordinary comprehension.
- Keep nested comprehensions available through ordinary expression composition before new grammar variants proliferate.

## 5. Vector patterns

```calque
pub fn firstOrZero(values: [i64]) i64 {
    match values {
        [] => 0,
        [first, .._] => first,
    }
}

const description = match scores {
    [] => "empty",
    [_] => "one value",
    [first, ..middle, last] => "multiple values",
};
```

### Pattern contract

- Support empty and exact-length patterns.
- Support one rest pattern at the beginning, middle, or end.
- Support a named rest capture and `.._` without an unused capture.
- Specify the minimum length of patterns with fixed prefixes and suffixes.
- Make exact patterns require an exact length.
- Make a persistent-vector rest capture an immutable vector value.
- Compose element patterns with structs, tagged variants, literals, guards, and pinning.
- Use the existing pattern compiler and exhaustiveness machinery.
- Produce concrete length or element witnesses for uncovered cases.
- Do not treat guarded arms as unconditional coverage.
- Do not allocate ignored rest captures.
- Preserve efficient length checks before element extraction.
- Diagnose incompatible array and SIMD pattern lengths under their own fixed-length contracts.

## 6. SIMD must read like arithmetic

```calque
const a: vec[4]f32 = [1.0, 2.0, 3.0, 4.0];
const b: vec[4]f32 = [5.0, 6.0, 7.0, 8.0];

const scaled = a * 2.0 + b;
const clipped = simd.select(scaled > 10.0, 10.0, scaled);

assert.equal(clipped, [7.0, 10.0, 10.0, 10.0]);

const rgba: vec[4]f32 = [0.2, 0.4, 0.8, 1.0];
const bgra = rgba.zyxw;
```

### Operators and scalar broadcast

- Implement lane-wise arithmetic for admitted element types.
- Support scalar broadcast for operators with one SIMD operand and one compatible scalar operand.
- Apply existing contextual numeric-literal rules to broadcast literals.
- Require nonliteral scalar operands to satisfy the documented element-type contract.
- Do not add implicit narrowing or arbitrary numeric promotion.
- Evaluate a broadcast scalar once, not once per lane.
- Reject mismatched lane counts and incompatible element types with source-level diagnostics.
- Define comparisons as lane-mask results.
- Keep structural test equality available through `assert.equal` without an implicit mask-to-Boolean conversion.
- Infer the second equality argument from the first when its literal permits contextual typing.
- Support mask operations with explicitly documented lane semantics.
- Reject a lane mask where an ordinary Boolean condition is required.
- Suggest explicit `simd.any` or `simd.all`. Leave the choice of condition to the user.
- Keep `simd.select(mask, thenValue, elseValue)` as one obvious selection primitive.
- Permit compatible scalar broadcasts in its value arguments.
- Evaluate its arguments eagerly in the normal call order.
- Do not claim that selection prevents traps in an already evaluated branch expression.

### Lane access and swizzles

- Support checked lane access through the ordinary index syntax.
- Provide read-only `x`, `y`, `z`, and `w` lane names where those indices exist.
- Provide read-only two-to-four-lane swizzles such as `.xy`, `.xyz`, `.zyxw`, and `.xx`.
- Diagnose unavailable source lanes.
- Make swizzles compile-time lane selection without an intermediate collection.
- Do not introduce assignable or mixed naming-family swizzles in this amendment.
- Keep general shuffles available through one typed library operation.

### Semantics and representation

- Preserve the previously specified checked arithmetic and floating-point policies.
- Define checked SIMD failures through scalar lane semantics with stable failing-lane diagnostics.
- Do not enable global reassociation or contraction merely because vector syntax appears.
- Retain explicit masked memory operations for safe tail handling.
- Keep the logical lane count independent from the hardware register width.
- If native SIMD is unavailable, scalarize under the same value and failure contract.
- Do not make ordinary SIMD arithmetic allocate managed collections.
- Preserve SIMD values through function arguments, results, structs, and specialization.
- Verify actual vector operations and native instructions where the target supports them.

## 7. Developer tools and cost transparency

- Format collection, comprehension, pattern, and SIMD syntax canonically through the existing formatter.
- Complete vector element types and SIMD lanes through the shared LSP services.
- Show `[T]`, `[N]T`, and `vec[N]T` distinctly in hover and diagnostics.
- Point at the incompatible element, lane count, index, or broadcast operand.
- Preserve source origins through comprehension and update expansion.
- Keep generated builder or intrinsic internals out of the primary user-error location.
- Document potential slice retention, concatenation cost, and explicit builder alternatives.
- Do not call a transformation allocation-free without the existing cost-analysis evidence.
- Keep automatic vectorization available for suitable ordinary code.
- Do not require explicit SIMD for every performance-sensitive function.
- Explain missed vectorization. Do not rewrite safe source into unchecked operations.
- Make all syntax and cost documentation available through CLI Markdown.

## 8. Verification and completion

### Required evidence

- Verify contextual literals across persistent vectors, inline arrays, and SIMD values.
- Verify that default literals do not require `vector.from` wrappers.
- Verify immutable alias preservation after indexed rebinding.
- Verify spread and comprehension evaluation order with effect-bearing fixtures.
- Verify filtered comprehensions do not evaluate rejected result expressions.
- Verify exact-length and rest-pattern exhaustiveness with independent expected results.
- Verify checked bounds, overflow, invalid lane counts, and scalar-type errors.
- Compare SIMD sugar with scalar per-lane semantics under interpreter, JIT, and AOT.
- Test floating-point edge cases and checked integer failures.
- Verify that contextual SIMD literals do not construct temporary persistent vectors.
- Verify formatter idempotence, doctests, incomplete-source completion, and source-level failures.
- Fuzz literals, comprehensions, patterns, and operator expressions through existing infrastructure.
- Inspect representative generated code and allocation reports.
- Preserve active compiler sources, historical bootstrap artifacts, and the self-host fixed point.

### Delivery

- Migrate ordinary documentation and examples away from wrapper-heavy collection construction.
- Use the new syntax in the compiler and standard-library tests.
- Add this amendment's tests to existing collection and CPU gates.
- Preserve all previous mandatory features and safety contracts.
- Update the specification, acceptance matrix, and `STATE.md` with actual evidence.
- Do not mark intrinsic availability as completion of this syntax amendment.

**The implementation can contain builders, vectors, masks, and intrinsics. Ordinary Calque source must not expose that machinery unnecessarily.**
