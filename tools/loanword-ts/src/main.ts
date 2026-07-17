// loanword-ts — the TS frontend (SPEC §6.3, specimen TS-0 slice; D-045).
// Checker-as-oracle: tsc typechecks (strict); this tool never re-implements
// the checker — it walks the checked AST for the TS-0 subset and emits
// loanword v1: canonical JSON (recursively sorted keys, no whitespace),
// UTF-8, SHA-256 content id over the sha-less canonical bytes. Every node
// carries a [start, end) byte span into the embedded source (§6.5: the
// consumer threads these into MLIR locations).
//
// Anything outside the subset is a LOUD error naming the construct and its
// span — fences are law (L5).
//
// Usage: node src/main.ts FILE.ts   (canonical loanword on stdout)

import ts from "typescript";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { basename } from "node:path";

type Json = null | boolean | number | string | Json[] | { [k: string]: Json };

function canonical(value: Json): string {
  if (value === null || typeof value !== "object") return JSON.stringify(value);
  if (Array.isArray(value)) return "[" + value.map(canonical).join(",") + "]";
  const keys = Object.keys(value).sort();
  return "{" + keys.map((k) => JSON.stringify(k) + ":" + canonical(value[k])).join(",") + "}";
}

const file = process.argv[2];
if (!file) {
  console.error("usage: main.ts FILE.ts");
  process.exit(2);
}
const source = readFileSync(file, "utf8");

// Minimal ambient prelude instead of a lib: the classic noLib global
// set plus exactly the members TS-0 speaks (.length on string/array,
// console.log). The checker still owns all the typing.
const PRELUDE = `
interface Object {}
interface Function {}
interface CallableFunction extends Function {}
interface NewableFunction extends Function {}
interface IArguments {}
interface RegExp {}
interface Symbol {}
interface Number {}
interface Boolean {}
interface String { readonly length: number; }
interface Array<T> { readonly length: number; [n: number]: T; }
interface ReadonlyArray<T> { readonly length: number; readonly [n: number]: T; }
interface ConcatArray<T> {}
interface TemplateStringsArray {}
declare const console: { log(x: number | boolean | string): void };
`;
const preludeName = "__frk_prelude.d.ts";

// noImplicitReturns: checker-as-oracle corollary (D-050) — when the
// oracle offers a flag that eliminates a divergence class (fall-off
// returning frankish-0 vs node-undefined), SET THE FLAG. The reader's
// zero-synthesis is defensive dead code from here on.
const options: ts.CompilerOptions = {
  strict: true,
  noEmit: true,
  noLib: true,
  noImplicitReturns: true,
};
const host = ts.createCompilerHost(options);
const baseGet = host.getSourceFile.bind(host);
host.getSourceFile = (name, lang, ...rest) => {
  if (name === preludeName) return ts.createSourceFile(name, PRELUDE, lang);
  if (name === file) return ts.createSourceFile(name, source, lang);
  return baseGet(name, lang, ...rest);
};
const program = ts.createProgram([preludeName, file], options, host);
const sourceFile = program.getSourceFile(file)!;
const checker = program.getTypeChecker();

const diagnostics = ts.getPreEmitDiagnostics(program).filter(
  // noLib strips the default lib on purpose; the checker still types
  // the primitive algebra. Suppress only the lib-absence complaint.
  (d) => d.code !== 2318 /* Cannot find global type ... */
);
if (diagnostics.length > 0) {
  for (const d of diagnostics) {
    const where = d.file && d.start !== undefined
      ? `${d.file.fileName}:${d.start}`
      : "<unknown>";
    console.error(`tsc: ${where}: ${ts.flattenDiagnosticMessageText(d.messageText, " ")}`);
  }
  process.exit(1);
}

function fail(node: ts.Node, what: string): never {
  console.error(
    `loanword-ts: ${basename(file)}:[${node.getStart(sourceFile)},${node.end}): ` +
    `${what} is outside TS-0 (fences are law, L5)`
  );
  process.exit(1);
}

function span(node: ts.Node): Json {
  return [node.getStart(sourceFile), node.end];
}

