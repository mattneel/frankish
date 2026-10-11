# Calque: demos and syntax

**Calque needs a recognizable shape: Zig-like declarations, functional values, and ordinary code that can execute at compile time.**

Everything below is a **proposed syntax and demo storyboard**. The commands and output are release targets, not captured runs. Examples omit routine imports and terminal-output wrappers.

The memorable promise is:

> Your compiler is a program. Your compile-time code is a program. They speak the same language and use the same interpreter.

## 1. “The compiler was its own first serious program.”

This is the flagship clip. The interesting source file is the compiler itself, not a toy program that prints its own text.

```calque
// tools/calque/compiler/compile.calq

pub fn compile(source: Source, target: Target) CompileError!Artifact {
    const tokens = try lex(source);
    const syntax = try parse(tokens);
    const typed = try check(syntax, target);

    emit(typed, target)
}
```

The proposed terminal reveal:

```console
$ make calque-selfhost

seed + recorded bootstrap chain    verified
compiler under the interpreter     produced stage-1.kernel
native compiler                    produced stage-2.kernel
rebuilt native compiler            produced stage-3.kernel

canonical stage 1 == stage 2        true
canonical stage 2 == stage 3        true
compiler corpus                    equivalent
```

The comparison uses canonical kernel artifacts for one target and configuration. It does not require identical linked binary bytes.

The seed remains an explicit trust root. The canonical frontend remains Calque source. Frankish and its native backend remain host infrastructure.

**The proof needs a compiler edit that changes generated output.** A fixed point alone does not prove that the compiler understands its input.

**Caption:** “My language’s first serious application was its own compiler. There was no later rewrite.”

## 2. “My compiler compiles my compiler’s language during compilation.”

This is the recursion joke with an actual implementation behind it.

```calque
const answer = comptime {
    const artifact = compiler.compile(
        Source.inline("answer.calq", """
            pub fn answer() i64 {
                6 * 7
            }
        """),
        Target.current(),
    ) |> result.expect("the nested program must compile");

    frankish.eval(i64, artifact, .{ .entry = "answer" })
        |> result.expect("the nested program must return i64")
};

test "the compiler is a library" {
    assert.equal(answer, 42);
}
```

```console
$ calque test demos/inception.calq --explain-comptime

compile-time call: compiler.compile
  source: answer.calq
  result: verified kernel artifact

compile-time call: frankish.eval
  entry: answer
  result: 42 : i64

embedded runtime value: 42
PASS the compiler is a library
```

`compiler.compile` is the canonical compiler library written in Calque. `frankish.eval` is the reusable host service, not another Calque interpreter.

The artifact stays inside the compile-time session. Only the integer crosses into the outer program. `Target.current()` describes the outer build target, not the host machine.

The runtime executable needs neither the nested compiler nor its source text after dead-code removal. Artifact inspection must establish that claim.

The clip becomes stronger with one live edit:

```diff
-                6 * 7
+                6 * 9
```

The embedded value changes to `54`. No fixture or cached answer can substitute for that dependency.

**Caption:** “Not a macro. Not a transpiler trick. The compiler is an ordinary library, so I called it at compile time.”

## 3. “Lua at compile time. Calque at runtime. NOW KITH.”

This is the Frankish-specific reveal. Calque does not invent another Lua evaluator. It requests the registered specimen frontend and the shared interpreter.

```calque
const luaAnswer = comptime {
    frankish.evalSpecimen(f64, .{
        .profile = "femto_lua",
        .source = Source.inline("answer.lua", """
            return 6 * 7
        """),
    }) |> result.expect("the Lua fragment must return a number")
};

test "a number crossed the specimen boundary" {
    assert.equal(luaAnswer, 42.0);
}
```

```console
$ calque test demos/now-kith.calq --explain-comptime

frontend:          femto_lua
execution:         Frankish interpreter
boundary value:    42 : f64
embedded constant: 42 : f64

PASS a number crossed the specimen boundary
```

The interface uses an explicit portable value boundary. The foreign program stays inside its specimen’s admitted subset.

A later cut can show the same contract across every supported profile. Each profile needs its own conformance case and declared result type.

**The runtime claim is narrow:** the resulting application does not need a Lua frontend or interpreter. The build still needs the registered frontend.

The end card can contain exactly two words:

```text
NOW KITH
```

**Caption:** “I asked Lua a question while compiling Calque. Frankish already knew how to answer.”

## 4. “The expensive function exists in source. It does not exist at runtime.”

A regular-expression constructor makes this visible without compiler internals as the opening shot.

