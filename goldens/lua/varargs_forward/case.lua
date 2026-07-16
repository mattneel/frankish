-- forwarding: `return ...` and mixed prefix + `...` in call args
local function pass(...)
  return ...
end
local function shift(first, ...)
  return ...
end
print(pass(1, 2, 3))
print(shift(1, 2, 3))
local function sum3(a, b, c)
  return a + b + c
end
local function add_head(x, ...)
  return sum3(x, ...)
end
print(add_head(100, 20, 3))