// Interned type table. v1 rows: num | bool | void | str | arr, plus
// the TS-1 extension (D-072, additive per D-046): obj (a union
// variant: kind literal + payload fields) and union (variant row
// refs, declaration order).
const typeRows: Json[] = [];
const typeIndex = new Map<string, number>();
function internType(row: Json): number {
  const key = canonical(row);
  const existing = typeIndex.get(key);
  if (existing !== undefined) return existing;
  typeRows.push(row);
  typeIndex.set(key, typeRows.length - 1);
  return typeRows.length - 1;
}

// ---- TS-1 discriminated unions (D-072) ----
// Pass A collects object-type aliases (the variants); pass B resolves
// union aliases over them and interns their rows. The discriminant is
// a `kind: "<lit>"` property by fence; payload fields are num/bool/str.

interface VariantDef {
  aliasName: string;
  kind: string;
  fields: { name: string; ty: number }[];
}
interface UnionDef {
  rowIdx: number;
  variants: VariantDef[];
}

const objAliases = new Map<string, ts.TypeLiteralNode>();
const unionAliasNodes = new Map<string, ts.UnionTypeNode>();
for (const top of sourceFile.statements) {
  if (!ts.isTypeAliasDeclaration(top)) continue;
  if (top.typeParameters) fail(top, "a generic type alias (TS-4 territory)");
  const name = top.name.text;
  if (ts.isTypeLiteralNode(top.type)) {
    objAliases.set(name, top.type);
  } else if (ts.isUnionTypeNode(top.type)) {
    unionAliasNodes.set(name, top.type);
  } else {
    fail(top.type, "a type alias that is neither an object type nor a union");
  }
}

function scalarField(node: ts.TypeNode): number {
  const text = node.getText(sourceFile);
  if (text === "number") return internType({ k: "num" });
  if (text === "boolean") return internType({ k: "bool" });
  if (text === "string") return internType({ k: "str" });
  fail(node, `variant field type \`${text}\` (num/bool/str only in TS-1)`);
}

function variantOf(aliasName: string, owner: ts.Node): VariantDef {
  const literal = objAliases.get(aliasName);
  if (!literal) fail(owner, `union member \`${aliasName}\` (not an object-type alias)`);
  let kind: string | null = null;
  const fields: { name: string; ty: number }[] = [];
  for (const member of literal.members) {
    if (!ts.isPropertySignature(member) || !member.name || !ts.isIdentifier(member.name))
      fail(member, "a non-property union-variant member");
    if (member.questionToken) fail(member, "an optional variant property");
    if (!member.type) fail(member, "an unannotated variant property");
    const propName = member.name.text;
    if (propName === "kind") {
      if (
        !ts.isLiteralTypeNode(member.type) ||
        !ts.isStringLiteral(member.type.literal)
      )
        fail(member.type, "a kind that is not a string-literal type");
      kind = member.type.literal.text;
    } else {
      fields.push({ name: propName, ty: scalarField(member.type) });
    }
  }
  if (kind === null)
    fail(literal, `variant \`${aliasName}\` without a \`kind: "<lit>"\` discriminant`);
  return { aliasName, kind, fields };
}

const unions = new Map<string, UnionDef>();
for (const [name, node] of unionAliasNodes) {
  const variants: VariantDef[] = [];
  for (const member of node.types) {
    if (!ts.isTypeReferenceNode(member) || !ts.isIdentifier(member.typeName))
      fail(member, "a union member that is not a named variant alias");
    variants.push(variantOf(member.typeName.text, member));
  }
  const kinds = new Set(variants.map((v) => v.kind));
  if (kinds.size !== variants.length)
    fail(node, `union \`${name}\` with duplicate kind literals`);
  const variantRows = variants.map((v) =>
    internType({
      k: "obj",
      kind: v.kind,
      fields: v.fields.map((f) => ({ name: f.name, ty: f.ty })),
    })
  );
  const rowIdx = internType({ k: "union", variants: variantRows });
  unions.set(name, { rowIdx, variants });
}

// ---- TS-2.0 classes (D-073/D-074) ----
// Monomorphic classes: annotated fields (scalars + class references —
// crefs intern as their own rows so recursive shapes close), a single
// all-assigning constructor, methods. Nominal in this slice.

interface ClassRecord {
  rowIdx: number;
  fields: { name: string; ty: number }[];
}
const classNames = new Set<string>();
for (const top of sourceFile.statements) {
  if (ts.isClassDeclaration(top)) {
    if (!top.name) fail(top, "an anonymous class");
    if (top.typeParameters) fail(top, "a generic class (TS-4 territory)");
    if (top.heritageClauses) fail(top, "extends/implements (fenced in TS-2.0)");
    classNames.add(top.name.text);
  }
}

