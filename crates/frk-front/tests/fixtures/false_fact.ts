type A = { kind: "a"; x: number };
type B = { kind: "b"; x: number };
type AB = A | B;
function get(v: AB): number {
  const k = v.kind;
  if (k === "a") {
    return v.x;
  }
  return 0;
}
console.log(get({ kind: "a", x: 5 }));
