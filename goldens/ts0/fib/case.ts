// The TS-0 demo (SPEC §13 M9): fib.ts → native, node as oracle.
function fib(n: number): number {
  return n < 2 ? n : fib(n - 1) + fib(n - 2);
}
console.log(fib(30));
