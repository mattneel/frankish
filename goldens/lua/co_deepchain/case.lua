-- yield from N frames deep, including through a RECURSIVE chain and
-- a callee reached via a table method — the full stackful property.
local function leaf(depth)
  coroutine.yield("leaf-at", depth)
  return depth
end
local function descend(n)
  if n == 0 then return leaf(0) end
  local r = descend(n - 1)      -- NOT a tail call: frame must survive
  return r + 1
end
local co = coroutine.create(function()
  local r = descend(5)
  return "final", r
end)
print(coroutine.resume(co))     -- suspends 7 frames deep
print(coroutine.resume(co))     -- all 7 frames resume; arithmetic unwinds