const classes = new Map<string, ClassRecord>();
function classFieldType(node: ts.TypeNode, owner: ts.Node): number {
  const text = node.getText(sourceFile);
  if (text === "number") return internType({ k: "num" });
  if (text === "boolean") return internType({ k: "bool" });
  if (text === "string") return internType({ k: "str" });
  if (classNames.has(text)) return internType({ k: "cref", name: text });
  fail(owner, `field type \`${text}\` (num/bool/str/class refs in TS-2.0)`);
}
for (const top of sourceFile.statements) {
  if (!ts.isClassDeclaration(top) || !top.name) continue;
  const fields: { name: string; ty: number }[] = [];
  for (const member of top.members) {
    if (!ts.isPropertyDeclaration(member)) continue;
    if (!ts.isIdentifier(member.name)) fail(member, "a computed field name");
    if (member.initializer) fail(member, "a field initializer (assign in the constructor)");
    if (member.questionToken) fail(member, "an optional field");
    if (!member.type) fail(member, "an unannotated field");
    if (ts.canHaveModifiers(member) && ts.getModifiers(member)?.length)
      fail(member, "a field modifier (static/readonly/private fenced)");
    fields.push({ name: member.name.text, ty: classFieldType(member.type, member) });
  }
  const rowIdx = internType({
    k: "class",
    name: top.name.text,
    fields: fields.map((f) => ({ name: f.name, ty: f.ty })),
  });
  classes.set(top.name.text, { rowIdx, fields });
}

/// The recorded class of a checker type, if any (class instance types
/// carry their declaration symbol).
function classOf(type: ts.Type): ClassRecord | undefined {
  const name = type.symbol?.name;
  return name ? classes.get(name) : undefined;
}

// ---- TS-2b structural interfaces + object closures (D-075) ----

interface IfaceRecord {
  rowIdx: number;
  methods: { name: string }[];
}
const ifaces = new Map<string, IfaceRecord>();
for (const top of sourceFile.statements) {
  if (!ts.isInterfaceDeclaration(top)) continue;
  if (top.typeParameters) fail(top, "a generic interface (TS-4 territory)");
  if (top.heritageClauses) fail(top, "interface extends (fenced in TS-2)");
  const methods: { name: string; params: number[]; ret: number }[] = [];
  for (const member of top.members) {
    if (!ts.isMethodSignature(member) || !member.name || !ts.isIdentifier(member.name))
      fail(member, "an interface member that is not a method (method-only, D-075)");
    if (!member.type) fail(member, "an unannotated interface method return");
    if (member.type.getText(sourceFile) === "void")
      fail(member.type, "a void interface method (non-void returns only, D-075)");
    const params = member.parameters.map((p) => {
      if (!ts.isIdentifier(p.name)) fail(p, "a destructuring parameter");
      return annotationType(p.type, p);
    });
    methods.push({ name: member.name.text, params, ret: annotationType(member.type, member) });
  }
  if (methods.length === 0) fail(top, "an empty interface");
  const rowIdx = internType({
    k: "iface",
    name: top.name.text,
    methods: methods.map((m) => ({ name: m.name, params: m.params, ret: m.ret })),
  });
  ifaces.set(top.name.text, { rowIdx, methods });
}

function ifaceOf(type: ts.Type): IfaceRecord | undefined {
  const name = type.symbol?.name;
  return name ? ifaces.get(name) : undefined;
}

/// An expression in a COERCION position (call/return): a class value
/// flowing into an interface-typed context wraps in the conversion
/// node (D-075) — the consumer synthesizes the method-symbol list.
function coerced(node: ts.Expression): Json {
  const contextual = checker.getContextualType(node);
  if (contextual) {
    const iface = ifaceOf(contextual);
    if (iface) {
      const cls = classOf(checker.getTypeAtLocation(node));
      if (cls) {
        return {
          k: "iwrap",
          e: expr(node),
          i: iface.rowIdx,
          c: cls.rowIdx,
          span: span(node),
        };
      }
    }
  }
  return expr(node);
}

