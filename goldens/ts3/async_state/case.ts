// State across awaits (captures by box) + the awaitless async fn
// (the rule-1 correction: its promise resolves BEFORE the caller
// awaits it — one tick, not a subscription).
async function instant(): Promise<number> {
  console.log("instant-ran");
  return 7;
}
async function acc(): Promise<number> {
  let total = 0;
  const a = await instant();
  total = total + a;
  const b = await 10;
  total = total + b;
  console.log(total);
  return total;
}
async function shout(s: string): Promise<void> {
  const t = await s;
  console.log(t);
}
acc();
shout("hey");
console.log("main-end");
