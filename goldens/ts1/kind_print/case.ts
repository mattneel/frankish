// The discriminant as a VALUE (D-072): unnarrowed reads lower to a
// tag-selected literal chain; narrowed reads are literals.
type On = { kind: "on"; level: number };
type Off = { kind: "off"; since: number };
type State = On | Off;

function label(s: State): string {
  return s.kind;
}

function describe(s: State): string {
  if (s.kind === "on") {
    return s.kind + "!";
  }
  return label(s);
}

console.log(label({ kind: "on", level: 3 }));
console.log(label({ kind: "off", since: 7 }));
console.log(describe({ kind: "on", level: 1 }));
console.log(describe({ kind: "off", since: 2 }));
