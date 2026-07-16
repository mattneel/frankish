-- multi-expression RHS: non-final calls truncate, final expands,
-- shortfall nil-fills
local function two()
  return 10, 20
end
local a, b, c = two(), two()
print(a)
print(b)
print(c)
local d, e, f = two(), 5
print(d)
print(e)
print(f)
local g, h = 1
print(g)
print(h)
x, y, z = two(), two()
print(x)
print(y)
print(z)