```calque
const ticketPattern = comptime {
    regex.compile("^[A-Z]{3}-[0-9]{4}$")
        |> result.expect("the ticket pattern must be valid")
};

@noalloc
pub fn validTicket(text: Str) bool {
    regex.isMatch(ticketPattern, text)
}

test "ticket codes" {
    assert.equal(validTicket("ABC-2026"), true);
    assert.equal(validTicket("abc-2026"), false);
    assert.equal(validTicket("ABC-26"), false);
}
```

This demo uses a regular-expression subset with a table-driven matcher. Compile-time execution constructs the table. Runtime code still executes the matcher.

```console
$ calque explain demos/tickets.calq --runtime-artifact

retained:
  validTicket
  table-driven matcher
  ticketPattern constant data

absent:
  regular-expression parser
  automaton constructor

validTicket: no-allocation contract verified
```

The useful trick is broader than regular expressions. The same model applies to these library constructions:

- A command parser from an option declaration.
- A lookup table from an ordinary numeric function.
- A protocol decoder from a schema.
- A decision table from validated configuration.

A second shot introduces an invalid pattern. The failure points to the source string and the compile-time call trace.

**Caption:** “I shipped the result of the algorithm. I did not ship the algorithm that built it.”

## 5. “Functional source. One ordinary loop.”

This is the beauty shot for pipelines and pattern matching together.

```calque
const Event = union(enum) {
    Sale: struct {
        cents: i64,
        settled: bool,
    },
    Refund: i64,
    View: Str,
};

@noalloc
pub fn netRevenue(events: Vector(Event)) i64 {
    events
        |> seq.iter()
        |> seq.filterMap(fn (event) {
            match event {
                .Sale(.{ .cents = amount, .settled = true })
                    => .Some(amount),
                .Refund(amount) => .Some(-amount),
                _ => .None,
            }
        })
        |> seq.fold(0, fn (total, amount) { total + amount })
}
```

The fixture separates settled revenue from unrelated events:

```calque
const events: Vector(Event) = vector.from([
    .Sale(.{ .cents = 1200, .settled = true }),
    .Sale(.{ .cents = 500, .settled = false }),
    .Refund(200),
    .View("/pricing"),
]);

test "money, not page views" {
    assert.equal(netRevenue(events), 1000);
}
```

```console
$ calque explain demos/revenue.calq --function netRevenue

sequence stages:       fused
closure environments:  none
intermediate heaps:    none
no-allocation contract: verified

$ calque test demos/revenue.calq
PASS money, not page views
```

`seq` describes an explicit pull sequence. The language itself remains strict. Eager collection operations remain distinct from sequence operations.

The pipeline rule stays small:

```calque
value |> function(argument)

// Exactly the call shape:
function(value, argument)
```

The left side executes once. The call preserves left-to-right evaluation. Pipelines do not silently unwrap errors or insert asynchronous execution.

The allocation contract covers reachable calls, not just the function body. Unresolved dynamic calls fail the contract unless another verified contract establishes allocation freedom.

**The proof requires lowered-code inspection and allocation verification.** An attractive pipeline is not evidence of fusion.

**Caption:** “I wrote a pipeline. The compiler proved that it does not need a heap.”

## 6. “Add one variant. The compiler finds every unfinished decision.”

This is a particularly good edit-and-reveal clip because the audience can understand the defect immediately.

```calque
const Job = union(enum) {
    Queued: u32,
    Running: struct {
        worker: Str,
        attempt: u8,
    },
    Done: Str,
    Failed: Str,
};

pub fn badge(job: Job) Str {
    match job {
        .Queued(0) => "up next",
        .Queued(n) if n > 100 => "bring a book",
        .Queued(_) => "in queue",
        .Running(.{ .attempt = 1, .. }) => "first try",
        .Running(_) => "again, with feeling",
        .Done(_) => "shipped",
        .Failed(_) => "well, that happened",
    }
}
```

The live edit adds a variant:

```diff
     Failed: Str,
+    Cancelled,
```

```console
$ calque check demos/jobs.calq

error: match does not cover Job.Cancelled

  missing case:
    .Cancelled

  affected function:
    badge
```

The repair can use an or-pattern:

```calque
.Failed(_) | .Cancelled => "not happening",
```

The pattern does not bind different variables on its alternatives. The guarded queue case also retains an unguarded fallback.

A guarded arm does not establish complete coverage of its variant. The compiler reports concrete missing cases rather than a generic “not exhaustive” message.

