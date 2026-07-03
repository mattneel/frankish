function bump(x: number): number {
  return x + 1;
}
let counter = 40;
counter = bump(counter);
counter = bump(counter);
console.log(counter);
console.log(counter === 42);
