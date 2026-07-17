// The DEMOTION witness (D-072): tsc narrows through an aliased
// discriminant (TS 4.4); our dominance pass honestly cannot — the
// imported fact stays a runtime contract check, and the output still
// matches node byte for byte.
type Circle = { kind: "circle"; radius: number };
type Square = { kind: "square"; side: number };
type Shape = Circle | Square;

function pick(s: Shape): number {
  const k = s.kind;
  if (k === "circle") {
    return s.radius * 2;
  }
  return 7;
}

console.log(pick({ kind: "circle", radius: 5 }));
console.log(pick({ kind: "square", side: 3 }));