/// Is this checker type one of our recorded unions or variants?
function recordedAliasName(type: ts.Type): string | undefined {
  return type.aliasSymbol?.name;
}
function isRecordedObjectish(type: ts.Type): boolean {
  const name = recordedAliasName(type);
  if (name !== undefined && (unions.has(name) || objAliases.has(name))) return true;
  // A PARTIALLY narrowed union (e.g. the else of a three-variant
  // chain) is an anonymous subset type — the alias is gone but the
  // discriminant property still marks it as ours.
  return type.getProperty("kind") !== undefined;
}

function annotationType(node: ts.TypeNode | undefined, owner: ts.Node): number {
  if (!node) fail(owner, "a missing type annotation (TS-0 decls are fully annotated)");
  // Function types ((x: T) => R) intern structurally (D-075).
  if (ts.isFunctionTypeNode(node)) {
    const params = node.parameters.map((p) => {
      if (!ts.isIdentifier(p.name)) fail(p, "a destructuring parameter");
      return annotationType(p.type, p);
    });
    return internType({ k: "fn", params, ret: annotationType(node.type, node) });
  }
  const text = node.getText(sourceFile);
  if (text === "number") return internType({ k: "num" });
  if (text === "boolean") return internType({ k: "bool" });
  if (text === "string") return internType({ k: "str" });
  if (text === "void") return internType({ k: "void" });
  if (text === "number[]") return internType({ k: "arr", elem: internType({ k: "num" }) });
  if (text === "boolean[]") return internType({ k: "arr", elem: internType({ k: "bool" }) });
  if (text === "string[]") return internType({ k: "arr", elem: internType({ k: "str" }) });
  const union = unions.get(text);
  if (union) return union.rowIdx;
  const cls = classes.get(text);
  if (cls) return cls.rowIdx;
  const iface = ifaces.get(text);
  if (iface) return iface.rowIdx;
  if (objAliases.has(text))
    fail(node, `variant alias \`${text}\` as an annotation — annotate with its union (TS-1)`);
  fail(node, `type annotation \`${text}\``);
}

const BIN_OPS = new Map<ts.SyntaxKind, string>([
  [ts.SyntaxKind.PlusToken, "+"],
  [ts.SyntaxKind.MinusToken, "-"],
  [ts.SyntaxKind.AsteriskToken, "*"],
  [ts.SyntaxKind.SlashToken, "/"],
  [ts.SyntaxKind.PercentToken, "%"],
  [ts.SyntaxKind.LessThanToken, "<"],
  [ts.SyntaxKind.LessThanEqualsToken, "<="],
  [ts.SyntaxKind.GreaterThanToken, ">"],
  [ts.SyntaxKind.GreaterThanEqualsToken, ">="],
  [ts.SyntaxKind.EqualsEqualsEqualsToken, "==="],
  [ts.SyntaxKind.ExclamationEqualsEqualsToken, "!=="],
  [ts.SyntaxKind.AmpersandAmpersandToken, "&&"],
  [ts.SyntaxKind.BarBarToken, "||"],
]);

