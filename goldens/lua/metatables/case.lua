local base = { greeting = "hi" }
local derived = setmetatable({}, { __index = base })
print(derived.greeting)
print(derived.missing)
derived.greeting = "yo"
print(derived.greeting)
print(base.greeting)

local dynamic = setmetatable({}, {
  __index = function(t, k)
    return k .. "!"
  end
})
print(dynamic.boom)

local chained = setmetatable({}, { __index = derived })
print(chained.greeting)
