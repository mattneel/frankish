class Counter {
  n: number;
  label: string;
  constructor(start: number, label: string) {
    this.n = start;
    this.label = label;
  }
  inc(by: number): void {
    this.n = this.n + by;
  }
  read(): number {
    return this.n;
  }
}

function double(c: Counter): void {
  c.inc(c.read());
}

const a = new Counter(10, "a");
a.inc(5);
double(a);
console.log(a.read());
console.log(a.label);
console.log(a.n);
