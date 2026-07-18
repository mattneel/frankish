// A function whose body ENDS with throw (M34; the M27 dead-join
// class): the orphan exit block is detached, so native lowering
// agrees with interp and node instead of refusing the module.
function boom(): void {
  throw "x";
}
function pick(n: number): number {
  if (n < 0) {
    return 0 - n;
  }
  throw "positive";
}
try {
  boom();
} catch {
  console.log("caught-boom");
}
try {
  console.log(pick(-4));
  console.log(pick(4));
} catch {
  console.log("caught-pick");
}
console.log("end");
