function add(x, y)
  return x + y
end
print(add(40, 2))

local function fact(n)
  if n == 0 then
    return 1
  end
  return n * fact(n - 1)
end
print(fact(6))

local function counter()
  local n = 0
  return function()
    n = n + 1
    return n
  end
end
local c = counter()
print(c())
print(c())
print(c())
local d = counter()
print(d())

local twice = function(f, x)
  return f(f(x))
end
print(twice(function(v) return v * 3 end, 2))