function expr(node: ts.Expression): Json {
  if (ts.isParenthesizedExpression(node)) return expr(node.expression);
  if (ts.isNumericLiteral(node)) {
    // Bit-exact via JS ToString: shortest round-trip digits.
    return { k: "num", v: String(Number(node.text)), span: span(node) };
  }
  if (ts.isStringLiteral(node)) return { k: "str", v: node.text, span: span(node) };
  if (ts.isArrayLiteralExpression(node)) {
    return { k: "arr", items: node.elements.map((e) => expr(e)), span: span(node) };
  }
  if (ts.isObjectLiteralExpression(node)) {
    // A union-variant construction (D-072): admitted only where the
    // contextual type names a recorded union — the consumer needs the
    // sum type, and the checker already agreed the literal fits it.
    const contextual = checker.getContextualType(node);
    const unionName = contextual && recordedAliasName(contextual);
    const union = unionName ? unions.get(unionName) : undefined;
    if (!union)
      fail(node, "an object literal outside a union-typed context (TS-1)");
    let kindValue: string | null = null;
    const byName = new Map<string, ts.Expression>();
    for (const property of node.properties) {
      if (!ts.isPropertyAssignment(property) || !ts.isIdentifier(property.name))
        fail(property, "a non-plain object-literal property (TS-1)");
      if (property.name.text === "kind") {
        if (!ts.isStringLiteral(property.initializer))
          fail(property, "a kind that is not a string literal");
        kindValue = property.initializer.text;
      } else {
        byName.set(property.name.text, property.initializer);
      }
    }
    if (kindValue === null) fail(node, "an object literal without a kind");
    const v = union.variants.findIndex((m) => m.kind === kindValue);
    if (v < 0) fail(node, `kind \`${kindValue}\` (not a variant of \`${unionName}\`)`);
    const variant = union.variants[v];
    const fields = variant.fields.map((f) => {
      const initializer = byName.get(f.name);
      if (!initializer) fail(node, `an object literal missing field \`${f.name}\``);
      return expr(initializer);
    });
    return { k: "obj", u: union.rowIdx, v, fields, span: span(node) };
  }
  if (ts.isElementAccessExpression(node)) {
    return {
      k: "index",
      a: expr(node.expression),
      i: expr(node.argumentExpression),
      span: span(node),
    };
  }
  if (node.kind === ts.SyntaxKind.ThisKeyword) {
    return { k: "var", name: "this", span: span(node) };
  }
  if (ts.isNewExpression(node)) {
    if (!ts.isIdentifier(node.expression)) fail(node, "new of a non-identifier");
    const cls = classes.get(node.expression.text);
    if (!cls) fail(node, `new of unknown class \`${node.expression.text}\``);
    return {
      k: "new",
      c: cls.rowIdx,
      args: (node.arguments ?? []).map(coerced),
      span: span(node),
    };
  }
  if (ts.isPropertyAccessExpression(node)) {
    const target = checker.getTypeAtLocation(node.expression);
    const targetClass = classOf(target);
    if (targetClass) {
      return {
        k: "prop",
        e: expr(node.expression),
        name: node.name.text,
        span: span(node),
      };
    }
    if (isRecordedObjectish(target)) {
      // Union/variant field access (D-072). The identifier underneath
      // carries its own narrow wrapper when the checker narrowed it.
      return {
        k: "prop",
        e: expr(node.expression),
        name: node.name.text,
        span: span(node),
      };
    }
    if (node.name.text === "length") {
      return { k: "len", e: expr(node.expression), span: span(node) };
    }
    fail(node, `property \`${node.name.text}\``);
  }
  if (node.kind === ts.SyntaxKind.TrueKeyword) return { k: "bool", v: true, span: span(node) };
  if (node.kind === ts.SyntaxKind.FalseKeyword) return { k: "bool", v: false, span: span(node) };
  if (ts.isIdentifier(node)) {
    const variable: Json = { k: "var", name: node.text, span: span(node) };
    // The imported flow fact (D-072): where the checker has NARROWED a
    // union-typed name to one variant, export the fact as a narrow
    // cast annotation. The consumer re-verifies it (or demotes it to a
    // runtime check) — this is untrusted input by design.
    const symbol = checker.getSymbolAtLocation(node);
    const declaration = symbol?.valueDeclaration;
    if (symbol && declaration) {
      const declared = checker.getTypeOfSymbolAtLocation(symbol, declaration);
      const unionName = recordedAliasName(declared);
      const union = unionName ? unions.get(unionName) : undefined;
      if (union) {
        const here = checker.getTypeAtLocation(node);
        const memberName = recordedAliasName(here);
        const v = union.variants.findIndex((m) => m.aliasName === memberName);
        if (v >= 0) {
          return { k: "narrow", e: variable, u: union.rowIdx, v, span: span(node) };
        }
      }
    }
    return variable;
  }
  if (ts.isBinaryExpression(node)) {
    const op = BIN_OPS.get(node.operatorToken.kind);
    if (!op) fail(node, `operator \`${node.operatorToken.getText(sourceFile)}\``);
    return { k: "bin", op, l: expr(node.left), r: expr(node.right), span: span(node) };
  }
  if (ts.isPrefixUnaryExpression(node)) {
    if (node.operator === ts.SyntaxKind.MinusToken)
      return { k: "un", op: "-", e: expr(node.operand), span: span(node) };
    if (node.operator === ts.SyntaxKind.ExclamationToken)
      return { k: "un", op: "!", e: expr(node.operand), span: span(node) };
    fail(node, "a prefix operator");
  }
  if (ts.isConditionalExpression(node)) {
    return {
      k: "cond",
      c: expr(node.condition),
      t: expr(node.whenTrue),
      e: expr(node.whenFalse),
      span: span(node),
    };
  }
  if (ts.isArrowFunction(node)) {
    // Object closures (D-075): annotated params, EXPRESSION body,
    // captures computed here (tsc knows the bindings) — parameters
    // by value, let-locals by their box, downstream.
    if (!node.type) fail(node, "an arrow without a return annotation");
    if (ts.isBlock(node.body)) fail(node, "a block-bodied arrow (expression bodies only)");
    const params: Json[] = node.parameters.map((p) => {
      if (!ts.isIdentifier(p.name)) fail(p, "a destructuring parameter");
      if (p.questionToken || p.initializer) fail(p, "an optional/defaulted parameter");
      return { name: p.name.text, ty: annotationType(p.type, p) };
    });
    const captures: string[] = [];
    const visit = (n: ts.Node): void => {
      if (n.kind === ts.SyntaxKind.ThisKeyword)
        fail(n, "`this` inside an arrow (fenced, D-075)");
      if (ts.isIdentifier(n)) {
        const symbol = checker.getSymbolAtLocation(n);
        const declaration = symbol?.valueDeclaration;
        if (
          declaration &&
          (ts.isVariableDeclaration(declaration) || ts.isParameter(declaration)) &&
          (declaration.getEnd() < node.getStart(sourceFile) ||
            declaration.getStart(sourceFile) > node.end) &&
          !captures.includes(n.text)
        ) {
          captures.push(n.text);
        }
      }
      n.forEachChild(visit);
    };
    visit(node.body);
    return {
      k: "arrow",
      params,
      ret: annotationType(node.type, node),
      e: expr(node.body),
      captures,
      span: span(node),
    };
  }
  if (ts.isCallExpression(node)) {
    // Method calls (TS-2): obj.m(args) on class- or interface-typed
    // receivers.
    if (ts.isPropertyAccessExpression(node.expression)) {
      const receiver = node.expression.expression;
      const receiverType = checker.getTypeAtLocation(receiver);
      const cls = classOf(receiverType);
      if (cls) {
        return {
          k: "mcall",
          e: expr(receiver),
          c: cls.rowIdx,
          m: node.expression.name.text,
          args: node.arguments.map(coerced),
          span: span(node),
        };
      }
      const iface = ifaceOf(receiverType);
      if (iface) {
        return {
          k: "imcall",
          e: expr(receiver),
          i: iface.rowIdx,
          m: node.expression.name.text,
          args: node.arguments.map(coerced),
          span: span(node),
        };
      }
    }
    if (!ts.isIdentifier(node.expression)) fail(node, "a non-identifier callee");
    // A call through a closure-typed VALUE (param or let) is apply,
    // not a direct call (D-075).
    const calleeSymbol = checker.getSymbolAtLocation(node.expression);
    const calleeDecl = calleeSymbol?.valueDeclaration;
    if (calleeDecl && (ts.isVariableDeclaration(calleeDecl) || ts.isParameter(calleeDecl))) {
      return {
        k: "fcall",
        f: expr(node.expression),
        args: node.arguments.map(coerced),
        span: span(node),
      };
    }
    return {
      k: "call",
      name: node.expression.text,
      args: node.arguments.map(coerced),
      span: span(node),
    };
  }
  fail(node, `expression kind ${ts.SyntaxKind[node.kind]}`);
}

