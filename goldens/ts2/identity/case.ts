// Reference semantics: aliases observe writes; distinct instances
// do not. String fields ride the managed tracing (D-073).
class Cell {
  tag: string;
  v: number;
  constructor(tag: string, v: number) {
    this.tag = tag;
    this.v = v;
  }
}

function poke(c: Cell): void {
  c.v = c.v * 2;
}

const x = new Cell("x", 21);
const alias = x;
poke(alias);
console.log(x.v);
const y = new Cell("y", 21);
poke(y);
poke(y);
console.log(y.v);
console.log(x.v);
console.log(y.tag + x.tag);
