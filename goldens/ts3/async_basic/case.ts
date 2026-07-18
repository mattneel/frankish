async function f(): Promise<number> {
  console.log("f-start");
  const v = await 21;
  console.log("f-resumed");
  return v * 2;
}
async function watcher(): Promise<void> {
  const x = await f();
  console.log(x);
}
watcher();
console.log("main-end");
