function classify(n: number): number {
  if (n < 0) {
    return -1;
  }
  if (n === 0) {
    return 0;
  }
  return 1;
}
console.log(classify(-5) + classify(0) + classify(7));
console.log(true && false || true);
console.log(!(3 > 4));
