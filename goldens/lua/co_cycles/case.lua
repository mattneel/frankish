-- 100k yield/resume cycles at fixed shallow depth (the
-- tail_recursion precedent number): the generator loop.
local gen = coroutine.create(function()
  local i = 0
  while true do
    i = i + 1
    coroutine.yield(i)
  end
end)
local sum = 0
local n = 0
while n < 100000 do
  local ok, v = coroutine.resume(gen)
  sum = sum + v
  n = n + 1
end
print(sum)
