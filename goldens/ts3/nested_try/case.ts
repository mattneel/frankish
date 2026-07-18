// Nesting: the inner catch handles; the outer stays clean; a
// rethrow from a catch reaches the outer handler.
let trail = 0;
try {
  try {
    throw "inner";
  } catch {
    trail = trail + 1;
    console.log("inner caught");
  }
  console.log("between");
  try {
    throw "again";
  } catch {
    trail = trail + 10;
    throw "rethrown";
  }
} catch {
  trail = trail + 100;
  console.log("outer caught");
}
console.log(trail);