**Caption:** “I added a state. The compiler handed me the exact list of decisions I forgot to make.”

## 7. “The struct is the codec. The struct is the schema.”

This demonstrates compile-time types and reflection through a useful library.

```calque
const Config = struct {
    port: u16,
    theme: Str,
    plugins: Vector(Str),
};

const ConfigJson = json.Codec(Config);

test "configuration survives the round trip" {
    const original = Config {
        .port = 8080,
        .theme = "dark",
        .plugins = vector.from(["calque", "frankish"]),
    };

    const encoded = ConfigJson.encode(original);
    const decoded = ConfigJson.decode(encoded)
        |> result.expect("encoded configuration must decode");

    assert.equal(decoded, original);
}
```

`json.Codec` is a compile-time type factory. It specializes its methods from the fields of `Config`. The same declaration controls the codec and its schema.

An invalid document creates a precise runtime error:

```json
{
  "port": 70000,
  "theme": "dark",
  "plugins": []
}
```

```text
configuration error at $.port

expected: u16, from 0 through 65535
received: 70000

source field: Config.port
```

The second reveal changes `port` to a different type. The codec and schema change through the same compiler dependency.

The factory does not rewrite source text. It returns a type through the compile-time type interface.

**Caption:** “One struct. No handwritten decoder. No second schema to forget to update.”

## 8. “Undo is an old value.”

This is the functional-programming clip that does not need the audience to care about functional programming.

```calque
const original = map.from([
    ("theme", "dark"),
    ("font", "mono"),
]);

const edited = original
    |> map.put("theme", "light");

const history = vector.from([
    original,
    edited,
    original,
]);
```

```console
$ calque run demos/undo.calq

initial: theme = dark
edit:    theme = light
undo:    theme = dark

original value after edit: theme = dark
```

The implementation shares unchanged structure. The old value remains valid after the update. Undo selects a previous value. It does not reconstruct overwritten state.

A stronger version applies the same model to compiler scopes. A speculative parse can keep the original environment without an ad hoc restore procedure.

Persistent values do not imply allocation-free updates. The cost report must distinguish shared structure from newly allocated nodes.

**Caption:** “Undo was not a feature I implemented. I kept the previous value.”

## 9. “A million recursive calls. A fixed stack.”

This clip gives the functional core a concrete systems-level guarantee.

```calque
pub fn sumTo(n: u64, total: u64) u64 {
    if n == 0 {
        total
    } else {
        sumTo(n - 1, total + n)
    }
}

test "one million tail calls" {
    assert.equal(sumTo(1_000_000, 0), 500_000_500_000);
}
```

```console
$ make calque-demo-tailcalls

interpreter result:       500000500000
native result:            500000500000
native fixed-stack test:  PASS
mutual-tail-call test:    PASS
closure-tail-call test:   PASS
```

The proof uses stack instrumentation or a constrained-stack runner. The large input alone does not establish constant stack use.

The guarantee applies to tail calls within the admitted calling convention. An ordinary recursive call with work after its return remains a different case.

**Caption:** “Recursion is not the stack problem. Unfinished work is.”

## 10. “Build-time code gets a file, not my entire laptop.”

This demonstrates a useful boundary around the compile-time power.

The proposed manifest exposes one named build input:

```toml
# calque.toml

[build.inputs]
routes = "routes.json"
```

The source requests that input by name:

```calque
const routeTable = comptime {
    const source = build.readText("routes")
        |> result.expect("the declared routes input must exist");

    const definitions = json.decode(Vector(RouteSpec), source)
        |> result.expect("the routes input must match RouteSpec");

    router.compile(definitions)
        |> result.expect("the route table must be valid")
};
```

The terminal explains the dependency:

```console
$ calque build demos/router.calq --explain-comptime
routeTable: evaluated
dependency: routes.json

$ calque build demos/router.calq --explain-comptime
routeTable: reused
```

After one route changes:

```console
$ calque build demos/router.calq --explain-comptime
routeTable: invalidated because routes.json changed
routeTable: evaluated
```

The input’s content participates in cache validity. An undeclared path does not become readable because compile-time code requests it.

A negative test demonstrates the boundary:

```text
compile-time capability error

requested input: private-key
available input: routes

the build did not grant access to private-key
```

The same execution service denies ambient network and shell access. Enforcement belongs at the service boundary, not only in frontend annotations.

**Caption:** “Compile-time execution is powerful. Ambient authority is optional. Calque defaults to no.”

# The rest of the syntax goodies

