-- Proper tail calls are Lua law (5.1 manual: "a return f(args) call
-- never grows the stack") — delivered by D-063's uniform convention:
-- 100k frames deep, fixed stack, against PUC lua5.1 as the oracle.
-- rc-native runners are fenced (D-063: block-exit releases break the
-- native tail shape; release scheduling is a future rung).
-- frk-case: runners=interp,jit,lua,aot-x86_64,aot-aarch64,aot-riscv64,aot-wasm32,aot-s390x
local function loop(n)
  if n == 0 then return 0 end
  return loop(n - 1)
end
print(loop(100000))
