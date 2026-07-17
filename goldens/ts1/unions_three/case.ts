// Three variants, else-if chain: the D-072 dataflow proves the final
// arm by two mask subtractions; a `!==` guard proves by ne-true.
type Circle = { kind: "circle"; radius: number };
type Square = { kind: "square"; side: number };
type Tri = { kind: "tri"; base: number; height: number };
type Shape = Circle | Square | Tri;

function area(s: Shape): number {
  if (s.kind === "circle") {
    return 3 * s.radius * s.radius;
  } else if (s.kind === "square") {
    return s.side * s.side;
  } else {
    return s.base * s.height / 2;
  }
}

function flat(s: Shape): number {
  if (s.kind !== "tri") {
    return area(s);
  }
  return s.base;
}

console.log(area({ kind: "circle", radius: 2 }));
console.log(area({ kind: "square", side: 4 }));
console.log(area({ kind: "tri", base: 6, height: 5 }));
console.log(flat({ kind: "tri", base: 9, height: 1 }));
console.log(flat({ kind: "square", side: 5 }));