function isConsoleLog(node: ts.Expression): node is ts.CallExpression {
  return (
    ts.isCallExpression(node) &&
    ts.isPropertyAccessExpression(node.expression) &&
    ts.isIdentifier(node.expression.expression) &&
    node.expression.expression.text === "console" &&
    node.expression.name.text === "log"
  );
}

/// The printed argument's static type decides the print op downstream.
function logKind(node: ts.Expression): string {
  const type = checker.getTypeAtLocation(node);
  if (type.flags & (ts.TypeFlags.Number | ts.TypeFlags.NumberLiteral)) return "num";
  if (type.flags & (ts.TypeFlags.Boolean | ts.TypeFlags.BooleanLiteral)) return "bool";
  if (type.flags & (ts.TypeFlags.String | ts.TypeFlags.StringLiteral)) return "str";
  // A union discriminant reads as a union of string literals (D-072).
  if (type.isUnion() && type.types.every((t) => t.flags & ts.TypeFlags.StringLiteral))
    return "str";
  fail(node, `console.log of type \`${checker.typeToString(type)}\``);
}

function stmt(node: ts.Statement): Json {
  if (ts.isExpressionStatement(node)) {
    if (isConsoleLog(node.expression)) {
      const arg = node.expression.arguments[0];
      if (!arg || node.expression.arguments.length !== 1)
        fail(node, "console.log with != 1 argument");
      return { k: "log", ty: logKind(arg), e: expr(arg), span: span(node) };
    }
    if (
      ts.isBinaryExpression(node.expression) &&
      node.expression.operatorToken.kind === ts.SyntaxKind.EqualsToken
    ) {
      const target = node.expression.left;
      if (ts.isElementAccessExpression(target)) {
        return {
          k: "iset",
          a: expr(target.expression),
          i: expr(target.argumentExpression),
          e: expr(node.expression.right),
          span: span(node),
        };
      }
      if (ts.isPropertyAccessExpression(target)) {
        // Field mutation (TS-2.0): obj.f = e / this.f = e.
        const cls = classOf(checker.getTypeAtLocation(target.expression));
        if (!cls) fail(target, "property assignment to a non-class value");
        return {
          k: "pset",
          e: expr(target.expression),
          name: target.name.text,
          v: expr(node.expression.right),
          span: span(node),
        };
      }
      if (!ts.isIdentifier(target)) fail(node, "assignment to a non-identifier");
      return {
        k: "assign",
        name: target.text,
        e: expr(node.expression.right),
        span: span(node),
      };
    }
    return { k: "expr", e: expr(node.expression), span: span(node) };
  }
  if (ts.isVariableStatement(node)) {
    const decls = node.declarationList.declarations;
    if (decls.length !== 1) fail(node, "a multi-declarator let");
    const decl = decls[0];
    if (!ts.isIdentifier(decl.name)) fail(node, "a destructuring let");
    if (!decl.initializer) fail(node, "a let without an initializer");
    // const vs let both accepted; mutation legality is tsc's problem
    // (checker-as-oracle), boxes are ours. FENCE (D-072): union-typed
    // locals — box reads have no SSA identity, so their narrow facts
    // would silently demote; admit when a case needs them, with the
    // demotion named.
    const initType = checker.getTypeAtLocation(decl.initializer);
    if (isRecordedObjectish(initType))
      fail(node, "a union-typed local (TS-1: unions live in parameters and expressions)");
    if (ifaceOf(initType))
      fail(node, "an interface-typed local (D-075: iface values are borrows — params only)");
    return {
      k: "let",
      name: decl.name.text,
      e: expr(decl.initializer),
      span: span(node),
    };
  }
  if (ts.isIfStatement(node)) {
    return {
      k: "if",
      c: expr(node.expression),
      then: block(node.thenStatement),
      else: node.elseStatement ? block(node.elseStatement) : null,
      span: span(node),
    };
  }
  if (ts.isWhileStatement(node)) {
    return { k: "while", c: expr(node.expression), body: block(node.statement), span: span(node) };
  }
  if (ts.isReturnStatement(node)) {
    return {
      k: "ret",
      e: node.expression ? coerced(node.expression) : null,
      span: span(node),
    };
  }
  fail(node, `statement kind ${ts.SyntaxKind[node.kind]}`);
}

