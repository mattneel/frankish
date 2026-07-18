// TS-3a (D-076): throw/catch on the effects lane. The catch is the
// optional-binding form; mutation inside try is visible outside
// (captures by box).
let hits = 0;
try {
  hits = hits + 1;
  console.log("before");
  throw "boom";
} catch {
  hits = hits + 10;
  console.log("caught");
}
console.log(hits);
console.log("after");
