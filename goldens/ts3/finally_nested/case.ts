// The M33 review's TS twin of the wind suspend/merge fix (D-081):
// a finally crossed by a throw must run its WHOLE body — user calls
// after the first one AND a nested try/finally — exactly as node
// runs it. Pre-fix, the live pending cell truncated the finally
// after the first guarded call and skipped the nested wind.
// (Throw stays INLINE: a fn whose body ENDS with throw trips the
// pre-existing dead-join lowering bug — the M34 landmine.)
function say(s: string): void {
  console.log(s);
}
try {
  try {
    if (1 < 2) {
      throw "x";
    }
    say("unreached");
  } finally {
    try {
      say("inner-body");
    } finally {
      say("inner-fin");
    }
    say("f2");
    say("f3");
  }
} catch {
  console.log("caught");
}
console.log("end");
