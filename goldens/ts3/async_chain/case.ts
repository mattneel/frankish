// The pending-subscription path (D-079 rule 3): the awaiter resumes
// exactly one tick after the resolver's completing tick.
async function slow(): Promise<number> {
  const a = await 1;
  console.log("slow-mid");
  const b = await 2;
  console.log("slow-done");
  return a + b;
}
async function waiter(): Promise<void> {
  const v = await slow();
  console.log("waiter");
  console.log(v);
}
waiter();
console.log("main-end");
