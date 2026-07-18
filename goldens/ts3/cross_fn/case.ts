// The throw unwinds THROUGH calls (D-061 guards): the suppressed
// prints prove the pending cell routes past post-call statements.
function boom(n: number): number {
  if (n > 2) {
    throw "deep";
  }
  return n * 10;
}
function middle(n: number): number {
  const v = boom(n);
  console.log("mid");
  return v + 1;
}
try {
  console.log(middle(1));
  console.log(middle(7));
  console.log("unreached");
} catch {
  console.log("caught deep");
}