function block(node: ts.Statement): Json {
  if (ts.isBlock(node)) return node.statements.map(stmt);
  return [stmt(node)];
}

function classDecl(top: ts.ClassDeclaration): Json {
  const name = top.name!.text;
  const cls = classes.get(name)!;
  let ctor: Json = null;
  const methods: Json[] = [];
  for (const member of top.members) {
    if (ts.isPropertyDeclaration(member)) continue; // fields already interned
    if (ts.isConstructorDeclaration(member)) {
      if (ctor !== null) fail(member, "a second constructor");
      if (!member.body) fail(member, "a bodyless constructor");
      const params: Json[] = member.parameters.map((p) => {
        if (!ts.isIdentifier(p.name)) fail(p, "a destructuring parameter");
        if (p.questionToken || p.initializer) fail(p, "an optional/defaulted parameter");
        if (ts.canHaveModifiers(p) && ts.getModifiers(p)?.length)
          fail(p, "a parameter property (constructor(public x) is fenced)");
        return { name: p.name.text, ty: annotationType(p.type, p) };
      });
      // The slice constructor (D-073): a sequence of `this.f = expr`
      // covering every field exactly once. Values evaluate in SOURCE
      // order; the consumer builds the record in declaration order.
      const sets: Json[] = [];
      const seen = new Set<string>();
      for (const statement of member.body.statements) {
        if (
          !ts.isExpressionStatement(statement) ||
          !ts.isBinaryExpression(statement.expression) ||
          statement.expression.operatorToken.kind !== ts.SyntaxKind.EqualsToken ||
          !ts.isPropertyAccessExpression(statement.expression.left) ||
          statement.expression.left.expression.kind !== ts.SyntaxKind.ThisKeyword
        ) {
          fail(statement, "a constructor statement that is not `this.field = expr`");
        }
        const fieldName = statement.expression.left.name.text;
        if (seen.has(fieldName)) fail(statement, `field \`${fieldName}\` assigned twice`);
        seen.add(fieldName);
        if (statement.expression.right.kind === ts.SyntaxKind.ThisKeyword) {
          // `this.next = this` — the cycle bootstrap (D-074): the
          // consumer seeds the slot and back-patches after box_new.
          sets.push({ name: fieldName, self: true, span: span(statement) });
        } else {
          sets.push({
            name: fieldName,
            e: expr(statement.expression.right),
            span: span(statement),
          });
        }
      }
      for (const field of cls.fields) {
        if (!seen.has(field.name))
          fail(member, `constructor does not assign field \`${field.name}\``);
      }
      ctor = { params, sets };
      continue;
    }
    if (ts.isMethodDeclaration(member)) {
      if (!ts.isIdentifier(member.name)) fail(member, "a computed method name");
      if (!member.body) fail(member, "a bodyless method");
      if (member.typeParameters) fail(member, "a generic method");
      if (ts.canHaveModifiers(member) && ts.getModifiers(member)?.length)
        fail(member, "a method modifier (static/private fenced)");
      const params: Json[] = member.parameters.map((p) => {
        if (!ts.isIdentifier(p.name)) fail(p, "a destructuring parameter");
        if (p.questionToken || p.initializer) fail(p, "an optional/defaulted parameter");
        return { name: p.name.text, ty: annotationType(p.type, p) };
      });
      methods.push({
        name: member.name.text,
        params,
        ret: annotationType(member.type, member),
        body: member.body.statements.map(stmt),
        span: span(member),
      });
      continue;
    }
    fail(member, `class member kind ${ts.SyntaxKind[member.kind]}`);
  }
  if (ctor === null) fail(top, `class \`${name}\` without a constructor`);
  return { k: "class", name, ty: cls.rowIdx, ctor, methods, span: span(top) };
}

