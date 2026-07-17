// Structural interfaces (D-075): no `implements` anywhere — shape is
// the contract. Different field layouts behind one interface; the
// itab dispatches, mutation is visible through the borrowed object.
interface Shaped {
  area(): number;
  label(): string;
}

class Circle {
  tag: string;
  r: number;
  constructor(tag: string, r: number) {
    this.tag = tag;
    this.r = r;
  }
  area(): number {
    return 3 * this.r * this.r;
  }
  label(): string {
    return this.tag;
  }
  grow(by: number): void {
    this.r = this.r + by;
  }
}

class Rect {
  w: number;
  h: number;
  name: string;
  constructor(w: number, h: number, name: string) {
    this.w = w;
    this.h = h;
    this.name = name;
  }
  area(): number {
    return this.w * this.h;
  }
  label(): string {
    return this.name;
  }
}

function describe(s: Shaped): string {
  return s.label();
}

function total(a: Shaped, b: Shaped): number {
  return a.area() + b.area();
}

const c = new Circle("round", 2);
const r = new Rect(4, 5, "boxy");
console.log(total(c, r));
console.log(describe(c) + describe(r));
c.grow(1);
console.log(total(c, r));
