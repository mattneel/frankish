// Object closures (D-075): an arrow capturing a class instance
// shares the alias — mutation between calls is visible (JS law).
class Counter {
  n: number;
  constructor(n: number) {
    this.n = n;
  }
  inc(by: number): void {
    this.n = this.n + by;
  }
  read(): number {
    return this.n;
  }
}

function tally(f: (x: number) => number): number {
  return f(1) + f(10);
}

const c = new Counter(5);
const scaled = (x: number): number => c.read() * x;
console.log(tally(scaled));
c.inc(5);
console.log(tally(scaled));
const base = 100;
const shift = (x: number): number => x + base;
console.log(tally(shift));
console.log(shift(c.read()));
