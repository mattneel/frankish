// The cycle case (D-073/D-074): self-bootstrap via `this.next = this`,
// then a two-node ring tied by mutation — recursive class types,
// recref knots, and (under rc) a live object cycle.
class Node {
  val: number;
  next: Node;
  constructor(val: number) {
    this.val = val;
    this.next = this;
  }
  link(n: Node): void {
    this.next = n;
  }
  sum3(): number {
    return this.val + this.next.val + this.next.next.val;
  }
}

const a = new Node(7);
console.log(a.sum3());
const b = new Node(11);
a.link(b);
b.link(a);
console.log(a.sum3());
console.log(a.next.next.next.val);
console.log(b.next.val);
