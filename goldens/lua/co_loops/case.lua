local co = coroutine.create(function()
  local acc = 0
  while true do
    local v = coroutine.yield(acc)
    if v == nil then break end
    acc = acc + v
  end
  return "sum", acc
end)
print(coroutine.resume(co))       -- prime
print(coroutine.resume(co, 5))
print(coroutine.resume(co, 7))
print(coroutine.resume(co))       -- nil -> break
