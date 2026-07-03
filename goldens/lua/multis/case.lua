-- D-058: multiple returns, destructuring, nil-fill, tail forwarding.
local function divmod(a, b)
  return a - a % b, a % b
end
local q, r = divmod(17, 5)
print(q)
print(r)

local function swap(x, y)
  return y, x
end
local a, b = swap(1, 2)
print(a)
print(b)

-- nil-fill: missing args are nil; extras drop.
local function three(x, y, z)
  print(x)
  print(y)
  print(z)
end
three(10)
three(1, 2, 3, 4)

-- destructuring past the pack nil-fills.
local m, n, o = divmod(7, 2)
print(m)
print(n)
print(o)

-- tail-position call forwards its whole pack.
local function forward(a, b)
  return divmod(a, b)
end
local fq, fr = forward(9, 4)
print(fq)
print(fr)
