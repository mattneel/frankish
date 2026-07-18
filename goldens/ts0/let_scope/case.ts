// Block-scoped let (M34; the D-080 landmine): a let inside an if arm,
// while body, or catch arm must NOT survive its block — the outer
// binding is visible again afterwards, exactly as node scopes it.
let x: number = 1;
if (1 < 2) {
  let x: number = 50;
  console.log(x);
}
console.log(x);
let i: number = 0;
while (i < 2) {
  let x: number = 100 + i;
  console.log(x);
  i = i + 1;
}
console.log(x);
try {
  throw "x";
} catch {
  let x: number = 77;
  console.log(x);
}
console.log(x);
