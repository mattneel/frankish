-- __newindex: function form observes absent-key writes; existing
-- keys raw-assign without consulting the metamethod
local count = 0
local last = nil
local watched = setmetatable({ present = 1 }, {
  __newindex = function(t, k, v)
    count = count + 1
    last = k
  end,
})
watched.present = 2       -- existing key: raw, no metamethod
watched.absent = 3        -- absent: metamethod fires, no store
print(watched.present)
print(watched.absent)
print(count)
print(last)

-- table form: absent writes redirect into the target table. NOTE
-- (oracle-verified): reads do NOT follow — front has no __index, so
-- front.alpha stays nil, and the SECOND write fires __newindex again
-- (the key is still absent in front itself).
local backing = {}
local front = setmetatable({}, { __newindex = backing })
front.alpha = 42
print(front.alpha)
print(backing.alpha)
front.alpha = 43
print(backing.alpha)
