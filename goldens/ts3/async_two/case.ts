// FIFO fairness (the panel's clock pattern): two async fns
// round-robin their awaits on the shared queue.
async function ticker(label: string): Promise<void> {
  console.log(label + "1");
  await 0;
  console.log(label + "2");
  await 0;
  console.log(label + "3");
}
async function once(): Promise<void> {
  await 0;
  console.log("once");
}
ticker("a");
once();
ticker("b");
console.log("main-end");
