-- explicit iterator triple: a user-authored stateless iterator
local function step(limit, current)
  current = current + 1
  if current <= limit then
    return current, current * current
  end
end
for i, sq in step, 5, 0 do
  print(i)
  print(sq)
end
-- and a parenthesized-single-call triple still works
local t = { 4, 5, 6 }
for i, v in ipairs(t) do
  print(i + v)
end