The demos cover the large features. These smaller choices make ordinary code feel like Calque.

## Expression bodies without mandatory returns

```calque
pub fn clamp(value: i64, low: i64, high: i64) i64 {
    if value < low {
        low
    } else if value > high {
        high
    } else {
        value
    }
}
```

A block’s final expression supplies its value. `return` remains available for an early exit. A semicolon discards an expression’s value.

## Closures without automatic currying

```calque
pub fn offsetBy(amount: i64) fn(i64) i64 {
    fn (value) { value + amount }
}

const addTen = offsetBy(10);
const answer = 32 |> addTen();
```

The closure keeps its capture alive automatically. Function calls retain ordinary argument lists. Partial application stays explicit.

## Record updates without mutation

```calque
const User = struct {
    name: Str,
    score: i64,
    active: bool,
};

const before = User {
    .name = "Matt",
    .score = 41,
    .active = true,
};

const after = User {
    ..before,
    .score = 42,
};

const .{ .name, .score, .. } = after;
```

The spread copies the value’s fields semantically. Explicit fields override the spread. The compiler chooses physical copies or safe reuse. Existing aliases retain their original values.

## Errors with payloads, not just names

```calque
const PortError = union(enum) {
    NotANumber: Str,
    OutOfRange: u64,
};

pub fn parsePort(text: Str) PortError!u16 {
    const value = int.parse(u64, text) catch {
        return .Err(.NotANumber(text));
    };

    if value > 65_535 {
        return .Err(.OutOfRange(value));
    }

    .Ok(@intCast(u16, value))
}

const port = parsePort("70000") catch |err| {
    match err {
        .NotANumber(_) => 8080,
        .OutOfRange(_) => 8080,
    }
};
```

`E!T` is shorthand for `Result(T, E)`. `.Ok` and `.Err` are ordinary variants. `try` propagates an error, while `catch` supplies a replacement value.

`try` does not convert unrelated error types implicitly. A library can expose `result.mapError` for explicit adaptation.

## Optional values with ordinary combinators

```calque
// findUser returns ?User.

const score = findUser("matt")
    |> option.map(fn (user) { user.score })
    |> option.unwrapOr(0);
```

`?T` is shorthand for `Option(T)`. Its variants are `.Some(value)` and `.None`. The pipeline itself needs no special knowledge of optional values.

## Types as compile-time results

```calque
pub fn Pair(comptime T: type) type {
    struct {
        first: T,
        second: T,
    }
}

const names = Pair(Str) {
    .first = "Calque",
    .second = "Frankish",
};
```

The same mechanism supports library factories such as `json.Codec(Config)`. The compiler evaluates type construction before runtime layout selection.

The advanced form accepts both type and value parameters:

```calque
pub fn Fixed(comptime T: type, comptime count: usize) type {
    struct {
        values: [count]T,
    }
}

const Window = Fixed(f32, 8);
```

## Local mutation inside a pure function

```calque
pub fn sum(values: Vector(i64)) i64 {
    var total: i64 = 0;

    for (values) |value| {
        total += value;
    }

    total
}
```

No caller-visible state changes. The language does not force a functional-looking spelling when a private accumulator expresses the algorithm better.

## Explicit resources and useful cleanup

```calque
pub fn readDocument(fs: Fs, path: Path) FsError!Str {
    const file = try fs.openRead(path);
    defer file.close();

    file.readText()
}
```

The filesystem capability appears in the signature. `defer` runs on ordinary returns and error propagation. Heap reclamation does not define file lifetime.

That contract does not promise cleanup after process termination.

# What earns the first four clips

| Order | Clip | Reason |
|---|---|---|
| 1 | Functional source, ordinary loop | The source is readable before the audience knows Calque. |
| 2 | The compiler was its own first program | The architecture becomes concrete. |
| 3 | Compilation inside compilation | The same architecture produces an unusual capability. |
| 4 | NOW KITH | Frankish’s multi-specimen design becomes the reveal. |

The short versions can target roughly 20–35 seconds. Full verification remains available through the checked-in demo commands and complete transcripts.

## Capture procedure

1. Open with the source, not installation output.
2. Run the checked-in demonstration command.
3. Keep the result visible beside its verification.
4. Record the actual terminal session as `.cast`.
5. Export the same session to MP4.
6. Label edits to the session’s timing or verification output.

No invented timing belongs in these clips. Allocation and artifact claims need their own executable tests, not decorative terminal summaries.

**The overall reveal is not “look how many features we added.” It is “look how much follows from making the compiler ordinary code.”**
