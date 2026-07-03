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

// Interned type table. v1 rows: num | bool | void | fun.
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

function annotationType(node: ts.TypeNode | undefined, owner: ts.Node): number {
  if (!node) fail(owner, "a missing type annotation (TS-0 decls are fully annotated)");
  const text = node.getText(sourceFile);
  if (text === "number") return internType({ k: "num" });
  if (text === "boolean") return internType({ k: "bool" });
  if (text === "string") return internType({ k: "str" });
  if (text === "void") return internType({ k: "void" });
  if (text === "number[]") return internType({ k: "arr", elem: internType({ k: "num" }) });
  if (text === "boolean[]") return internType({ k: "arr", elem: internType({ k: "bool" }) });
  if (text === "string[]") return internType({ k: "arr", elem: internType({ k: "str" }) });
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
  if (ts.isElementAccessExpression(node)) {
    return {
      k: "index",
      a: expr(node.expression),
      i: expr(node.argumentExpression),
      span: span(node),
    };
  }
  if (ts.isPropertyAccessExpression(node)) {
    if (node.name.text === "length") {
      return { k: "len", e: expr(node.expression), span: span(node) };
    }
    fail(node, `property \`${node.name.text}\``);
  }
  if (node.kind === ts.SyntaxKind.TrueKeyword) return { k: "bool", v: true, span: span(node) };
  if (node.kind === ts.SyntaxKind.FalseKeyword) return { k: "bool", v: false, span: span(node) };
  if (ts.isIdentifier(node)) return { k: "var", name: node.text, span: span(node) };
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
  if (ts.isCallExpression(node)) {
    if (!ts.isIdentifier(node.expression)) fail(node, "a non-identifier callee");
    return {
      k: "call",
      name: node.expression.text,
      args: node.arguments.map((a) => expr(a)),
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
    // (checker-as-oracle), boxes are ours.
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
    return { k: "ret", e: node.expression ? expr(node.expression) : null, span: span(node) };
  }
  fail(node, `statement kind ${ts.SyntaxKind[node.kind]}`);
}

function block(node: ts.Statement): Json {
  if (ts.isBlock(node)) return node.statements.map(stmt);
  return [stmt(node)];
}

const decls: Json[] = [];
const stmts: Json[] = [];
for (const top of sourceFile.statements) {
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
