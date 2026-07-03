function sum(xs: number[]): number {
  let total = 0;
  let i = 0;
  while (i < xs.length) {
    total = total + xs[i];
    i = i + 1;
  }
  return total;
}
let data = [3, 1, 4, 1, 5, 9, 2, 6];
console.log(sum(data));
console.log(data.length);
data[0] = 100;
console.log(sum(data));
let alias = data;
alias[1] = 200;
console.log(data[1]);
