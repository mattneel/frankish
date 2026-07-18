// finally = wind (D-076): it runs on the normal path AND on the
// throw path, in order, exactly once.
function risky(n: number): number {
  if (n < 0) {
    throw "neg";
  }
  return n + 1;
}
try {
  try {
    console.log(risky(1));
  } finally {
    console.log("inner-finally");
  }
  try {
    console.log(risky(-1));
  } finally {
    console.log("thrown-finally");
  }
} catch {
  console.log("outer-caught");
}
console.log("end");
