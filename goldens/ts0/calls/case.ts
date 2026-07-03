function square(x: number): number {
  return x * x;
}
function sumSquares(a: number, b: number): number {
  return square(a) + square(b);
}
function report(n: number): void {
  console.log(n);
  console.log(n > 100);
}
report(sumSquares(3, 4));