const decls: Json[] = [];
const stmts: Json[] = [];
for (const top of sourceFile.statements) {
  // Type aliases and interfaces were consumed by the type tables
  // above; they carry no runtime statements.
  if (ts.isTypeAliasDeclaration(top)) continue;
  if (ts.isInterfaceDeclaration(top)) continue;
  if (ts.isClassDeclaration(top)) {
    decls.push(classDecl(top));
    continue;
  }
  if (ts.isFunctionDeclaration(top)) {
    if (!top.name) fail(top, "an anonymous function declaration");
    if (!top.body) fail(top, "a bodyless function declaration");
    if (top.typeParameters) fail(top, "a generic function (TS-4 territory)");
    const params: Json[] = top.parameters.map((p) => {
      if (!ts.isIdentifier(p.name)) fail(p, "a destructuring parameter");
      if (p.questionToken || p.initializer) fail(p, "an optional/defaulted parameter");
      return { name: p.name.text, ty: annotationType(p.type, p) };
    });
    decls.push({
      k: "fn",
      name: top.name.text,
      params,
      ret: annotationType(top.type, top),
      body: top.body.statements.map(stmt),
      span: span(top),
    });
    continue;
  }
  stmts.push(stmt(top));
}

const doc: { [k: string]: Json } = {
  loanword: 1,
  producer: `loanword-ts 0.1.0/tsc ${ts.version}`,
  file: basename(file),
  source,
  types: typeRows,
  decls,
  stmts,
};
doc.sha256 = createHash("sha256").update(canonical(doc), "utf8").digest("hex");
process.stdout.write(canonical(doc));
