function greet(name: string): string {
  return "hello, " + name + "!";
}
let message = greet("wörld");
console.log(message);
console.log(message.length);
console.log(message === "hello, wörld!");
console.log(message === "hello");
console.log("😀".length);
