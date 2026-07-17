// TS-1 smoke: discriminated union + if/else narrowing.
type Circle = { kind: "circle"; radius: number };
type Square = { kind: "square"; side: number };
type Shape = Circle | Square;

function area(s: Shape): number {
  if (s.kind === "circle") {
    return 3.14 * s.radius * s.radius;
  } else {
    return s.side * s.side;
  }
}

function make(round: boolean, size: number): Shape {
  if (round) {
    return { kind: "circle", radius: size };
  }
  return { kind: "square", side: size };
}

console.log(area(make(true, 2)));
console.log(area(make(false, 3)));
console.log(make(true, 1).kind);
