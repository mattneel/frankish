-- seed.lua - the Calque b0 seed translator (M37, D-086/D-092/D-094).
--
-- Runs on the femto_lua specimen. Input: `__input`, an array of
-- {name=..., ns=..., text=...} unit files (driver-assembled; ASCII
-- only, SPEC F18). Output: the kernel-IR artifact text on stdout, one
-- print per line, beginning with the version marker (BOOTSTRAP 2).
--
-- Failure protocol: there is no `error` primitive in this host
-- language, so `fail` prints exactly one line
--   __calque_seed_error: <file>:<line>:<col>: <message>
-- and then forces a deterministic runtime trap (nil index). The driver
-- reads the marker from the captured output; a trap WITHOUT a marker
-- is a seed bug, not a diagnostic.
--
-- Scope (seed v0, frozen at the M37 self-host gate): functions with
-- explicit types (i64/bool/Str/unit/declared), const/var/assignment,
-- if (statement and expression), while, return, calls (unqualified and
-- the str/io preludes), the checked-arithmetic subset + - / %,
-- comparisons, string/int/bool literals, `pub`, comments, AND the
-- records/unions growth (D-095): `const T = struct {...}` /
-- `union(enum) {...}`, `%T{field: value}` construction with literal
-- defaults, `%T{base | field: value}` immutable update, record
-- patterns `%T{f: pat}` / `%T{}`, variant patterns, `^pin`, literal
-- sub-patterns, and `match` (tag dispatch for unions; field-test
-- chains for records; literal chains for builtins), AND Vector(T)
-- (F34/D-091: i64/Str/declared-type slots; `vector.new()` is
-- instantiated by the BINDING ANNOTATION - b0 spells types, the seed
-- never infers; `vector.push/get/set/len` resolve to per-slot-kind
-- monomorphic intrinsics in the driver's intrinsics module). Vectors
-- mutate IN PLACE through any binding (const included - like a managed
-- box); out-of-range get/set trap through the runtime row.
-- REPRESENTATION:
-- struct = managed box of its product; union = frk_adt sum; every
-- NAMED type in storage position erases to !frk_mem.recref over a box
-- (D-074/D-088.2 applied uniformly — no cycle analysis in the seed).
-- Everything else fails LOUDLY (the seed never guesses).

-- ---------------------------------------------------------------- util

local PRINTABLE = {}
local ALLPRINT = " !\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~"
for i = 1, #ALLPRINT do
  PRINTABLE[string.sub(ALLPRINT, i, i)] = true
end

-- The seed's escape set is CLOSED: Calque string escapes are
-- \n \t \r \0 \\ \" (SPEC 2), so those are the only non-printable
-- bytes that can ever reach esc(). Anything else is an internal bug.
local ESCAPES = {
  ["\n"] = "\\0A",
  ["\t"] = "\\09",
  ["\r"] = "\\0D",
  ["\000"] = "\\00",
}

-- MLIR string-attribute escaping for the closed set above.
local function esc(text)
  local out = ""
  for i = 1, #text do
    local c = string.sub(text, i, i)
    if c == "\"" then
      out = out .. "\\\""
    elseif c == "\\" then
      out = out .. "\\\\"
    elseif ESCAPES[c] ~= nil then
      out = out .. ESCAPES[c]
    elseif PRINTABLE[c] then
      out = out .. c
    else
      fail(1, 1, "internal: byte outside the seed's escape set")
    end
  end
  return out
end

local CURRENT_FILE = "?"

local function fail(line, col, msg)
  print("__calque_seed_error: " .. CURRENT_FILE .. ":" .. tostring(line) .. ":" .. tostring(col) .. ": " .. msg)
  local boom = nil
  local _ = boom[1]
end

-- ---------------------------------------------------------------- lexer

local KEYWORDS = {}
for _, k in ipairs({
  "and", "bool", "break", "const", "continue", "else", "false", "fn",
  "i64", "if", "match", "or", "pub", "return", "struct", "true",
  "union", "var", "while", "not", "try", "catch", "defer", "for",
}) do
  KEYWORDS[k] = true
end

-- Character classes as LOOKUP TABLES: the host's string ordering is
-- not part of the verified subset, so the seed never compares strings
-- with < (equality only; interning makes that a pointer compare).
local LOWER = {}
local UPPER = {}
local DIGITS = {}
local LOWER_ALPHA = "abcdefghijklmnopqrstuvwxyz"
local UPPER_ALPHA = "ABCDEFGHIJKLMNOPQRSTUVWXYZ"
local DIGIT_TEXT = "0123456789"
for i = 1, #LOWER_ALPHA do LOWER[string.sub(LOWER_ALPHA, i, i)] = true end
for i = 1, #UPPER_ALPHA do UPPER[string.sub(UPPER_ALPHA, i, i)] = true end
for i = 1, #DIGIT_TEXT do DIGITS[string.sub(DIGIT_TEXT, i, i)] = true end

local function is_ident_start(c)
  return LOWER[c] or UPPER[c] or c == "_"
end
local function is_digit(c)
  return DIGITS[c]
end
local function is_ident_char(c)
  return is_ident_start(c) or is_digit(c)
end

-- tokens: {k=kind, v=text/value, line=, col=}
-- kinds: "name", "int", "str", "kw", "punct", "eof"
local function lex(source, file)
  local toks = {}
  local i = 1
  local line = 1
  local col = 1
  local n = #source
  local function push(k, v, l, c)
    toks[#toks + 1] = { k = k, v = v, line = l, col = c }
  end
  while i <= n do
    local c = string.sub(source, i, i)
    if c == "\n" then
      i = i + 1
      line = line + 1
      col = 1
    elseif c == " " or c == "\t" or c == "\r" then
      i = i + 1
      col = col + 1
    elseif c == "/" and string.sub(source, i + 1, i + 1) == "/" then
      while i <= n and string.sub(source, i, i) ~= "\n" do
        i = i + 1
      end
    elseif c == "/" and string.sub(source, i + 1, i + 1) == "*" then
      local closed = false
      i = i + 2
      col = col + 2
      while i <= n do
        if string.sub(source, i, i) == "*" and string.sub(source, i + 1, i + 1) == "/" then
          i = i + 2
          col = col + 2
          closed = true
          break
        end
        if string.sub(source, i, i) == "\n" then
          line = line + 1
          col = 1
        else
          col = col + 1
        end
        i = i + 1
      end
      if not closed then fail(line, col, "unterminated block comment") end
    elseif is_ident_start(c) then
      local start = i
      local start_col = col
      while i <= n and is_ident_char(string.sub(source, i, i)) do
        i = i + 1
        col = col + 1
      end
      local word = string.sub(source, start, i - 1)
      if KEYWORDS[word] then
        push("kw", word, line, start_col)
      else
        push("name", word, line, start_col)
      end
    elseif is_digit(c) then
      local start = i
      local start_col = col
      while i <= n and is_digit(string.sub(source, i, i)) do
        i = i + 1
        col = col + 1
      end
      push("int", string.sub(source, start, i - 1), line, start_col)
    elseif c == "\"" then
      local start_col = col
      i = i + 1
      col = col + 1
      local bytes = ""
      local closed = false
      while i <= n do
        local b = string.sub(source, i, i)
        if b == "\n" then
          fail(line, start_col, "unterminated string literal")
        elseif b == "\\" then
          local e = string.sub(source, i + 1, i + 1)
          if e == "n" then
            bytes = bytes .. "\n"
            i = i + 2
            col = col + 2
          elseif e == "t" then
            bytes = bytes .. "\t"
            i = i + 2
            col = col + 2
          elseif e == "r" then
            bytes = bytes .. "\r"
            i = i + 2
            col = col + 2
          elseif e == "\\" then
            bytes = bytes .. "\\"
            i = i + 2
            col = col + 2
          elseif e == "\"" then
            bytes = bytes .. "\""
            i = i + 2
            col = col + 2
          elseif e == "0" then
            bytes = bytes .. "\000"
            i = i + 2
            col = col + 2
          else
            fail(line, col, "unsupported escape \\" .. e)
          end
        elseif b == "\"" then
          i = i + 1
          col = col + 1
          closed = true
          break
        else
          bytes = bytes .. b
          i = i + 1
          col = col + 1
        end
      end
      if not closed then fail(line, start_col, "unterminated string literal") end
      push("str", bytes, line, start_col)
    else
      local two = string.sub(source, i, i + 1)
      if two == "==" or two == "!=" or two == "<=" or two == ">=" or two == "|>" or two == "=>" then
        push("punct", two, line, col)
        i = i + 2
        col = col + 2
      else
        local one = string.sub(source, i, i)
        if one == "(" or one == ")" or one == "{" or one == "}" or one == ","
          or one == ";" or one == ":" or one == "." or one == "=" or one == "<"
          or one == ">" or one == "+" or one == "-" or one == "*" or one == "/"
          or one == "%" or one == "!" or one == "^" or one == "|" then
          push("punct", one, line, col)
          i = i + 1
          col = col + 1
        else
          fail(line, col, "unexpected character " .. one)
        end
      end
    end
  end
  push("eof", "", line, col)
  return toks
end

-- ---------------------------------------------------------------- parser

-- AST nodes (tables):
--   fn   = {name=, params={{name,ty}}, ret=ty|nil, body=block}
--   block= {stmts={...}, tail=expr|nil}
--   const/var = {name=, ty=, expr=}
--   assign = {name=, expr=}
--   ret = {expr=}
--   ifstmt = {cond=, then=block, els=block|nil}
--   loop = {cond=, body=block}
--   expr = literal nodes {lit="int"|"bool"|"str", text=} | {ref=name}
--        | {call={mod=?,name=,member=?}, args={...}}
--        | {block=block} | {if=...} | {bin=op, l=, r=} | (parens vanish)

local P = {} -- parser state: {toks=, pos=, file=}

local function peek()
  return P.toks[P.pos]
end
local function next_tok()
  local t = P.toks[P.pos]
  P.pos = P.pos + 1
  return t
end
local function at_punct(v)
  local t = peek()
  return t.k == "punct" and t.v == v
end
local function at_kw(v)
  local t = peek()
  return t.k == "kw" and t.v == v
end
local function expect_punct(v)
  local t = peek()
  if not (t.k == "punct" and t.v == v) then
    fail(t.line, t.col, "expected `" .. v .. "`, found `" .. t.v .. "`")
  end
  return next_tok()
end
local function expect_kw(v)
  local t = peek()
  if not (t.k == "kw" and t.v == v) then
    fail(t.line, t.col, "expected keyword `" .. v .. "`")
  end
  return next_tok()
end
local function expect_name()
  local t = peek()
  if t.k ~= "name" then
    fail(t.line, t.col, "expected an identifier")
  end
  next_tok()
  return t
end

local function parse_type()
  local t = peek()
  if t.k == "kw" and t.v == "i64" then
    next_tok()
    return "i64"
  end
  if t.k == "kw" and t.v == "bool" then
    next_tok()
    return "bool"
  end
  if t.k == "name" and t.v == "Str" then
    next_tok()
    return "Str"
  end
  if t.k == "punct" and t.v == "(" and P.toks[P.pos + 1].k == "punct" and P.toks[P.pos + 1].v == ")" then
    next_tok()
    next_tok()
    return "unit"
  end
  if t.k == "name" and t.v == "Vector" then
    next_tok()
    local lp = peek()
    if not (lp.k == "punct" and lp.v == "(") then
      fail(lp.line, lp.col, "Vector takes a slot type: write `Vector(i64)` (seed v0)")
    end
    next_tok()
    local inner = parse_type()
    local rp = peek()
    if not (rp.k == "punct" and rp.v == ")") then
      fail(rp.line, rp.col, "expected `)` after the Vector slot type")
    end
    next_tok()
    if inner ~= "i64" and inner ~= "Str" and not (type(inner) == "table" and inner.named ~= nil) then
      fail(t.line, t.col, "unsupported Vector slot (seed v0 instantiations: i64, Str, declared types)")
    end
    return { vec = inner }
  end
  if t.k == "name" then
    next_tok()
    return { named = t.v }
  end
  fail(t.line, t.col, "unsupported type (seed v0: i64, bool, Str, unit, declared types)")
end

function parse_expr(level)
  -- v0 subset: levels 4..6 (comparisons, +-, */%), unary, postfix.
  if level > 6 then
    return parse_unary()
  end
  local l = parse_expr(level + 1)
  while true do
    local t = peek()
    local op = nil
    if t.k == "punct" then
      if level == 4 and (t.v == "==" or t.v == "!=" or t.v == "<" or t.v == "<=" or t.v == ">" or t.v == ">=") then
        op = t.v
      elseif level == 5 and (t.v == "+" or t.v == "-") then
        op = t.v
      elseif level == 6 and (t.v == "*" or t.v == "/" or t.v == "%") then
        op = t.v
      end
    end
    if op == nil then
      return l
    end
    next_tok()
    local r = parse_expr(level + 1)
    l = { bin = op, l = l, r = r, line = t.line, col = t.col }
    if level == 4 then
      local nxt = peek()
      if nxt.k == "punct" and (nxt.v == "==" or nxt.v == "!=" or nxt.v == "<" or nxt.v == "<=" or nxt.v == ">" or nxt.v == ">=") then
        fail(nxt.line, nxt.col, "comparisons do not chain (SPEC 5.1)")
      end
    end
  end
end

function parse_unary()
  local t = peek()
  if t.k == "punct" and t.v == "-" then
    fail(t.line, t.col, "unary minus is not in seed v0 (write `0 - x`)")
  end
  if t.k == "punct" and t.v == "!" then
    fail(t.line, t.col, "`!` is not in seed v0")
  end
  return parse_postfix()
end

function parse_postfix()
  local e = parse_primary()
  while true do
    local t = peek()
    if t.k == "punct" and t.v == "(" then
      -- call: e must be a ref (unqualified) or a dotted ref
      local args = {}
      next_tok()
      if not at_punct(")") then
        while true do
          args[#args + 1] = parse_expr(4)
          if at_punct(",") then
            next_tok()
          else
            break
          end
        end
      end
      expect_punct(")")
      if e.ref == nil then
        fail(t.line, t.col, "callee must be a name or module member (seed v0)")
      end
      e = { call = e.ref, args = args, line = t.line, col = t.col }
    elseif t.k == "punct" and t.v == "." then
      next_tok()
      local member = expect_name()
      if e.ref == nil or e.ref.mod ~= nil then
        fail(t.line, t.col, "chained module paths are not in seed v0")
      end
      e = { ref = { name = member.v, mod = e.ref.name }, line = t.line, col = t.col }
    else
      return e
    end
  end
end

function parse_primary()
  local t = peek()
  if t.k == "int" then
    next_tok()
    return { lit = "int", text = t.v, line = t.line, col = t.col }
  end
  if t.k == "str" then
    next_tok()
    return { lit = "str", text = t.v, line = t.line, col = t.col }
  end
  if t.k == "kw" and (t.v == "true" or t.v == "false") then
    next_tok()
    return { lit = "bool", text = t.v, line = t.line, col = t.col }
  end
  if t.k == "kw" and t.v == "if" then
    return parse_if()
  end
  if t.k == "kw" and t.v == "match" then
    return parse_match()
  end
  if t.k == "punct" and t.v == "%" then
    return parse_struct_literal_or_map()
  end
  if t.k == "name" then
    next_tok()
    return { ref = { name = t.v }, line = t.line, col = t.col }
  end
  if t.k == "punct" and t.v == "(" then
    next_tok()
    local e = parse_expr(4)
    expect_punct(")")
    return e
  end
  if t.k == "punct" and t.v == "{" then
    return { block = parse_block() }
  end
  if t.k == "kw" then
    fail(t.line, t.col, "`" .. t.v .. "` is not in seed v0")
  end
  fail(t.line, t.col, "unexpected token `" .. t.v .. "`")
end

function parse_if()
  local kw = expect_kw("if")
  local cond = parse_expr(4)
  local then_blk = parse_block()
  local else_blk = nil
  if at_kw("else") then
    next_tok()
    if at_kw("if") then
      local nested = parse_if()
      else_blk = { stmts = {}, tail = nested }
    else
      else_blk = parse_block()
    end
  end
  return { iff = { cond = cond, yes = then_blk, els = else_blk }, line = kw.line, col = kw.col }
end

function parse_struct_literal_or_map()
  local pct = next_tok() -- %
  if at_punct("{") then
    fail(pct.line, pct.col, "maps are C2 (D-095): seed v0 has `%Type{...}` structs only")
  end
  local name = expect_name()
  expect_punct("{")
  if at_punct("}") then
    next_tok()
    return { make = { ty = name.v, fields = {}, base = nil }, line = pct.line, col = pct.col }
  end
  local base = nil
  local fields = {}
  -- field list iff NAME followed by `:` / `,` / `}`; otherwise an update base.
  local a = peek()
  local b = P.toks[P.pos + 1]
  local is_field_list = false
  if a.k == "name" and b ~= nil and b.k == "punct" and (b.v == ":" or b.v == "," or b.v == "}") then
    is_field_list = true
  end
  if not is_field_list then
    base = parse_expr(4)
    expect_punct("|")
  end
  while not at_punct("}") do
    local f = expect_name()
    if at_punct(":") then
      next_tok()
      local e = parse_expr(4)
      fields[#fields + 1] = { name = f.v, expr = e, line = f.line, col = f.col }
    else
      -- Elixir shorthand: `%T{x}` binds field x from the local x.
      fields[#fields + 1] = {
        name = f.v,
        expr = { ref = { name = f.v }, line = f.line, col = f.col },
        line = f.line,
        col = f.col,
      }
    end
    if at_punct(",") then
      next_tok()
    else
      break
    end
  end
  expect_punct("}")
  return { make = { ty = name.v, fields = fields, base = base }, line = pct.line, col = pct.col }
end

function parse_match()
  local kw = next_tok() -- match
  local scrut = parse_expr(4)
  expect_punct("{")
  local arms = {}
  if at_punct("}") then
    fail(kw.line, kw.col, "a match needs at least one arm")
  end
  while true do
    if at_punct("}") then
      break
    end
    local pat = parse_pattern()
    expect_punct("=>")
    local body = parse_expr(4)
    arms[#arms + 1] = { pat = pat, body = body, line = pat.line, col = pat.col }
    if at_punct(",") then
      next_tok()
    else
      break
    end
  end
  expect_punct("}")
  return { match = { scrut = scrut, arms = arms }, line = kw.line, col = kw.col }
end

function parse_pattern()
  local t = peek()
  if t.k == "name" and t.v == "_" then
    next_tok()
    return { pat = "wild", line = t.line, col = t.col }
  end
  if t.k == "punct" and t.v == "^" then
    next_tok()
    local n = expect_name()
    return { pat = "pin", name = n.v, line = t.line, col = t.col }
  end
  if t.k == "int" then
    next_tok()
    return { pat = "lit", litty = "i64", text = t.v, line = t.line, col = t.col }
  end
  if t.k == "str" then
    next_tok()
    return { pat = "lit", litty = "Str", text = t.v, line = t.line, col = t.col }
  end
  if t.k == "kw" and (t.v == "true" or t.v == "false") then
    next_tok()
    return { pat = "lit", litty = "bool", text = t.v, line = t.line, col = t.col }
  end
  if t.k == "punct" and t.v == "%" then
    next_tok()
    local name = expect_name()
    expect_punct("{")
    local fields = {}
    while not at_punct("}") do
      local f = expect_name()
      if at_punct(":") then
        next_tok()
        local sub = parse_pattern()
        fields[#fields + 1] = { name = f.v, sub = sub, line = f.line, col = f.col }
      else
        fields[#fields + 1] = {
          name = f.v,
          sub = { pat = "bind", name = f.v, line = f.line, col = f.col },
          line = f.line,
          col = f.col,
        }
      end
      if at_punct(",") then
        next_tok()
      else
        break
      end
    end
    expect_punct("}")
    return { pat = "rec", ty = name.v, fields = fields, line = t.line, col = t.col }
  end
  if t.k == "name" then
    next_tok()
    local nxt = peek()
    if nxt.k == "punct" and nxt.v == "." then
      next_tok()
      local v = expect_name()
      local sub = nil
      if at_punct("(") then
        next_tok()
        sub = parse_pattern()
        expect_punct(")")
      end
      return { pat = "variant", ty = t.v, variant = v.v, sub = sub, line = t.line, col = t.col }
    end
    return { pat = "bind", name = t.v, line = t.line, col = t.col }
  end
  fail(t.line, t.col, "unsupported pattern (seed v0)")
end

function parse_block()
  expect_punct("{")
  local stmts = {}
  local tail = nil
  while true do
    if at_punct("}") then
      next_tok()
      break
    end
    local t = peek()
    if t.k == "kw" and t.v == "const" then
      next_tok()
      local name = expect_name()
      local ty = nil
      if at_punct(":") then
        next_tok()
        ty = parse_type()
      end
      expect_punct("=")
      local e = parse_expr(4)
      expect_punct(";")
      stmts[#stmts + 1] = { const = { name = name.v, ty = ty, expr = e, line = name.line, col = name.col } }
    elseif t.k == "kw" and t.v == "var" then
      next_tok()
      local name = expect_name()
      expect_punct(":")
      local ty = parse_type()
      expect_punct("=")
      local e = parse_expr(4)
      expect_punct(";")
      stmts[#stmts + 1] = { var = { name = name.v, ty = ty, expr = e, line = name.line, col = name.col } }
    elseif t.k == "kw" and t.v == "return" then
      next_tok()
      if at_punct(";") then
        next_tok()
        stmts[#stmts + 1] = { ret = { expr = nil, line = t.line, col = t.col } }
      else
        local e = parse_expr(4)
        expect_punct(";")
        stmts[#stmts + 1] = { ret = { expr = e, line = t.line, col = t.col } }
      end
    elseif t.k == "kw" and t.v == "while" then
      next_tok()
      local cond = parse_expr(4)
      local body = parse_block()
      stmts[#stmts + 1] = { loop = { cond = cond, body = body, line = t.line, col = t.col } }
    elseif t.k == "kw" and t.v == "if" then
      local iff = parse_if()
      stmts[#stmts + 1] = { ifstmt = iff.iff }
    elseif t.k == "name" and P.toks[P.pos + 1].k == "punct" and P.toks[P.pos + 1].v == "=" then
      local name = next_tok()
      next_tok() -- `=`
      local e = parse_expr(4)
      expect_punct(";")
      stmts[#stmts + 1] = { assign = { name = name.v, expr = e, line = name.line, col = name.col } }
    elseif t.k == "kw" and t.v == "match" then
      local e = parse_expr(4)
      if at_punct(";") then
        next_tok()
        stmts[#stmts + 1] = { expr = e }
      else
        tail = e
        expect_punct("}")
        break
      end
    elseif t.k == "kw" then
      fail(t.line, t.col, "`" .. t.v .. "` is not in seed v0")
    else
      local e = parse_expr(4)
      if at_punct(";") then
        next_tok()
        stmts[#stmts + 1] = { expr = e }
      else
        tail = e
        expect_punct("}")
        break
      end
    end
  end
  return { stmts = stmts, tail = tail }
end

local function parse_type_decl()
  if at_kw("pub") then
    next_tok()
  end
  expect_kw("const")
  local name = expect_name()
  expect_punct("=")
  local t = peek()
  if at_kw("struct") then
    next_tok()
    expect_punct("{")
    local fields = {}
    while not at_punct("}") do
      local f = expect_name()
      expect_punct(":")
      local ty = parse_type()
      local default = nil
      if at_punct("=") then
        next_tok()
        local d = peek()
        if d.k == "int" then
          default = { lit = "int", text = d.v, line = d.line, col = d.col }
          next_tok()
        elseif d.k == "str" then
          default = { lit = "str", text = d.v, line = d.line, col = d.col }
          next_tok()
        elseif d.k == "kw" and (d.v == "true" or d.v == "false") then
          default = { lit = "bool", text = d.v, line = d.line, col = d.col }
          next_tok()
        else
          fail(d.line, d.col, "seed v0 supports literal field defaults only (D-095)")
        end
      end
      fields[#fields + 1] = { name = f.v, ty = ty, default = default, line = f.line, col = f.col }
      if at_punct(",") then
        next_tok()
      else
        break
      end
    end
    expect_punct("}")
    expect_punct(";")
    return { name = name.v, kind = "struct", fields = fields, line = name.line, col = name.col }
  end
  if at_kw("union") then
    next_tok()
    expect_punct("(")
    local e = expect_name()
    if e.v ~= "enum" then
      fail(e.line, e.col, "expected `union(enum)` (SPEC 3)")
    end
    expect_punct(")")
    expect_punct("{")
    local variants = {}
    while not at_punct("}") do
      local v = expect_name()
      local payload = nil
      if at_punct(":") then
        next_tok()
        payload = parse_type()
      end
      variants[#variants + 1] = { name = v.v, ty = payload, line = v.line, col = v.col }
      if at_punct(",") then
        next_tok()
      else
        break
      end
    end
    expect_punct("}")
    expect_punct(";")
    return { name = name.v, kind = "union", variants = variants, line = name.line, col = name.col }
  end
  fail(t.line, t.col, "seed v0 type declarations are `struct {...}` or `union(enum) {...}`")
end

local function parse_fn()
  if at_kw("pub") then
    next_tok()
  end
  expect_kw("fn")
  local name = expect_name()
  expect_punct("(")
  local params = {}
  if not at_punct(")") then
    while true do
      local p = expect_name()
      expect_punct(":")
      local ty = parse_type()
      params[#params + 1] = { name = p.v, ty = ty, line = p.line, col = p.col }
      if at_punct(",") then
        next_tok()
      else
        break
      end
    end
  end
  expect_punct(")")
  local ret = "unit"
  if not at_punct("{") then
    ret = parse_type()
  end
  local body = parse_block()
  return { name = name.v, params = params, ret = ret, body = body, line = name.line, col = name.col }
end

local function parse_file(source, file)
  P = { toks = lex(source, file), pos = 1, file = file }
  local fns = {}
  local types = {}
  while peek().k ~= "eof" do
    local t = peek()
    if t.k == "kw" and t.v == "const" then
      types[#types + 1] = parse_type_decl()
    elseif t.k == "kw" and t.v == "pub" and P.toks[P.pos + 1].k == "kw" and P.toks[P.pos + 1].v == "const" then
      types[#types + 1] = parse_type_decl()
    elseif t.k == "kw" and (t.v == "pub" or t.v == "fn") then
      fns[#fns + 1] = parse_fn()
    else
      fail(t.line, t.col, "seed v0 accepts top-level fn declarations and struct/union type declarations (got `" .. t.v .. "`)")
    end
  end
  return { fns = fns, types = types }
end

-- ---------------------------------------------------------------- emitter

local TY_IR = { i64 = "i64", bool = "i1", Str = "!frk_bstr.str" }
local TY_BOX = { i64 = "!frk_mem.box<i64>", bool = "!frk_mem.box<i1>", Str = "!frk_mem.box<!frk_bstr.str>" }

local E = {}
-- E fields: out (artifact lines), blocks (list of {label, lines}), cur,
-- tmp counter, fn context (symbol, ret, scopes, loops), unit fns table.

local function fresh()
  E.tmp = E.tmp + 1
  return "%t" .. tostring(E.tmp)
end

local function blk()
  return E.blocks[E.cur].lines
end

local function emit(line)
  if E.terminated then
    fail(1, 1, "internal: emission after terminator")
  end
  local lines = blk()
  lines[#lines + 1] = line
end

local function new_block(label)
  E.blocks[#E.blocks + 1] = { label = label, lines = {} }
  E.cur = #E.blocks
  E.terminated = false
end

local function terminate(line)
  emit(line)
  E.terminated = true
end

local function fresh_label()
  E.label = E.label + 1
  return "b" .. tostring(E.label)
end

local function scope_push()
  E.scopes[#E.scopes + 1] = {}
end
local function scope_pop()
  E.scopes[#E.scopes] = nil
end
local function declare(name, binding, line, col)
  local top = E.scopes[#E.scopes]
  if top[name] ~= nil then
    fail(line, col, "duplicate binding `" .. name .. "` in the same scope")
  end
  if name == "str" or name == "io" or name == "vector" then
    fail(line, col, "`" .. name .. "` is a reserved prelude name")
  end
  top[name] = binding
end
local function lookup(name)
  local i = #E.scopes
  while i >= 1 do
    if E.scopes[i][name] ~= nil then
      return E.scopes[i][name]
    end
    i = i - 1
  end
  return nil
end

local function ir_ty(ty, line, col)
  if is_vec(ty) then
    return vec_box_ir(ty.vec, line, col)
  end
  local mapped = TY_IR[ty]
  if mapped == nil then
    fail(line, col, "type `" .. ty .. "` has no IR mapping (seed v0)")
  end
  return mapped
end

local function guard(cond, message, line, col)
  -- cond: i1 SSA. Emits: cond_br cond, ^trap, ^ok; trap composes the
  -- message and aborts; ^ok continues.
  local trap_label = fresh_label()
  local ok_label = fresh_label()
  emit("cf.cond_br " .. cond .. ", ^" .. trap_label .. ", ^" .. ok_label)
  new_block(trap_label)
  local msg = fresh()
  emit(msg .. " = \"frk_bstr.lit\"() {text = \"" .. esc(message) .. "\"} : () -> !frk_bstr.str")
  emit("func.call @frk_rt_calque_trap(" .. msg .. ") : (!frk_bstr.str) -> ()")
  if E.ret == "unit" then
    terminate("func.return")
  else
    terminate("func.return " .. poison(E.ret, line, col) .. " : " .. value_ir(E.ret, line, col))
  end
  new_block(ok_label)
end

local function loc_msg(line, col, code, what)
  return CURRENT_FILE .. ":" .. tostring(line) .. ":" .. tostring(col) .. ": error: " .. code .. ": " .. what
end

local function const_i64(text_val)
  local v = fresh()
  emit(v .. " = arith.constant " .. text_val .. " : i64")
  return v
end

local function emit_checked(op, a, b, line, col)
  local res = fresh()
  local zero = const_i64("0")
  if op == "+" or op == "-" then
    emit(res .. " = arith." .. (op == "+" and "addi" or "subi") .. " " .. a .. ", " .. b .. " : i64")
    local x1 = fresh()
    local x2 = fresh()
    local x3 = fresh()
    local neg = fresh()
    if op == "+" then
      emit(x1 .. " = arith.xori " .. a .. ", " .. res .. " : i64")
      emit(x2 .. " = arith.xori " .. b .. ", " .. res .. " : i64")
    else
      emit(x1 .. " = arith.xori " .. a .. ", " .. res .. " : i64")
      emit(x2 .. " = arith.xori " .. a .. ", " .. b .. " : i64")
    end
    emit(x3 .. " = arith.andi " .. x1 .. ", " .. x2 .. " : i64")
    emit(neg .. " = arith.cmpi slt, " .. x3 .. ", " .. zero .. " : i64")
    local code = op == "+" and "CQ0001" or "CQ0002"
    local what = op == "+" and "integer overflow (i64 add)" or "integer overflow (i64 sub)"
    guard(neg, loc_msg(line, col, code, what), line, col)
    return res
  end
  -- division/remainder: zero divisor and MIN/-1 overflow checks
  local zero2 = const_i64("0")
  local is_zero = fresh()
  emit(is_zero .. " = arith.cmpi eq, " .. b .. ", " .. zero2 .. " : i64")
  local what = op == "/" and "division by zero" or "remainder by zero"
  guard(is_zero, loc_msg(line, col, "CQ0004", what), line, col)
  -- MIN/-1: derive MIN as (0 - MAX) - 1 (raw ops; both wrap-safe)
  local maxlit = const_i64("9223372036854775807")
  local one = const_i64("1")
  local m1 = fresh()
  local minv = fresh()
  local minus1 = fresh()
  emit(m1 .. " = arith.subi " .. zero2 .. ", " .. maxlit .. " : i64")
  emit(minv .. " = arith.subi " .. m1 .. ", " .. one .. " : i64")
  emit(minus1 .. " = arith.subi " .. zero2 .. ", " .. one .. " : i64")
  local is_min = fresh()
  local is_m1 = fresh()
  local both = fresh()
  emit(is_min .. " = arith.cmpi eq, " .. a .. ", " .. minv .. " : i64")
  emit(is_m1 .. " = arith.cmpi eq, " .. b .. ", " .. minus1 .. " : i64")
  emit(both .. " = arith.andi " .. is_min .. ", " .. is_m1 .. " : i1")
  local what2 = op == "/" and "division overflow (i64 min / -1)" or "remainder overflow (i64 min % -1)"
  guard(both, loc_msg(line, col, "CQ0005", what2), line, col)
  emit(res .. " = arith." .. (op == "/" and "divsi" or "remsi") .. " " .. a .. ", " .. b .. " : i64")
  return res
end

local function cmp_pred(op)
  if op == "==" then return "eq" end
  if op == "!=" then return "ne" end
  if op == "<" then return "slt" end
  if op == "<=" then return "sle" end
  if op == ">" then return "sgt" end
  return "sge"
end

local function emit_cmp(op, a, b)
  local res = fresh()
  local pred = cmp_pred(op)
  emit(res .. " = arith.cmpi " .. pred .. ", " .. a .. ", " .. b .. " : i64")
  return res
end

-- ------------------------------- expressions

local emit_expr = nil -- forward (initializer-required locals)

-- --------------------- declared types, records, unions (D-095/D-088)
--
-- REPRESENTATION (seed v0, uniform and conservative): a struct is a
-- managed box of its field product; a union is a frk_adt sum. Any
-- NAMED type in STORAGE position (struct field, union payload) is
-- erased to !frk_mem.recref over a box — the D-074/D-088.2 recipe
-- applied UNIFORMLY (no cycle analysis in the seed; C2 may refine to
-- inline non-recursive nesting). Every named local/param/return is a
-- VALUE: struct = its box, union = its sum; storage helpers wrap
-- (rec_ref, boxing unions first) and unwrap (rec_cast + box_get).

function find_type(name, line, col)
  local d = E.types[name]
  if d == nil then
    fail(line, col, "unknown type `" .. name .. "`")
  end
  return d
end

function is_named(ty)
  return ty ~= nil and type(ty) == "table" and ty.named ~= nil
end

function is_vec(ty)
  return ty ~= nil and type(ty) == "table" and ty.vec ~= nil
end

function same_type(a, b)
  if is_named(a) or is_named(b) then
    return is_named(a) and is_named(b) and a.named == b.named
  end
  if is_vec(a) or is_vec(b) then
    return is_vec(a) and is_vec(b) and same_type(a.vec, b.vec)
  end
  return a == b
end

-- Vector(T) (F34/D-091): a managed box of [arr<slot>, len, cap];
-- vector.* resolves to the per-slot-kind intrinsics module functions.
function vec_slot(inner, line, col)
  if inner == "i64" then
    return "i64", "__calque_vector_i64"
  end
  if inner == "Str" then
    return "!frk_bstr.str", "__calque_vector_bstr"
  end
  if is_named(inner) then
    find_type(inner.named, line, col)
    return "!frk_mem.recref", "__calque_vector_box"
  end
  fail(line, col, "unsupported Vector slot (seed v0 instantiations: i64, Str, declared types)")
end

function vec_box_ir(inner, line, col)
  local slot = vec_slot(inner, line, col)
  return "!frk_mem.box<!frk_adt.product<[!frk_mem.arr<" .. slot .. ">, i64, i64]>>"
end

-- `vector.new()` is instantiated by its declaration's annotation
-- (syntax-directed: b0 spells types; the seed never infers).
function is_vec_new_call(expr)
  return expr ~= nil and expr.call ~= nil and expr.args ~= nil
    and expr.call.mod == "vector" and expr.call.name == "new" and #expr.args == 0
end

function emit_vec_new(inner, line, col)
  local slot, prefix = vec_slot(inner, line, col)
  local res = fresh()
  emit(res .. " = func.call @" .. prefix .. "_new() : () -> " .. vec_box_ir(inner, line, col))
  E.vtypes[res] = { vec = inner }
  return res
end

function join_parts(parts)
  local out = ""
  for i = 1, #parts do
    if i > 1 then
      out = out .. ", "
    end
    out = out .. parts[i]
  end
  return out
end

function storage_ir(ty, line, col)
  if is_vec(ty) then
    return vec_box_ir(ty.vec, line, col)
  end
  if is_named(ty) then
    find_type(ty.named, line, col)
    return "!frk_mem.recref"
  end
  if ty == "unit" then
    fail(line, col, "unit-typed storage is not in seed v0")
  end
  return ir_ty(ty, line, col)
end

function product_ir(decl)
  local parts = {}
  for i = 1, #decl.fields do
    parts[#parts + 1] = storage_ir(decl.fields[i].ty, decl.fields[i].line, decl.fields[i].col)
  end
  return "!frk_adt.product<[" .. join_parts(parts) .. "]>"
end

function value_ir(ty, line, col)
  if is_vec(ty) then
    return vec_box_ir(ty.vec, line, col)
  end
  if not is_named(ty) then
    return ir_ty(ty, line, col)
  end
  local d = find_type(ty.named, line, col)
  if d.kind == "struct" then
    return "!frk_mem.box<" .. product_ir(d) .. ">"
  end
  local parts = {}
  for i = 1, #d.variants do
    local v = d.variants[i]
    if v.ty == nil then
      parts[#parts + 1] = "[]"
    else
      parts[#parts + 1] = "[" .. storage_ir(v.ty, v.line, v.col) .. "]"
    end
  end
  return "!frk_adt.sum<[" .. join_parts(parts) .. "]>"
end

function box_ir(ty, line, col)
  return "!frk_mem.box<" .. value_ir(ty, line, col) .. ">"
end

-- Terminating poison for the unreachable arms of emitted traps: builtins
-- become constants, structs a box of storage-poisoned fields, unions a
-- sum of variant 0's storage poison (named storage positions are
-- recref_null — no recursion).
function poison(ty, line, col)
  line = line or 1
  col = col or 1
  if not is_named(ty) then
    if ty == "i64" then
      local v = fresh()
      emit(v .. " = arith.constant 0 : i64")
      return v
    end
    if ty == "bool" then
      local v = fresh()
      emit(v .. " = arith.constant false")
      return v
    end
    if ty == "Str" then
      local v = fresh()
      emit(v .. " = \"frk_bstr.lit\"() {text = \"\"} : () -> !frk_bstr.str")
      return v
    end
    fail(line, col, "internal: no poison for `" .. tostring(ty) .. "`")
  end
  local d = find_type(ty.named, line, col)
  if d.kind == "struct" then
    local prod = emit_product(d.fields, function(field)
      if is_named(field.ty) then
        local r = fresh()
        emit(r .. " = \"frk_mem.recref_null\"() : () -> !frk_mem.recref")
        return r, "!frk_mem.recref"
      end
      local v = poison(field.ty, line, col)
      return v, storage_ir(field.ty, line, col)
    end)
    local b = fresh()
    emit(b .. " = \"frk_mem.box_new\"(" .. prod.value .. ") : (" .. prod.ty .. ") -> " .. value_ir(ty, line, col))
    return b
  end
  -- union: variant 0's product, storage-poisoned
  local v0 = d.variants[1]
  local prod = nil
  if v0.ty == nil then
    local e = fresh()
    emit(e .. " = \"frk_adt.product_new\"() : () -> !frk_adt.product<[]>")
    prod = { value = e, ty = "!frk_adt.product<[]>" }
  else
    local value = nil
    if is_named(v0.ty) then
      value = fresh()
      emit(value .. " = \"frk_mem.recref_null\"() : () -> !frk_mem.recref")
    else
      value = poison(v0.ty, line, col)
    end
    prod = emit_product_single(value, storage_ir(v0.ty, v0.line, v0.col))
  end
  local s = fresh()
  emit(s .. " = \"frk_adt.make_sum\"(" .. prod.value .. ") {variant = 0 : i64} : (" .. prod.ty .. ") -> " .. value_ir(ty, line, col))
  return s
end

-- Builds a product from a list of field declarations + a value producer
-- callback (returns value ssa, storage type). Returns {value, ty}.
function emit_product(fields, produce)
  local acc = fresh()
  emit(acc .. " = \"frk_adt.product_new\"() : () -> !frk_adt.product<[]>")
  local acc_ty = "!frk_adt.product<[]>"
  local parts = {}
  for i = 1, #fields do
    local v, st = produce(fields[i], i)
    parts[#parts + 1] = st
    local next = fresh()
    emit(next .. " = \"frk_adt.product_snoc\"(" .. acc .. ", " .. v .. ") : (" .. acc_ty .. ", " .. st .. ") -> !frk_adt.product<[" .. join_parts(parts) .. "]>")
    acc = next
    acc_ty = "!frk_adt.product<[" .. join_parts(parts) .. "]>"
  end
  return { value = acc, ty = acc_ty }
end

function emit_product_single(value, storage)
  local e = fresh()
  emit(e .. " = \"frk_adt.product_new\"() : () -> !frk_adt.product<[]>")
  local p = fresh()
  emit(p .. " = \"frk_adt.product_snoc\"(" .. e .. ", " .. value .. ") : (!frk_adt.product<[]>, " .. storage .. ") -> !frk_adt.product<[" .. storage .. "]>")
  return { value = p, ty = "!frk_adt.product<[" .. storage .. "]>" }
end

function wrap_store(value, ty, line, col)
  if not is_named(ty) then
    return value, storage_ir(ty, line, col)
  end
  local d = find_type(ty.named, line, col)
  local vty = value_ir(ty, line, col)
  local box = value
  local ref_operand_ty = vty
  if d.kind == "union" then
    -- a union value is the raw sum; box it before erasing
    box = fresh()
    emit(box .. " = \"frk_mem.box_new\"(" .. value .. ") : (" .. vty .. ") -> !frk_mem.box<" .. vty .. ">")
    ref_operand_ty = "!frk_mem.box<" .. vty .. ">"
  end
  local r = fresh()
  emit(r .. " = \"frk_mem.rec_ref\"(" .. box .. ") : (" .. ref_operand_ty .. ") -> !frk_mem.recref")
  return r, "!frk_mem.recref"
end

function unwrap_load(stored, ty, line, col)
  if not is_named(ty) then
    return stored
  end
  local d = find_type(ty.named, line, col)
  local vty = value_ir(ty, line, col)
  local cast_ty = vty
  if d.kind == "union" then
    cast_ty = "!frk_mem.box<" .. vty .. ">"
  end
  local box = fresh()
  emit(box .. " = \"frk_mem.rec_cast\"(" .. stored .. ") : (!frk_mem.recref) -> " .. cast_ty)
  if d.kind == "union" then
    local s = fresh()
    emit(s .. " = \"frk_mem.box_get\"(" .. box .. ") : (" .. cast_ty .. ") -> " .. vty)
    return s
  end
  return box
end

-- Reads a binding to its VALUE ssa + logical type.
function read_binding(b, line, col)
  if b.kind == "ssa" then
    return b.id, b.ty
  end
  local v = fresh()
  emit(v .. " = \"frk_mem.box_get\"(" .. b.id .. ") : (" .. box_ir(b.ty, line, col) .. ") -> " .. value_ir(b.ty, line, col))
  return v, b.ty
end

function find_field(decl, name, line, col)
  for i = 1, #decl.fields do
    if decl.fields[i].name == name then
      return i
    end
  end
  fail(line, col, "unknown field `" .. name .. "` in type `" .. decl.name .. "`")
end

function find_variant(decl, name, line, col)
  for i = 1, #decl.variants do
    if decl.variants[i].name == name then
      return i
    end
  end
  fail(line, col, "unknown variant `" .. name .. "` in type `" .. decl.name .. "`")
end

function emit_equality(a, b, ty, line, col)
  if ty == "i64" then
    local r = fresh()
    emit(r .. " = arith.cmpi eq, " .. a .. ", " .. b .. " : i64")
    return r
  end
  if ty == "bool" then
    local r = fresh()
    emit(r .. " = arith.cmpi eq, " .. a .. ", " .. b .. " : i1")
    return r
  end
  if ty == "Str" then
    local r = fresh()
    emit(r .. " = \"frk_bstr.eq\"(" .. a .. ", " .. b .. ") : (!frk_bstr.str, !frk_bstr.str) -> i1")
    return r
  end
  fail(line, col, "seed v0 compares only i64/bool/Str in patterns")
end

local function emit_make(node)
  local m = node.make
  local line = node.line
  local col = node.col
  local d = find_type(m.ty, line, col)
  if d.kind ~= "struct" then
    fail(line, col, "`" .. m.ty .. "` is a union; construct variants as `" .. m.ty .. ".Variant`")
  end
  local provided = {}
  for i = 1, #m.fields do
    local f = m.fields[i]
    if provided[f.name] ~= nil then
      fail(f.line, f.col, "duplicate field `" .. f.name .. "` in %" .. m.ty .. "{...}")
    end
    provided[f.name] = f
    find_field(d, f.name, f.line, f.col)
  end
  local base_value = nil
  if m.base ~= nil then
    local bv, bty = emit_expr(m.base), nil
    base_value = bv
    bty = E.vtypes[bv]
    if not is_named(bty) or bty.named ~= m.ty then
      fail(line, col, "the update base must be a `" .. m.ty .. "` value (get `" .. tostring(is_named(bty) and bty.named or bty) .. "`)")
    end
  end
  local prod = emit_product(d.fields, function(field, index)
    local value = nil
    if provided[field.name] ~= nil then
      value = emit_expr(provided[field.name].expr)
    elseif base_value ~= nil then
      local stored = fresh()
      emit(stored .. " = \"frk_mem.field_get\"(" .. base_value .. ") {field = " .. tostring(index - 1) .. " : i64} : (" .. value_ir({ named = m.ty }, line, col) .. ") -> " .. storage_ir(field.ty, field.line, field.col))
      value = unwrap_load(stored, field.ty, field.line, field.col)
    elseif field.default ~= nil then
      value = emit_expr(field.default)
    else
      fail(line, col, "missing required field `" .. field.name .. "` in %" .. m.ty .. "{...} (no default declared)")
    end
    local stored, sty = wrap_store(value, field.ty, field.line, field.col)
    return stored, sty
  end)
  local box = fresh()
  emit(box .. " = \"frk_mem.box_new\"(" .. prod.value .. ") : (" .. prod.ty .. ") -> " .. value_ir({ named = m.ty }, line, col))
  E.vtypes[box] = { named = m.ty }
  return box
end

function pattern_is_catch_all(sub)
  if sub.pat == "wild" or sub.pat == "bind" then
    return true
  end
  if sub.pat == "rec" then
    for i = 1, #sub.fields do
      if not pattern_is_catch_all(sub.fields[i].sub) then
        return false
      end
    end
    return true
  end
  return false
end

-- Pattern conditions: returns an i1 ssa (or nil) AND collects bindings
-- (declared later, only in the matching arm's body scope).
local function pattern_conditions(sub, value, lty, out_bindings)
  if sub.pat == "wild" then
    return nil
  end
  if sub.pat == "bind" then
    out_bindings[#out_bindings + 1] = { name = sub.name, ty = lty, id = value, line = sub.line, col = sub.col }
    return nil
  end
  if sub.pat == "pin" then
    local b = lookup(sub.name)
    if b == nil then
      fail(sub.line, sub.col, "unknown pinned name `" .. sub.name .. "`")
    end
    local other = read_binding(b, sub.line, sub.col)
    return emit_equality(value, other, lty, sub.line, sub.col)
  end
  if sub.pat == "lit" then
    local other = nil
    if sub.litty == "i64" then
      if lty ~= "i64" then
        fail(sub.line, sub.col, "integer patterns match only i64 subjects")
      end
      other = const_i64(sub.text)
    elseif sub.litty == "bool" then
      if lty ~= "bool" then
        fail(sub.line, sub.col, "boolean patterns match only bool subjects")
      end
      other = fresh()
      emit(other .. " = arith.constant " .. sub.text)
    else
      if lty ~= "Str" then
        fail(sub.line, sub.col, "string patterns match only Str subjects")
      end
      other = fresh()
      emit(other .. " = \"frk_bstr.lit\"() {text = \"" .. esc(sub.text) .. "\"} : () -> !frk_bstr.str")
    end
    return emit_equality(value, other, lty, sub.line, sub.col)
  end
  if sub.pat == "rec" then
    if not is_named(lty) or lty.named ~= sub.ty then
      fail(sub.line, sub.col, "pattern `%" .. sub.ty .. "{...}` does not match the subject type")
    end
    local d = find_type(sub.ty, sub.line, sub.col)
    if d.kind ~= "struct" then
      fail(sub.line, sub.col, "`%" .. sub.ty .. "{...}` is a struct pattern")
    end
    local cond = nil
    for i = 1, #sub.fields do
      local fp = sub.fields[i]
      local idx = find_field(d, fp.name, fp.line, fp.col)
      local fld = d.fields[idx]
      local stored = fresh()
      emit(stored .. " = \"frk_mem.field_get\"(" .. value .. ") {field = " .. tostring(idx - 1) .. " : i64} : (" .. value_ir({ named = d.name }, fp.line, fp.col) .. ") -> " .. storage_ir(fld.ty, fld.line, fld.col))
      local fv = unwrap_load(stored, fld.ty, fld.line, fld.col)
      local c2 = pattern_conditions(fp.sub, fv, fld.ty, out_bindings)
      if c2 ~= nil then
        if cond == nil then
          cond = c2
        else
          local both = fresh()
          emit(both .. " = arith.andi " .. cond .. ", " .. c2 .. " : i1")
          cond = both
        end
      end
    end
    return cond
  end
  if sub.pat == "variant" then
    if not is_named(lty) then
      fail(sub.line, sub.col, "variant patterns need a union subject")
    end
    local d = find_type(lty.named, sub.line, sub.col)
    if d.kind ~= "union" or d.name ~= sub.ty then
      fail(sub.line, sub.col, "pattern `" .. sub.ty .. "." .. sub.variant .. "` does not match the subject type")
    end
    local idx = find_variant(d, sub.variant, sub.line, sub.col)
    local tag = fresh()
    emit(tag .. " = \"frk_adt.tag_of\"(" .. value .. ") : (" .. value_ir(lty, sub.line, sub.col) .. ") -> i64")
    local eq = fresh()
    emit(eq .. " = arith.cmpi eq, " .. tag .. ", " .. const_i64(tostring(idx - 1)) .. " : i64")
    return eq
  end
  fail(sub.line, sub.col, "internal: unhandled pattern")
end

local function collect_bindings(bindings)
  for i = 1, #bindings do
    declare(bindings[i].name, { kind = "ssa", ty = bindings[i].ty, id = bindings[i].id }, bindings[i].line, bindings[i].col)
    E.vtypes[bindings[i].id] = bindings[i].ty
  end
end

local function emit_match(node)
  local m = node.match
  local line = node.line
  local col = node.col
  local sref = m.scrut
  if sref.ref == nil or sref.ref.mod ~= nil then
    fail(line, col, "a match scrutinee must be a named value (seed v0)")
  end
  local b = lookup(sref.ref.name)
  if b == nil then
    fail(line, col, "unknown name `" .. sref.ref.name .. "`")
  end
  local value, sty = read_binding(b, line, col)
  local join_label = fresh_label()
  local join_arg = "%j" .. tostring(E.label)
  local join_ty = nil

  -- Emits one arm into the CURRENT block. With conditions: a
  -- cond_br splits body vs continuation (the caller proceeds in the
  -- created next block). Without: the body runs IN PLACE — the arm
  -- was already selected by a test, or it is the fall arm.
  local function emit_arm(conditions, bindings, body, pat)
    local function run_body()
      scope_push()
      collect_bindings(bindings)
      local v = emit_expr(body)
      local vty = v ~= nil and E.vtypes[v] or "unit"
      if not E.terminated then
        if vty == "unit" then
          terminate("cf.br ^" .. join_label)
        else
          if join_ty == nil then
            join_ty = vty
          elseif join_ty ~= vty then
            fail(pat.line, pat.col, "match arms produce different types (`" .. tostring(join_ty) .. "` vs `" .. tostring(vty) .. "`)")
          end
          terminate("cf.br ^" .. join_label .. "(" .. v .. " : " .. value_ir(vty, pat.line, pat.col) .. ")")
        end
      end
      scope_pop()
    end
    if conditions ~= nil then
      local body_label = fresh_label()
      local next_label = fresh_label()
      emit("cf.cond_br " .. conditions .. ", ^" .. body_label .. ", ^" .. next_label)
      new_block(body_label)
      run_body()
      new_block(next_label)
      return "split"
    end
    run_body()
    return "inplace"
  end

  -- classify arms per scrutinee kind
  if is_named(sty) then
    local d = find_type(sty.named, line, col)
    if d.kind == "union" then
      local variant_arms = {}
      local default_arm = nil
      local covered = {}
      for i = 1, #m.arms do
        local a = m.arms[i]
        if a.pat.pat == "variant" then
          if a.pat.ty ~= d.name then
            fail(a.pat.line, a.pat.col, "pattern `" .. a.pat.ty .. "` does not match scrutinee type `" .. d.name .. "`")
          end
          local idx = find_variant(d, a.pat.variant, a.pat.line, a.pat.col)
          if covered[idx] then
            fail(a.pat.line, a.pat.col, "redundant arm: `" .. a.pat.variant .. "` is already covered")
          end
          covered[idx] = true
          variant_arms[#variant_arms + 1] = { idx = idx, arm = a }
        elseif a.pat.pat == "wild" or a.pat.pat == "bind" then
          if default_arm ~= nil then
            fail(a.pat.line, a.pat.col, "unreachable arm: a previous arm already covers everything")
          end
          default_arm = a
        else
          fail(a.pat.line, a.pat.col, "in a union match, arms are variant patterns or `_`/bindings (seed v0)")
        end
      end
      local missing = {}
      for i = 1, #d.variants do
        if not covered[i] then
          missing[#missing + 1] = d.name .. "." .. d.variants[i].name
        end
      end
      if default_arm == nil and #missing > 0 then
        fail(line, col, "match does not cover " .. join_parts(missing))
      end
      local chain = variant_arms
      local fall = default_arm
      if fall == nil then
        fall = variant_arms[#variant_arms].arm
        -- The final arm double-serves as the fallthrough: its payload
        -- sub-pattern must not be able to fail (a following arm would
        -- be unreachable otherwise).
        local sp = fall.pat.sub
        if sp ~= nil and (sp.pat == "lit" or sp.pat == "pin" or sp.pat == "rec" or sp.pat == "variant") then
          fail(sp.line, sp.col, "this payload pattern can fail; add a following `_` arm (seed v0)")
        end
        chain = {}
        for i = 1, #variant_arms - 1 do
          chain[i] = variant_arms[i]
        end
      end
      for i = 1, #chain do
        local a = chain[i].arm
        local idx = chain[i].idx
        local arm_label = fresh_label()
        local next_label = fresh_label()
        local tag = fresh()
        emit(tag .. " = \"frk_adt.tag_of\"(" .. value .. ") : (" .. value_ir(sty, line, col) .. ") -> i64")
        local eq = fresh()
        emit(eq .. " = arith.cmpi eq, " .. tag .. ", " .. const_i64(tostring(idx - 1)) .. " : i64")
        emit("cf.cond_br " .. eq .. ", ^" .. arm_label .. ", ^" .. next_label)
        new_block(arm_label)
        local conditions = nil
        local bindings = {}
        if a.pat.sub ~= nil then
          local vty = d.variants[idx].ty
          if vty == nil then
            fail(a.pat.line, a.pat.col, "variant `" .. a.pat.variant .. "` has no payload")
          end
          local stored = fresh()
          emit(stored .. " = \"frk_adt.extract\"(" .. value .. ") {variant = " .. tostring(idx - 1) .. " : i64, field = 0 : i64} : (" .. value_ir(sty, line, col) .. ") -> " .. storage_ir(vty, a.pat.line, a.pat.col))
          local pv = unwrap_load(stored, vty, a.pat.line, a.pat.col)
          conditions = pattern_conditions(a.pat.sub, pv, vty, bindings)
        end
        local step = emit_arm(conditions, bindings, a.body, a.pat)
        if step == "inplace" then
          new_block(next_label)
        end
      end
      -- The fall arm emits in the current block; its sub-pattern
      -- cannot fail (checked above).
      local a = fall
      local conditions = nil
      local bindings = {}
      if a.pat.pat == "variant" then
        local idx = find_variant(d, a.pat.variant, a.pat.line, a.pat.col)
        local vty = d.variants[idx].ty
        if a.pat.sub ~= nil then
          if vty == nil then
            fail(a.pat.line, a.pat.col, "variant `" .. a.pat.variant .. "` has no payload")
          end
          local stored = fresh()
          emit(stored .. " = \"frk_adt.extract\"(" .. value .. ") {variant = " .. tostring(idx - 1) .. " : i64, field = 0 : i64} : (" .. value_ir(sty, line, col) .. ") -> " .. storage_ir(vty, a.pat.line, a.pat.col))
          local pv = unwrap_load(stored, vty, a.pat.line, a.pat.col)
          conditions = pattern_conditions(a.pat.sub, pv, vty, bindings)
          if conditions ~= nil then
            fail(a.pat.line, a.pat.col, "internal: the fall arm's conditions can fail")
          end
        end
      elseif a.pat.pat == "bind" then
        bindings[#bindings + 1] = { name = a.pat.name, ty = sty, id = value, line = a.pat.line, col = a.pat.col }
      end
      local _ = emit_arm(conditions, bindings, a.body, a.pat)
      new_block(join_label)
      if join_ty ~= nil then
        E.blocks[#E.blocks].label = join_label .. "(" .. join_arg .. ": " .. value_ir(join_ty, line, col) .. ")"
        E.vtypes[join_arg] = join_ty
        return join_arg
      end
      return nil
    end
    -- struct scrutinee: record patterns, pins, literals; a catch-all
    -- (wildcard, binding, or an all-binding record pattern) is
    -- required, and it must be the final arm.
    local cond_arms = m.arms
    local d2 = d
    local default_seen = nil
    for i = 1, #cond_arms do
      local a = cond_arms[i]
      if default_seen ~= nil then
        fail(a.pat.line, a.pat.col, "unreachable arm: a previous arm already covers everything")
      end
      if pattern_is_catch_all(a.pat) then
        default_seen = i
      end
    end
    if default_seen == nil then
      fail(line, col, "a struct match needs a final catch-all arm (`_`, a binding, or an all-binding record pattern) (seed v0)")
    end
    for i = 1, #cond_arms do
      local a = cond_arms[i]
      local bindings = {}
      if i == default_seen then
        local conditions = pattern_conditions(a.pat, value, sty, bindings)
        if conditions ~= nil then
          fail(a.pat.line, a.pat.col, "internal: a catch-all arm carries conditions")
        end
        local _ = emit_arm(nil, bindings, a.body, a.pat)
      else
        local conditions = pattern_conditions(a.pat, value, sty, bindings)
        if conditions == nil then
          fail(a.pat.line, a.pat.col, "this pattern matches unconditionally; use `_` as the catch-all arm (seed v0)")
        end
        local _ = emit_arm(conditions, bindings, a.body, a.pat)
      end
    end
    new_block(join_label)
    if join_ty ~= nil then
      E.blocks[#E.blocks].label = join_label .. "(" .. join_arg .. ": " .. value_ir(join_ty, line, col) .. ")"
      E.vtypes[join_arg] = join_ty
      return join_arg
    end
    return nil
  end
  -- builtin scrutinee: literal/pin arms; a catch-all is required
  local default_seen = nil
  for i = 1, #m.arms do
    local a = m.arms[i]
    if a.pat.pat == "wild" or a.pat.pat == "bind" then
      if default_seen ~= nil then
        fail(a.pat.line, a.pat.col, "unreachable arm: a previous arm already covers everything")
      end
      default_seen = i
    end
  end
  if default_seen == nil then
    fail(line, col, "a builtin match needs a final `_` or binding arm (seed v0)")
  end
  for i = 1, #m.arms do
    local a = m.arms[i]
    local bindings = {}
    if i == default_seen then
      if a.pat.pat == "bind" then
        bindings[#bindings + 1] = { name = a.pat.name, ty = sty, id = value, line = a.pat.line, col = a.pat.col }
      end
      local _ = emit_arm(nil, bindings, a.body, a.pat)
    else
      local conditions = pattern_conditions(a.pat, value, sty, bindings)
      if conditions == nil then
        fail(a.pat.line, a.pat.col, "this pattern matches unconditionally; use `_` as the catch-all arm (seed v0)")
      end
      local _ = emit_arm(conditions, bindings, a.body, a.pat)
    end
  end
  new_block(join_label)
  if join_ty ~= nil then
    E.blocks[#E.blocks].label = join_label .. "(" .. join_arg .. ": " .. value_ir(join_ty, line, col) .. ")"
    E.vtypes[join_arg] = join_ty
    return join_arg
  end
  return nil
end

local function emit_call(node)
  local callee = node.call
  local line = node.line
  local col = node.col
  local args = node.args
  local td = callee.mod ~= nil and E.types[callee.mod] or nil
  if td ~= nil then
    -- `Type.Variant` construction (D-095): the sum value, wrapped at
    -- storage sites only.
    if td.kind ~= "union" then
      fail(line, col, "`" .. callee.mod .. "` is a struct type; construct it with `%" .. callee.mod .. "{...}`")
    end
    local idx = find_variant(td, callee.name, line, col)
    local v = td.variants[idx]
    local uty = { named = td.name }
    local uir = value_ir(uty, line, col)
    if #args == 0 then
      if v.ty ~= nil then
        fail(line, col, "variant `" .. callee.name .. "` carries a payload (one argument)")
      end
      local e = fresh()
      emit(e .. " = \"frk_adt.product_new\"() : () -> !frk_adt.product<[]>")
      local s = fresh()
      emit(s .. " = \"frk_adt.make_sum\"(" .. e .. ") {variant = " .. tostring(idx - 1) .. " : i64} : (!frk_adt.product<[]>) -> " .. uir)
      E.vtypes[s] = uty
      return s, uty
    end
    if #args ~= 1 then
      fail(line, col, "a variant payload is ONE value (seed v0)")
    end
    if v.ty == nil then
      fail(line, col, "variant `" .. callee.name .. "` has no payload")
    end
    local payload = emit_expr(args[1])
    local stored, sty = wrap_store(payload, v.ty, line, col)
    local prod = emit_product_single(stored, sty)
    local s = fresh()
    emit(s .. " = \"frk_adt.make_sum\"(" .. prod.value .. ") {variant = " .. tostring(idx - 1) .. " : i64} : (" .. prod.ty .. ") -> " .. uir)
    E.vtypes[s] = uty
    return s, uty
  end
  if callee.mod == "io" and callee.name == "print" then
    if #args ~= 1 then
      fail(line, col, "io.print takes exactly one Str (seed v0)")
    end
    local a = emit_expr(args[1])
    emit("func.call @frk_rt_calque_print(" .. a .. ") : (!frk_bstr.str) -> ()")
    return nil
  end
  -- The compiler's input channel (BOOTSTRAP 2.1): the driver-injected
  -- __calque_input_count()/__calque_input(i). Definitions live in the
  -- injection component; a call without it fails verification (loud).
  if callee.mod == nil and callee.name == "__calque_input_count" then
    if #args ~= 0 then
      fail(line, col, "__calque_input_count takes no arguments")
    end
    local res = fresh()
    emit(res .. " = func.call @__calque_input_count() : () -> i64")
    return res, "i64"
  end
  if callee.mod == nil and callee.name == "__calque_input" then
    if #args ~= 1 then
      fail(line, col, "__calque_input takes exactly one i64")
    end
    local a = emit_expr(args[1])
    local res = fresh()
    emit(res .. " = func.call @__calque_input(" .. a .. ") : (i64) -> !frk_bstr.str")
    return res, "Str"
  end
  if callee.mod == "vector" then
    -- Vector(T) surface (F34/D-091): monomorphic intrinsics. `new`
    -- spells the slot; push/get/set/len resolve it from the receiver's
    -- static type (b0 spells types; nothing is inferred).
    local vname = callee.name
    if vname == "new" then
      fail(line, col, "vector.new() takes its instantiation from the declaration's annotation (seed v0): write `const v: Vector(i64) = vector.new();`")
    end
    if #args < 1 then
      fail(line, col, "vector." .. vname .. " needs the vector as its first argument")
    end
    local vid = emit_expr(args[1])
    local vty = E.vtypes[vid]
    if not is_vec(vty) then
      fail(line, col, "vector." .. vname .. " expects a Vector receiver")
    end
    local slot, prefix = vec_slot(vty.vec, line, col)
    if vname == "len" then
      if #args ~= 1 then
        fail(line, col, "vector.len takes exactly one argument")
      end
      local res = fresh()
      emit(res .. " = func.call @" .. prefix .. "_len(" .. vid .. ") : (" .. vec_box_ir(vty.vec, line, col) .. ") -> i64")
      E.vtypes[res] = "i64"
      return res, "i64"
    end
    if vname == "push" or vname == "set" then
      local want = (vname == "push") and 2 or 3
      if #args ~= want then
        fail(line, col, "vector." .. vname .. " takes " .. tostring(want) .. " arguments (seed v0)")
      end
      local index_id = nil
      if vname == "set" then
        local iid = emit_expr(args[2])
        if E.vtypes[iid] ~= "i64" then
          fail(line, col, "vector.set index must be i64")
        end
        index_id = iid
      end
      local xid = emit_expr(args[want])
      local xty = E.vtypes[xid]
      if not same_type(xty, vty.vec) then
        fail(line, col, "vector." .. vname .. " value type does not match the Vector slot")
      end
      local stored = xid
      if is_named(vty.vec) then
        stored, _ = wrap_store(xid, xty, line, col)
      end
      local call_args = vid
      local call_types = vec_box_ir(vty.vec, line, col)
      if vname == "set" then
        call_args = call_args .. ", " .. index_id
        call_types = call_types .. ", i64"
      end
      call_args = call_args .. ", " .. stored
      call_types = call_types .. ", " .. slot
      emit("func.call @" .. prefix .. "_" .. vname .. "(" .. call_args .. ") : (" .. call_types .. ") -> ()")
      return nil
    end
    if vname == "get" then
      if #args ~= 2 then
        fail(line, col, "vector.get takes exactly two arguments (seed v0)")
      end
      local iid = emit_expr(args[2])
      if E.vtypes[iid] ~= "i64" then
        fail(line, col, "vector.get index must be i64")
      end
      local res = fresh()
      emit(res .. " = func.call @" .. prefix .. "_get(" .. vid .. ", " .. iid .. ") : (" .. vec_box_ir(vty.vec, line, col) .. ", i64) -> " .. slot)
      local value = res
      if is_named(vty.vec) then
        value = unwrap_load(res, vty.vec, line, col)
      end
      E.vtypes[value] = vty.vec
      return value, vty.vec
    end
    fail(line, col, "unknown vector operation `" .. vname .. "` (seed v0: new/push/get/set/len)")
  end
  if callee.mod == "str" then
    local member = callee.name
    if member == "len" then
      if #args ~= 1 then fail(line, col, "str.len takes exactly one argument") end
      local a = emit_expr(args[1])
      local res = fresh()
      emit(res .. " = \"frk_bstr.len\"(" .. a .. ") : (!frk_bstr.str) -> i64")
      return res, "i64"
    end
    if member == "concat" then
      if #args ~= 2 then fail(line, col, "str.concat takes exactly two arguments") end
      local a = emit_expr(args[1])
      local b = emit_expr(args[2])
      local res = fresh()
      emit(res .. " = \"frk_bstr.concat\"(" .. a .. ", " .. b .. ") : (!frk_bstr.str, !frk_bstr.str) -> !frk_bstr.str")
      return res, "Str"
    end
    if member == "sub" then
      if #args ~= 3 then fail(line, col, "str.sub takes exactly three arguments") end
      local s = emit_expr(args[1])
      local start = emit_expr(args[2])
      local len = emit_expr(args[3])
      local zero = const_i64("0")
      local e1 = fresh()
      local e2 = fresh()
      local endv = fresh()
      local e3 = fresh()
      local bad1 = fresh()
      local bad = fresh()
      local slen = fresh()
      emit(slen .. " = \"frk_bstr.len\"(" .. s .. ") : (!frk_bstr.str) -> i64")
      emit(e1 .. " = arith.cmpi slt, " .. start .. ", " .. zero .. " : i64")
      emit(e2 .. " = arith.cmpi slt, " .. len .. ", " .. zero .. " : i64")
      emit(endv .. " = arith.addi " .. start .. ", " .. len .. " : i64")
      emit(e3 .. " = arith.cmpi sgt, " .. endv .. ", " .. slen .. " : i64")
      emit(bad1 .. " = arith.ori " .. e1 .. ", " .. e2 .. " : i1")
      emit(bad .. " = arith.ori " .. bad1 .. ", " .. e3 .. " : i1")
      guard(bad, loc_msg(line, col, "CQ0006", "str.sub out of bounds"), line, col)
      local one = const_i64("1")
      local from = fresh()
      local to = fresh()
      emit(from .. " = arith.addi " .. start .. ", " .. one .. " : i64")
      emit(to .. " = arith.addi " .. start .. ", " .. len .. " : i64")
      local res = fresh()
      emit(res .. " = \"frk_bstr.sub\"(" .. s .. ", " .. from .. ", " .. to .. ") : (!frk_bstr.str, i64, i64) -> !frk_bstr.str")
      return res, "Str"
    end
    -- Other `str.*` members resolve against namespaced declarations
    -- (D-094): `str.fromI64` -> @str__fromI64. Fall through.
  end
  -- unit function call (unqualified or namespaced)
  local sym = nil
  local fn = nil
  if callee.mod ~= nil then
    sym = callee.mod .. "__" .. callee.name
    fn = E.fns_by_sym[sym]
  else
    fn = E.fns[callee.name]
    sym = fn and fn.sym or nil
  end
  if fn == nil then
    fail(line, col, "unknown function `" .. (callee.mod and (callee.mod .. ".") or "") .. callee.name .. "`")
  end
  if #args ~= #fn.params then
    fail(line, col, "`" .. sym .. "` expects " .. tostring(#fn.params) .. " argument(s), got " .. tostring(#args))
  end
  local parts = "%env"
  local types = "!frk_closure.envref"
  for i = 1, #args do
    local v = emit_expr(args[i])
    parts = parts .. ", " .. v
    types = types .. ", " .. value_ir(fn.params[i].ty, line, col)
  end
  if fn.ret == "unit" then
    emit("func.call @" .. sym .. "(" .. parts .. ") : (" .. types .. ") -> ()")
    return nil
  end
  local res = fresh()
  emit(res .. " = func.call @" .. sym .. "(" .. parts .. ") : (" .. types .. ") -> " .. value_ir(fn.ret, line, col))
  return res, fn.ret
end

local function emit_block_expr(node)
  scope_push()
  local value = nil
  for i = 1, #node.stmts do
    if E.terminated then
      fail(1, 1, "internal: statement after terminator")
    end
    E.emit_stmt(node.stmts[i])
  end
  if not E.terminated then
    if node.tail ~= nil then
      value = emit_expr(node.tail)
    end
  end
  scope_pop()
  return value
end

local function emit_if_expr(node)
  local line = node.line
  local col = node.col
  if node.els == nil then
    fail(line, col, "an `if` in expression position requires `else` (SPEC 5)")
  end
  local cond = emit_expr(node.cond)
  local then_label = fresh_label()
  local else_label = fresh_label()
  local join_label = fresh_label()
  emit("cf.cond_br " .. cond .. ", ^" .. then_label .. ", ^" .. else_label)
  new_block(then_label)
  local tv = emit_block_expr(node.yes)
  local then_end = E.cur
  local then_term = E.terminated
  new_block(else_label)
  local ev = nil
  if node.els ~= nil then
    ev = emit_block_expr(node.els)
  end
  local else_end = E.cur
  local else_term = E.terminated
  -- determine the result type from whichever arm produced a value
  local ty = nil
  if tv ~= nil then ty = E.vtypes[tv] end
  if ty == nil and ev ~= nil then ty = E.vtypes[ev] end
  if (tv ~= nil or ev ~= nil) and ty == nil then
    fail(line, col, "internal: an if arm produced an untyped value")
  end
  if tv ~= nil and ev ~= nil and E.vtypes[tv] ~= E.vtypes[ev] then
    fail(line, col, "if arm types differ (`" .. tostring(E.vtypes[tv]) .. "` vs `" .. tostring(E.vtypes[ev]) .. "`)")
  end
  new_block(join_label)
  local join_arg = nil
  if ty ~= nil then
    join_arg = "%j" .. tostring(E.label)
    -- patch the join header once the arg is known
    E.blocks[#E.blocks].label = join_label .. "(" .. join_arg .. ": " .. value_ir(ty, line, col) .. ")"
  end
  E.cur = then_end
  E.terminated = then_term
  if not E.terminated and tv ~= nil then
    terminate("cf.br ^" .. join_label .. "(" .. tv .. " : " .. value_ir(ty, line, col) .. ")")
  elseif not E.terminated then
    terminate("cf.br ^" .. join_label)
  end
  E.cur = else_end
  E.terminated = else_term
  if not E.terminated and ev ~= nil then
    terminate("cf.br ^" .. join_label .. "(" .. ev .. " : " .. value_ir(ty, line, col) .. ")")
  elseif not E.terminated then
    terminate("cf.br ^" .. join_label)
  end
  E.cur = #E.blocks
  E.terminated = false
  if join_arg ~= nil then
    E.vtypes[join_arg] = ty
  end
  return join_arg
end

emit_expr = function(node)
  local line = node.line or 1
  local col = node.col or 1
  if node.lit == "int" then
    local v = fresh()
    emit(v .. " = arith.constant " .. node.text .. " : i64")
    E.vtypes[v] = "i64"
    return v
  end
  if node.lit == "bool" then
    local v = fresh()
    emit(v .. " = arith.constant " .. node.text)
    E.vtypes[v] = "bool"
    return v
  end
  if node.lit == "str" then
    local v = fresh()
    emit(v .. " = \"frk_bstr.lit\"() {text = \"" .. esc(node.text) .. "\"} : () -> !frk_bstr.str")
    E.vtypes[v] = "Str"
    return v
  end
  if node.ref ~= nil then
    if node.ref.mod ~= nil then
      local td = E.types[node.ref.mod]
      if td ~= nil then
        -- unit-variant value: `Token.Plus` (D-095)
        if td.kind ~= "union" then
          fail(line, col, "`" .. node.ref.mod .. "` is a struct type; construct it with `%" .. node.ref.mod .. "{...}`")
        end
        local idx = find_variant(td, node.ref.name, line, col)
        if td.variants[idx].ty ~= nil then
          fail(line, col, "variant `" .. node.ref.name .. "` carries a payload (call its constructor)")
        end
        local uty = { named = td.name }
        local e = fresh()
        emit(e .. " = \"frk_adt.product_new\"() : () -> !frk_adt.product<[]>")
        local s = fresh()
        emit(s .. " = \"frk_adt.make_sum\"(" .. e .. ") {variant = " .. tostring(idx - 1) .. " : i64} : (!frk_adt.product<[]>) -> " .. value_ir(uty, line, col))
        E.vtypes[s] = uty
        return s
      end
      local owner = lookup(node.ref.mod)
      if owner == nil then
        if node.ref.mod == "str" or node.ref.mod == "io" or node.ref.mod == "vector" then
          fail(line, col, "module member `" .. node.ref.mod .. "." .. node.ref.name .. "` used as a value (seed v0)")
        end
        fail(line, col, "unknown name `" .. node.ref.mod .. "`")
      end
      if not is_named(owner.ty) then
        fail(line, col, "`" .. node.ref.mod .. "` has no fields")
      end
      local d = find_type(owner.ty.named, line, col)
      if d.kind ~= "struct" then
        fail(line, col, "`" .. node.ref.mod .. "` is a union; use `match`")
      end
      local value = read_binding(owner, line, col)
      local idx = find_field(d, node.ref.name, line, col)
      local fld = d.fields[idx]
      local stored = fresh()
      emit(stored .. " = \"frk_mem.field_get\"(" .. value .. ") {field = " .. tostring(idx - 1) .. " : i64} : (" .. value_ir(owner.ty, line, col) .. ") -> " .. storage_ir(fld.ty, fld.line, fld.col))
      local out = unwrap_load(stored, fld.ty, fld.line, fld.col)
      E.vtypes[out] = fld.ty
      return out
    end
    local b = lookup(node.ref.name)
    if b == nil then
      fail(line, col, "unknown name `" .. node.ref.name .. "`")
    end
    if b.kind == "ssa" then
      E.vtypes[b.id] = b.ty
      return b.id
    end
    local v = fresh()
    emit(v .. " = \"frk_mem.box_get\"(" .. b.id .. ") : (" .. b.box .. ") -> " .. value_ir(b.ty, line, col))
    E.vtypes[v] = b.ty
    return v
  end
  if node.call ~= nil then
    local v, ty = emit_call(node)
    if v ~= nil then
      E.vtypes[v] = ty
    end
    return v
  end
  if node.block ~= nil then
    return emit_block_expr(node.block)
  end
  if node.iff ~= nil then
    return emit_if_expr(node.iff)
  end
  if node.make ~= nil then
    return emit_make(node)
  end
  if node.match ~= nil then
    return emit_match(node)
  end
  if node.bin ~= nil then
    local op = node.bin
    local a = emit_expr(node.l)
    local b = emit_expr(node.r)
    if op == "==" or op == "!=" or op == "<" or op == "<=" or op == ">" or op == ">=" then
      local v = emit_cmp(op, a, b)
      E.vtypes[v] = "bool"
      return v
    end
    if op == "*" then
      fail(node.line, node.col, "`*` is not in seed v0 (mul-overflow slice pending)")
    end
    local v = emit_checked(op, a, b, node.line, node.col)
    E.vtypes[v] = "i64"
    return v
  end
  fail(line, col, "internal: unhandled expression node")
end

-- ------------------------------- statements

local function emit_stmt(node)
  if node.const ~= nil then
    local c = node.const
    local v = nil
    if is_vec_new_call(c.expr) then
      if not is_vec(c.ty) then
        fail(c.line, c.col, "vector.new() needs an annotated Vector binding (seed v0)")
      end
      v = emit_vec_new(c.ty.vec, c.line, c.col)
    else
      v = emit_expr(c.expr)
    end
    if v == nil then
      fail(c.line, c.col, "const initializer must be a value")
    end
    local ty = c.ty
    if ty == nil then
      ty = E.vtypes[v]
      if ty == nil then
        fail(c.line, c.col, "internal: untyped const initializer")
      end
    end
    declare(c.name, { kind = "ssa", ty = ty, id = v }, c.line, c.col)
    return
  end
  if node.var ~= nil then
    local d = node.var
    local v = nil
    if is_vec_new_call(d.expr) then
      if not is_vec(d.ty) then
        fail(d.line, d.col, "vector.new() needs an annotated Vector binding (seed v0)")
      end
      v = emit_vec_new(d.ty.vec, d.line, d.col)
    else
      v = emit_expr(d.expr)
    end
    local b = fresh()
    emit(b .. " = \"frk_mem.box_new\"(" .. v .. ") : (" .. value_ir(d.ty, d.line, d.col) .. ") -> " .. box_ir(d.ty, d.line, d.col))
    declare(d.name, { kind = "box", ty = d.ty, id = b, box = box_ir(d.ty, d.line, d.col) }, d.line, d.col)
    return
  end
  if node.assign ~= nil then
    local a = node.assign
    local b = lookup(a.name)
    if b == nil then
      fail(a.line, a.col, "unknown name `" .. a.name .. "` in assignment")
    end
    if b.kind ~= "box" then
      fail(a.line, a.col, "cannot assign to immutable `" .. a.name .. "`")
    end
    local v = emit_expr(a.expr)
    emit("\"frk_mem.box_set\"(" .. b.id .. ", " .. v .. ") : (" .. b.box .. ", " .. value_ir(b.ty, a.line, a.col) .. ") -> ()")
    return
  end
  if node.ret ~= nil then
    if node.ret.expr ~= nil then
      local v = emit_expr(node.ret.expr)
      terminate("func.return " .. v .. " : " .. value_ir(E.ret, node.ret.line, node.ret.col))
    else
      terminate("func.return")
    end
    return
  end
  if node.ifstmt ~= nil then
    local iff = node.ifstmt
    local cond = emit_expr(iff.cond)
    local then_label = fresh_label()
    local cont_label = fresh_label()
    local else_label = nil
    if iff.els ~= nil then
      else_label = fresh_label()
      emit("cf.cond_br " .. cond .. ", ^" .. then_label .. ", ^" .. else_label)
    else
      emit("cf.cond_br " .. cond .. ", ^" .. then_label .. ", ^" .. cont_label)
    end
    new_block(then_label)
    scope_push()
    local then_block = iff.yes
    for i = 1, #then_block.stmts do
      if not E.terminated then E.emit_stmt(then_block.stmts[i]) end
    end
    if not E.terminated then
      if then_block.tail ~= nil then
        emit_expr(then_block.tail)
      end
      terminate("cf.br ^" .. cont_label)
    end
    scope_pop()
    if else_label ~= nil then
      new_block(else_label)
      scope_push()
      local else_block = iff.els
      for i = 1, #else_block.stmts do
        if not E.terminated then E.emit_stmt(else_block.stmts[i]) end
      end
      if not E.terminated then
        if else_block.tail ~= nil then
          emit_expr(else_block.tail)
        end
        terminate("cf.br ^" .. cont_label)
      end
      scope_pop()
    end
    new_block(cont_label)
    return
  end
  if node.loop ~= nil then
    local w = node.loop
    local head = fresh_label()
    local body = fresh_label()
    local done = fresh_label()
    terminate("cf.br ^" .. head)
    new_block(head)
    local cond = emit_expr(w.cond)
    emit("cf.cond_br " .. cond .. ", ^" .. body .. ", ^" .. done)
    new_block(body)
    scope_push()
    for i = 1, #w.body.stmts do
      if not E.terminated then E.emit_stmt(w.body.stmts[i]) end
    end
    if not E.terminated then
      if w.body.tail ~= nil then
        emit_expr(w.body.tail)
      end
      terminate("cf.br ^" .. head)
    end
    scope_pop()
    new_block(done)
    return
  end
  if node.expr ~= nil then
    emit_expr(node.expr)
    return
  end
  fail(1, 1, "internal: unhandled statement node")
end
E.emit_stmt = emit_stmt

-- ------------------------------- functions

local function emit_fn(fn)
  CURRENT_FILE = fn.file
  E.tmp = 0
  E.label = 0
  E.blocks = {}
  E.cur = 0
  E.terminated = false
  E.scopes = {}
  E.vtypes = {}
  E.fns = FNS
  E.fns_by_sym = FNS_BY_SYM
  E.ret = fn.ret
  new_block(nil)
  local params = "%env: !frk_closure.envref"
  scope_push()
  for i = 1, #fn.params do
    local p = fn.params[i]
    if p.name == "env" then
      fail(p.line, p.col, "`env` is reserved in the seed's parameter namespace")
    end
    params = params .. ", %" .. p.name .. ": " .. value_ir(p.ty, p.line, p.col)
    declare(p.name, { kind = "ssa", ty = p.ty, id = "%" .. p.name }, p.line, p.col)
  end
  local ret_ir = ""
  if fn.ret ~= "unit" then
    ret_ir = " -> " .. value_ir(fn.ret, fn.line, fn.col)
  end
  local header = "func.func @" .. fn.sym .. "(" .. params .. ")" .. ret_ir .. " {"
  -- emit body
  local body = fn.body
  -- A trailing `if ... else ...` STATEMENT in a value-returning function
  -- is the function's value (b0 parses `if` at statement position; the
  -- return type decides the emission — SPEC 5's statement/expression
  -- duality). It is emitted once, as an expression.
  local tail_if = nil
  if fn.ret ~= "unit" and body.tail == nil and #body.stmts > 0 then
    local last = body.stmts[#body.stmts]
    if last.ifstmt ~= nil and last.ifstmt.els ~= nil then
      tail_if = last.ifstmt
    end
  end
  local upto = #body.stmts
  if tail_if ~= nil then
    upto = upto - 1
  end
  for i = 1, upto do
    if not E.terminated then E.emit_stmt(body.stmts[i]) end
  end
  if not E.terminated then
    if tail_if ~= nil then
      local v = emit_if_expr(tail_if)
      terminate("func.return " .. v .. " : " .. value_ir(fn.ret, fn.line, fn.col))
    elseif body.tail ~= nil then
      local v = emit_expr(body.tail)
      terminate("func.return " .. v .. " : " .. value_ir(fn.ret, fn.line, fn.col))
    else
      terminate("func.return")
    end
  end
  scope_pop()
  -- serialize
  E.out[#E.out + 1] = header
  for i = 1, #E.blocks do
    local b = E.blocks[i]
    if b.label ~= nil then
      E.out[#E.out + 1] = "^" .. b.label .. ":"
    end
    for j = 1, #b.lines do
      E.out[#E.out + 1] = "  " .. b.lines[j]
    end
  end
  E.out[#E.out + 1] = "}"
end

local function emit_entry_wrapper(fn)
  E.out[#E.out + 1] = "func.func @main() -> i64 attributes {llvm.emit_c_interface} {"
  E.out[#E.out + 1] = "  %env = \"frk_adt.product_new\"() : () -> !frk_adt.product<[]>"
  E.out[#E.out + 1] = "  %cl = \"frk_closure.make\"(%env) {callee = @" .. fn.sym .. "} : (!frk_adt.product<[]>) -> !frk_closure.fn<[], [i64]>"
  E.out[#E.out + 1] = "  %args = \"frk_adt.product_new\"() : () -> !frk_adt.product<[]>"
  E.out[#E.out + 1] = "  %r = \"frk_closure.apply\"(%cl, %args) : (!frk_closure.fn<[], [i64]>, !frk_adt.product<[]>) -> i64"
  E.out[#E.out + 1] = "  return %r : i64"
  E.out[#E.out + 1] = "}"
end

-- ---------------------------------------------------------------- run

FNS = {}
FNS_BY_SYM = {}

function run(files)
  E.out = {}
  E.out[#E.out + 1] = "// calque-artifact v0"
  local parsed = {}
  local types = {}
  for i = 1, #files do
    local f = files[i]
    CURRENT_FILE = f.name
    local unit = parse_file(f.text, f.name)
    parsed[#parsed + 1] = { file = f, fns = unit.fns }
    for j = 1, #unit.types do
      local td = unit.types[j]
      td.file = f.name
      if f.ns ~= nil then
        fail(td.line, td.col, "type declarations cannot live inside a namespace (seed v0)")
      end
      if td.name == "str" or td.name == "io" or td.name == "vector" or td.name == "Str" then
        fail(td.line, td.col, "`" .. td.name .. "` is reserved (SPEC 2)")
      end
      if types[td.name] ~= nil then
        fail(td.line, td.col, "duplicate type `" .. td.name .. "`")
      end
      types[td.name] = td
    end
  end
  E.types = types
  -- name resolution + mangling (D-094)
  for i = 1, #parsed do
    local p = parsed[i]
    for j = 1, #p.fns do
      local fn = p.fns[j]
      fn.file = p.file.name
      if p.file.ns ~= nil then
        if fn.name == "main" then
          fail(fn.line, fn.col, "`main` cannot be declared inside a namespace")
        end
        fn.sym = p.file.ns .. "__" .. fn.name
      elseif fn.name == "main" then
        fn.sym = "__calque_main"
        fn.is_entry = true
      else
        fn.sym = fn.name
      end
      if FNS_BY_SYM[fn.sym] ~= nil then
        fail(fn.line, fn.col, "duplicate top-level name `" .. fn.sym .. "`")
      end
      if p.file.ns == nil and E.types[fn.name] ~= nil then
        fail(fn.line, fn.col, "`" .. fn.name .. "` collides with a declared type")
      end
      FNS_BY_SYM[fn.sym] = fn
      if p.file.ns == nil and fn.name ~= "main" then
        FNS[fn.name] = fn
      end
    end
  end
  -- emit every function, then the entry wrapper (D-092)
  local entry = nil
  for i = 1, #parsed do
    local p = parsed[i]
    for j = 1, #p.fns do
      local fn = p.fns[j]
      if fn.is_entry then
        entry = fn
      else
        emit_fn(fn)
      end
    end
  end
  if entry == nil then
    fail(1, 1, "the unit has no `pub fn main() i64` entry (seed v0)")
  end
  if #entry.params ~= 0 or entry.ret ~= "i64" then
    fail(entry.line, entry.col, "seed v0 requires `fn main()` with no parameters returning i64")
  end
  emit_fn(entry)
  emit_entry_wrapper(entry)
  for i = 1, #E.out do
    print(E.out[i])
  end
end
