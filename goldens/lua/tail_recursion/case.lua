-- Proper tail calls are Lua law (5.1 manual: "a return f(args) call
-- never grows the stack") — delivered by D-063's uniform convention:
-- 100k frames deep, fixed stack, against PUC lua5.1 as the oracle.
-- rc runs UNFENCED since D-064 (tail-aware release scheduling).
local function loop(n)
  if n == 0 then return 0 end
  return loop(n - 1)
end
print(loop(100000))
