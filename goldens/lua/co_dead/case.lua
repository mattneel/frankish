-- resume-of-dead returns a TUPLE (false + message), not an error.
local co = coroutine.create(function(a)
  coroutine.yield(a + 1)
  return "fin"
end)
print(coroutine.resume(co, 1))
print(coroutine.resume(co))
print(coroutine.status(co))
print(coroutine.resume(co))
print(coroutine.resume(co))
