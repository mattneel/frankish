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
-- the str/io preludes), destructuring (`const {a, b} = e;` /
-- `var {a, b} = e;`, D-107), the checked-arithmetic subset + - * / %,
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
  "comptime",
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
      local start_col = col
      local digits = ""
      while i <= n do
        local ch = string.sub(source, i, i)
        if is_digit(ch) then
          digits = digits .. ch
          i = i + 1
          col = col + 1
        elseif ch == "_" then
          -- C2 digit separator (D-115): between digits only, stripped
          -- from the value.
          local nx = string.sub(source, i + 1, i + 1)
          if #digits == 0 or not is_digit(nx) then
            fail(line, start_col, "a digit separator `_` needs digits on both sides")
          end
          i = i + 1
          col = col + 1
        else
          break
        end
      end
      push("int", digits, line, start_col)
    elseif c == "\"" then
      if string.sub(source, i, i + 2) == "\"\"\"" then
        -- multiline string (C2, D-115): verbatim between the `"""`.
        local mstart_col = col
        i = i + 3
        col = col + 3
        local macc = ""
        local mclosed = false
        while i <= n do
          if string.sub(source, i, i + 2) == "\"\"\"" then
            i = i + 3
            col = col + 3
            mclosed = true
            break
          end
          local mch = string.sub(source, i, i)
          macc = macc .. mch
          i = i + 1
          if mch == "\n" then
            line = line + 1
            col = 1
          else
            col = col + 1
          end
        end
        if not mclosed then
          fail(line, mstart_col, "unterminated multiline string literal")
        end
        push("str", macc, line, mstart_col)
      else
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
      end
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
          or one == "%" or one == "!" or one == "^" or one == "|"
          or one == "[" or one == "]" or one == "?" then
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

local function parse_type_atom()
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
  if t.k == "kw" and t.v == "fn" then
    -- `fn (T, ...) R` - a first-class function type (SPEC 6.4).
    next_tok()
    local lp = peek()
    if not (lp.k == "punct" and lp.v == "(") then
      fail(lp.line, lp.col, "fn type needs a parameter list: `fn (T, ...) R`")
    end
    next_tok()
    local ps = {}
    if not at_punct(")") then
      while true do
        local pt = parse_type()
        ps[#ps + 1] = pt
        if at_punct(",") then
          next_tok()
        else
          break
        end
      end
    end
    local rp = peek()
    if not (rp.k == "punct" and rp.v == ")") then
      fail(rp.line, rp.col, "expected `)` after the fn type's parameters")
    end
    next_tok()
    local rty = parse_type()
    return { fnp = ps, fnr = rty }
  end
  if t.k == "punct" and t.v == "(" then
    -- `(T1, T2)` - the tuple type (D-114). `()` is not a tuple (unit
    -- has its own spelling in type position: it appears as an absent
    -- return type, not as a paren type).
    next_tok()
    if at_punct(")") then
      fail(t.line, t.col, "`()` is not a tuple type (use no return annotation for unit)")
    end
    local tys = {}
    while true do
      tys[#tys + 1] = parse_type()
      if at_punct(",") then
        next_tok()
      else
        break
      end
    end
    local rp2 = peek()
    if not (rp2.k == "punct" and rp2.v == ")") then
      fail(rp2.line, rp2.col, "expected `)` after the tuple type")
    end
    next_tok()
    if #tys < 2 then
      fail(t.line, t.col, "a tuple type has two or more element types (D-114)")
    end
    return { tup = tys }
  end
  if t.k == "punct" and t.v == "[" then
    -- `[T]` - the collection type (D-108's spelling; identical to
    -- Vector(T) in v0, per D-112's constructor-scope ruling).
    next_tok()
    local inner = parse_type()
    local rb = peek()
    if not (rb.k == "punct" and rb.v == "]") then
      fail(rb.line, rb.col, "expected `]` after the collection element type")
    end
    next_tok()
    if inner ~= "i64" and inner ~= "Str" and not (type(inner) == "table" and inner.named ~= nil) then
      fail(t.line, t.col, "unsupported [T] slot (seed v0 instantiations: i64, Str, declared types)")
    end
    return vec_ty(inner)
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
    return vec_ty(inner)
  end
  if t.k == "name" then
    next_tok()
    return named_ty(t.v)
  end
  fail(t.line, t.col, "unsupported type (seed v0: i64, bool, Str, unit, declared types)")
end

function parse_type()
  -- `?T` (Option) and `E!T` (Result) shorthands (SPEC 6.5): real
  -- structural types, no per-instantiation decls needed.
  local t = peek()
  if t.k == "punct" and t.v == "?" then
    next_tok()
    return { opt = parse_type_atom() }
  end
  local lhs = parse_type_atom()
  local nxt = peek()
  if nxt.k == "punct" and nxt.v == "!" then
    next_tok()
    return { reserr = lhs, resok = parse_type_atom() }
  end
  return lhs
end

function parse_expr_ops(level)
  -- levels 1..6 (pipeline, or, and, comparisons, +-, */%), unary,
  -- postfix (SPEC 5.1's precedence table, loosest first).
  if level > 6 then
    return parse_unary()
  end
  local l = parse_expr(level + 1)
  while true do
    local t = peek()
    local op = nil
    if t.k == "punct" then
      if level == 1 and t.v == "|>" then
        op = "|>"
      elseif level == 4 and (t.v == "==" or t.v == "!=" or t.v == "<" or t.v == "<=" or t.v == ">" or t.v == ">=") then
        op = t.v
      elseif level == 5 and (t.v == "+" or t.v == "-") then
        op = t.v
      elseif level == 6 and (t.v == "*" or t.v == "/" or t.v == "%") then
        op = t.v
      end
    elseif t.k == "kw" then
      if level == 2 and t.v == "or" then
        op = "or"
      elseif level == 3 and t.v == "and" then
        op = "and"
      end
    end
    if op == nil then
      return l
    end
    next_tok()
    local r = parse_expr(level + 1)
    if op == "|>" then
      -- x |> f(a, b) = f(x, a, b); x |> f = f(x) (SPEC 6.3).
      if r.call ~= nil and r.args ~= nil then
        local args = { l }
        local i = 1
        while i <= #r.args do
          args[#args + 1] = r.args[i]
          i = i + 1
        end
        l = { call = r.call, args = args, line = t.line, col = t.col }
      elseif r.ref ~= nil then
        l = { call = r.ref, args = { l }, line = t.line, col = t.col }
      else
        fail(t.line, t.col, "a pipeline stage must be a callable path (SPEC 6.3)")
      end
    else
      l = { bin = op, l = l, r = r, line = t.line, col = t.col }
    end
    if level == 4 then
      local nxt = peek()
      if nxt.k == "punct" and (nxt.v == "==" or nxt.v == "!=" or nxt.v == "<" or nxt.v == "<=" or nxt.v == ">" or nxt.v == ">=") then
        fail(nxt.line, nxt.col, "comparisons do not chain (SPEC 5.1)")
      end
    end
  end
end

function parse_expr(level)
  -- `catch` binds at the LOWEST precedence (level 0, loosest): the
  -- replacement block runs on Err and yields the unwrapped type
  -- (SPEC 6.5's recovery form).
  local e = parse_expr_ops(level)
  if level ~= 1 then
    return e
  end
  while true do
    local t = peek()
    if not (t.k == "kw" and t.v == "catch") then
      return e
    end
    next_tok()
    local bind = nil
    local t2 = peek()
    if t2.k == "punct" and t2.v == "|" then
      next_tok()
      local nm = next_tok()
      if nm.k ~= "name" then
        fail(nm.line, nm.col, "expected a binding name in `catch |name|`")
      end
      if expect_punct("|") == false then
        fail(t2.line, t2.col, "expected `|` after the catch binding")
      end
      bind = nm.v
    end
    local blk = parse_block()
    e = { catchx = e, bind = bind, block = blk, line = t.line, col = t.col }
  end
end

function parse_unary()
  local t = peek()
  if t.k == "kw" and t.v == "try" then
    next_tok()
    return { tryex = parse_unary(), line = t.line, col = t.col }
  end
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
          local at = peek()
          if at.k == "kw" and (at.v == "i64" or at.v == "bool") then
            next_tok()
            args[#args + 1] = { tyval = at.v, line = at.line, col = at.col }
          else
            args[#args + 1] = parse_expr(1)
          end
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
      local mt2 = peek()
      local member = nil
      if mt2.k == "int" then
        next_tok()
        member = mt2
      else
        member = expect_name()
      end
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
  if t.k == "kw" and t.v == "fn" then
    -- `fn (x: T, ...) R { body }` - a closure value (SPEC 6.4).
    next_tok()
    local lp = peek()
    if not (lp.k == "punct" and lp.v == "(") then
      fail(lp.line, lp.col, "fn literal needs a parameter list")
    end
    next_tok()
    local params = {}
    if not at_punct(")") then
      while true do
        local pn = expect_name()
        local pty = nil
        if at_punct(":") then
          next_tok()
          pty = parse_type()
        end
        params[#params + 1] = { name = pn.v, ty = pty, line = pn.line, col = pn.col }
        if at_punct(",") then
          next_tok()
        else
          break
        end
      end
    end
    local rp = peek()
    if not (rp.k == "punct" and rp.v == ")") then
      fail(rp.line, rp.col, "expected `)` after the fn literal's parameters")
    end
    next_tok()
    local rty = nil
    if not at_punct("{") then
      rty = parse_type()
    end
    local body = parse_block()
    return { lam = { params = params, ret = rty, body = body }, line = t.line, col = t.col }
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
  if t.k == "kw" and t.v == "fn" then
    -- `fn (T, ...) R` - a first-class function type (SPEC 6.4).
    next_tok()
    local lp = peek()
    if not (lp.k == "punct" and lp.v == "(") then
      fail(lp.line, lp.col, "fn type needs a parameter list: `fn (T, ...) R`")
    end
    next_tok()
    local ps = {}
    if not at_punct(")") then
      while true do
        local pt = parse_type()
        ps[#ps + 1] = pt
        if at_punct(",") then
          next_tok()
        else
          break
        end
      end
    end
    local rp = peek()
    if not (rp.k == "punct" and rp.v == ")") then
      fail(rp.line, rp.col, "expected `)` after the fn type's parameters")
    end
    next_tok()
    local rty = parse_type()
    return { fnp = ps, fnr = rty }
  end
  if t.k == "punct" and t.v == "." then
    -- `.Variant` / `.Variant(args)` - the type-elided constructor
    -- (SPEC 6.5's `.Err(...)` shape; the target type comes from the
    -- expected-type context).
    next_tok()
    local vn = peek()
    if vn.k ~= "name" then
      fail(vn.line, vn.col, "expected a variant name after `.`")
    end
    next_tok()
    local varcon_args = {}
    if at_punct("(") then
      next_tok()
      if not at_punct(")") then
        while true do
          varcon_args[#varcon_args + 1] = parse_expr(1)
          if at_punct(",") then
            next_tok()
          else
            break
          end
        end
      end
      expect_punct(")")
    end
    return { varcon = { name = vn.v, args = varcon_args }, line = t.line, col = t.col }
  end
  if t.k == "punct" and t.v == "[" then
    -- `[e1, e2]` / `[]` (D-112): the element type comes from the
    -- TARGET annotation (a binding or a field's declared type), never
    -- from inference.
    next_tok()
    local items = {}
    while not at_punct("]") do
      items[#items + 1] = parse_expr(1)
      if at_punct(",") then
        next_tok()
      else
        break
      end
    end
    if not at_punct("]") then
      local bt = peek()
      fail(bt.line, bt.col, "expected `]` to close the collection literal")
    end
    next_tok()
    return { list = items, line = t.line, col = t.col }
  end
  if t.k == "name" then
    next_tok()
    return { ref = { name = t.v }, line = t.line, col = t.col }
  end
  if t.k == "punct" and t.v == "(" then
    next_tok()
    local first = parse_expr(1)
    if at_punct(",") then
      local items = { first }
      while at_punct(",") do
        next_tok()
        items[#items + 1] = parse_expr(1)
      end
      expect_punct(")")
      return { tuple = items, line = t.line, col = t.col }
    end
    expect_punct(")")
    return first
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
  local cond = parse_expr(1)
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
    base = parse_expr(1)
    expect_punct("|")
  end
  while not at_punct("}") do
    local f = expect_name()
    if at_punct(":") then
      next_tok()
      local e = parse_expr(1)
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
  local scrut = parse_expr(1)
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
    local body = parse_expr(1)
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
  if t.k == "punct" and t.v == "." then
    -- `.Variant` / `.Variant(pat)` - the type-elided pattern (the
    -- scrutinee's union supplies the type; SPEC 6.5's `.Err(x)`).
    next_tok()
    local v = expect_name()
    local sub = nil
    if at_punct("(") then
      next_tok()
      sub = parse_pattern()
      expect_punct(")")
    end
    return { pat = "dot", variant = v.v, sub = sub, line = t.line, col = t.col }
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

-- `const {a, b} = e;` / `var {a, b} = e;` destructuring shorthand
-- (D-107): each NAME is a field of the struct-typed initializer bound
-- to a local of the same name. Braces hold field names only; renames
-- and nested patterns arrive with C2's pattern machinery.
local function parse_destr(kw_tok, mutable)
  next_tok() -- {
  local names = {}
  while true do
    local field = expect_name()
    local local_name = field
    if at_punct(":") then
      next_tok()
      local_name = expect_name()
    end
    names[#names + 1] = { field = field.v, name = local_name.v, line = field.line, col = field.col }
    if at_punct(",") then
      next_tok()
      if at_punct("}") then
        break
      end
    else
      break
    end
  end
  expect_punct("}")
  expect_punct("=")
  local e = parse_expr(1)
  expect_punct(";")
  return { mutable = mutable, names = names, expr = e, line = kw_tok.line, col = kw_tok.col }
end

-- `const (x, y) = t;` - tuple destructuring (D-114): names only, in
-- order, no field specs.
local function parse_tdestr(kw_tok, mutable)
  next_tok() -- (
  local names = {}
  while true do
    local nm = expect_name()
    names[#names + 1] = { name = nm.v, line = nm.line, col = nm.col }
    if at_punct(",") then
      next_tok()
    else
      break
    end
  end
  expect_punct(")")
  expect_punct("=")
  local e = parse_expr(1)
  expect_punct(";")
  return { mutable = mutable, names = names, expr = e, line = kw_tok.line, col = kw_tok.col }
end

-- Bounds-safe lookahead: reads past the eof sentinel as eof (the
-- multi-token lookahead needs more than one slot).
local function tok_ahead(off)
  local x = P.toks[P.pos + off]
  if x == nil then
    return { k = "eof", v = "", line = 1, col = 1 }
  end
  return x
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
      if at_punct("(") then
        local td = parse_tdestr(t, false)
        stmts[#stmts + 1] = { tdestr = td }
      elseif at_punct("{") then
        local d = parse_destr(t, false)
        stmts[#stmts + 1] = { destr = d }
      else
        local name = expect_name()
        local ty = nil
        if at_punct(":") then
          next_tok()
          ty = parse_type()
        end
        expect_punct("=")
        local e = parse_expr(1)
        expect_punct(";")
        stmts[#stmts + 1] = { const = { name = name.v, ty = ty, expr = e, line = name.line, col = name.col } }
      end
    elseif t.k == "kw" and t.v == "var" then
      next_tok()
      if at_punct("(") then
        local td2 = parse_tdestr(t, true)
        stmts[#stmts + 1] = { tdestr = td2 }
      elseif at_punct("{") then
        local d = parse_destr(t, true)
        stmts[#stmts + 1] = { destr = d }
      else
        local name = expect_name()
        local ty = nil
        if at_punct(":") then
          next_tok()
          ty = parse_type()
        end
        expect_punct("=")
        local e = parse_expr(1)
        expect_punct(";")
        stmts[#stmts + 1] = { var = { name = name.v, ty = ty, expr = e, line = name.line, col = name.col } }
      end
    elseif t.k == "kw" and t.v == "return" then
      next_tok()
      if at_punct(";") then
        next_tok()
        stmts[#stmts + 1] = { ret = { expr = nil, line = t.line, col = t.col } }
      else
        local e = parse_expr(1)
        expect_punct(";")
        stmts[#stmts + 1] = { ret = { expr = e, line = t.line, col = t.col } }
      end
    elseif t.k == "kw" and t.v == "while" then
      next_tok()
      local cond = parse_expr(1)
      local body = parse_block()
      stmts[#stmts + 1] = { loop = { cond = cond, body = body, line = t.line, col = t.col } }
    elseif t.k == "kw" and t.v == "if" then
      local iff = parse_if()
      stmts[#stmts + 1] = { ifstmt = iff.iff }
    elseif t.k == "name" and tok_ahead(1).k == "punct" and tok_ahead(1).v == "."
       and tok_ahead(2).k == "name"
       and tok_ahead(3).k == "punct" and tok_ahead(3).v == "=" then
      -- D-112 field assignment: `e.field = expr;`
      local base = next_tok()
      next_tok() -- `.`
      local fld = next_tok()
      next_tok() -- `=`
      local e = parse_expr(1)
      expect_punct(";")
      stmts[#stmts + 1] = { fassign = { name = base.v, field = fld.v, expr = e, line = base.line, col = base.col } }
    elseif t.k == "name" and P.toks[P.pos + 1].k == "punct" and P.toks[P.pos + 1].v == "=" then
      local name = next_tok()
      next_tok() -- `=`
      local e = parse_expr(1)
      expect_punct(";")
      stmts[#stmts + 1] = { assign = { name = name.v, expr = e, line = name.line, col = name.col } }
    elseif t.k == "kw" and t.v == "match" then
      local e = parse_expr(1)
      if at_punct(";") then
        next_tok()
        stmts[#stmts + 1] = { expr = e }
      else
        tail = e
        expect_punct("}")
        break
      end
    elseif t.k == "kw" and (t.v == "true" or t.v == "false") then
      -- a bare bool literal as a statement or block tail (b0's
      -- statement grammar owns `true`/`false` too).
      local e = parse_expr(1)
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
      local e = parse_expr(1)
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
        elseif d.k == "punct" and d.v == "[" then
          -- Collection-typed defaults (D-112): the literal takes the
          -- field's declared element type at emission.
          default = parse_expr(1)
        elseif d.k == "punct" and d.v == "%" then
          -- Constructor defaults (D-112): %T{...} takes the field's
          -- declared type at emission.
          default = parse_expr(1)
        elseif d.k == "name" then
          -- Qualified defaults (D-112): enum variants like State.Ok.
          default = parse_expr(1)
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
      local is_mut = false
      local is_ct = false
      local mt = peek()
      if mt.k == "kw" and mt.v == "comptime" then
        next_tok()
        is_ct = true
      end
      if not is_ct and mt.k == "name" and mt.v == "mut" then
        next_tok()
        is_mut = true
      end
      local p = expect_name()
      expect_punct(":")
      local ty = parse_type()
      if is_ct then
        if not (is_named(ty) and ty.named == "type") then
          fail(p.line, p.col, "a comptime parameter is declared `comptime NAME: type` (seed v0, D-113)")
        end
        if is_mut then
          fail(p.line, p.col, "a comptime parameter cannot be mut")
        end
      end
      if is_mut and not is_named(ty) then
        fail(p.line, p.col, "mut parameters take a record type (seed v0, D-112)")
      end
      params[#params + 1] = { name = p.v, ty = ty, mut = is_mut, is_comptime = is_ct, line = p.line, col = p.col }
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

-- A digits-only slot reader for tuple members (`t.0`); the host's
-- tonumber/string.byte are outside the verified lua subset the seed
-- runs under, so digits come from a lookup table.
local DIGIT_VAL = {}
DIGIT_VAL["0"] = 0
DIGIT_VAL["1"] = 1
DIGIT_VAL["2"] = 2
DIGIT_VAL["3"] = 3
DIGIT_VAL["4"] = 4
DIGIT_VAL["5"] = 5
DIGIT_VAL["6"] = 6
DIGIT_VAL["7"] = 7
DIGIT_VAL["8"] = 8
DIGIT_VAL["9"] = 9

-- A `sep`-joined string from a list (table.concat is outside the
-- verified lua subset the seed runs under).
local function join_strs(parts, sep)
  local acc = ""
  for i = 1, #parts do
    if i > 1 then
      acc = acc .. sep
    end
    acc = acc .. parts[i]
  end
  return acc
end

local function parse_slot(s)
  local n = 0
  if #s == 0 then
    return nil
  end
  for i = 1, #s do
    local d = DIGIT_VAL[string.sub(s, i, i)]
    if d == nil then
      return nil
    end
    n = n * 10 + d
  end
  return n
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
local function record_capture(name, b)
  for i = 1, #E.caps do
    if E.caps[i].name == name then
      return E.caps[i].proxy
    end
  end
  local cap_kind = "ssa"
  local storage = value_ir(b.ty, 1, 1)
  if b.kind == "box" or b.kind == "mut" then
    cap_kind = "box"
    storage = box_ir(b.ty, 1, 1)
  end
  local proxy = {
    kind = "cap",
    name = name,
    ty = b.ty,
    box = box_ir(b.ty, 1, 1),
    cap_idx = #E.caps,
    cap_kind = cap_kind,
    cap_storage = storage,
    cap_box = box_ir(b.ty, 1, 1),
  }
  E.caps[#E.caps + 1] = { name = name, orig = b, proxy = proxy }
  return proxy
end

local function lookup(name)
  local i = #E.scopes
  while i >= 1 do
    if E.scopes[i][name] ~= nil then
      local b = E.scopes[i][name]
      local bar = E.capbar
      if bar ~= nil and i < bar then
        return record_capture(name, b)
      end
      return b
    end
    i = i - 1
  end
  return nil
end

local function ir_ty(ty, line, col)
  if is_tup(ty) then
    return value_ir(ty, line, col)
  end
  if is_opt(ty) or is_res(ty) then
    return value_ir(ty, line, col)
  end
  if is_fn(ty) then
    return fn_ir(ty, line, col)
  end
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
  if op == "*" then
    -- multiply: overflow iff b != 0 and the wrapped product does not
    -- divide back. min * -1 lands the quotient on 0 via select (so the
    -- check fires) and the divisor selects to 1 when b == 0 (no
    -- div-by-zero; the b != 0 conjunct rejects that path's
    -- false positive). No divsi(min, -1) is ever executed.
    local resm = fresh()
    emit(resm .. " = arith.muli " .. a .. ", " .. b .. " : i64")
    local bnz = fresh()
    emit(bnz .. " = arith.cmpi ne, " .. b .. ", " .. zero .. " : i64")
    local bz = fresh()
    emit(bz .. " = arith.cmpi eq, " .. b .. ", " .. zero .. " : i64")
    local one1 = const_i64("1")
    local bsafe = fresh()
    emit(bsafe .. " = arith.select " .. bz .. ", " .. one1 .. ", " .. b .. " : i64")
    local maxlit = const_i64("9223372036854775807")
    local one2 = const_i64("1")
    local m0 = fresh()
    local minv = fresh()
    local minus1 = fresh()
    emit(m0 .. " = arith.subi " .. zero .. ", " .. maxlit .. " : i64")
    emit(minv .. " = arith.subi " .. m0 .. ", " .. one2 .. " : i64")
    emit(minus1 .. " = arith.subi " .. zero .. ", " .. one2 .. " : i64")
    local amin = fresh()
    local bm1 = fresh()
    local special = fresh()
    emit(amin .. " = arith.cmpi eq, " .. a .. ", " .. minv .. " : i64")
    emit(bm1 .. " = arith.cmpi eq, " .. b .. ", " .. minus1 .. " : i64")
    emit(special .. " = arith.andi " .. amin .. ", " .. bm1 .. " : i1")
    local zr = fresh()
    emit(zr .. " = arith.select " .. special .. ", " .. zero .. ", " .. resm .. " : i64")
    local q = fresh()
    emit(q .. " = arith.divsi " .. zr .. ", " .. bsafe .. " : i64")
    local nq = fresh()
    emit(nq .. " = arith.cmpi ne, " .. q .. ", " .. a .. " : i64")
    local ovf = fresh()
    emit(ovf .. " = arith.andi " .. nq .. ", " .. bnz .. " : i1")
    guard(ovf, loc_msg(line, col, "CQ0003", "integer overflow (i64 mul)"), line, col)
    return resm
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

local function emit_cmp(op, a, b, line, col)
  -- Str and bool operands (found by calq0's lexer, the first b0
  -- program to compare two Str/bool VALUES): == / != via
  -- emit_equality + an i1 negation; ordering stays i64-only (SPEC
  -- F-notes: use lookup tables for strings).
  local ty = E.vtypes[a]
  if ty == nil then
    ty = E.vtypes[b]
  end
  if ty == "i64" or ty == "bool" or ty == "Str" then
    if op == "==" or op == "!=" then
      local eqr = emit_equality(a, b, ty, line, col)
      if op == "==" then
        return eqr
      end
      local t = fresh()
      emit(t .. " = arith.constant true")
      local n = fresh()
      emit(n .. " = arith.xori " .. eqr .. ", " .. t .. " : i1")
      return n
    end
    if ty == "bool" then
      fail(line, col, "boolean ordering is not in the seed (use == / !=)")
    end
    if ty == "Str" then
      fail(line, col, "string ordering is outside the verified subset (use == / !=)")
    end
  end
  local res = fresh()
  local pred = cmp_pred(op)
  emit(res .. " = arith.cmpi " .. pred .. ", " .. a .. ", " .. b .. " : i64")
  return res
end

-- ------------------------------- expressions

local emit_expr = nil -- forward (initializer-required locals)
local emit_lambda = nil -- forward (closures, SPEC 6.4)

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

function is_opt(ty)
  return ty ~= nil and type(ty) == "table" and ty.opt ~= nil
end

function is_res(ty)
  return ty ~= nil and type(ty) == "table" and ty.reserr ~= nil
end

function is_tup(ty)
  return ty ~= nil and type(ty) == "table" and ty.tup ~= nil
end

function is_fn(ty)
  return ty ~= nil and type(ty) == "table" and ty.fnp ~= nil
end

function is_vec(ty)
  return ty ~= nil and type(ty) == "table" and ty.vec ~= nil
end

-- Canonical type instances: the seed's match-join compares result
-- types by IDENTITY, and record/union-typed match results (calq0's
-- parser is the first b0 program whose matches yield named types)
-- need every {named=X} / {vec=T} occurrence to be the SAME Lua table
-- per type. TY_CACHE interns both forms.
TY_CACHE = {}

function named_ty(name)
  local t = TY_CACHE["n:" .. name]
  if t == nil then
    t = { named = name }
    TY_CACHE["n:" .. name] = t
  end
  return t
end

function ty_key(ty)
  if type(ty) == "table" then
    if ty.named ~= nil then
      return "n:" .. ty.named
    end
    if ty.vec ~= nil then
      return "v:" .. ty_key(ty.vec)
    end
    return "?"
  end
  return tostring(ty)
end

function vec_ty(inner)
  local key = "v:" .. ty_key(inner)
  local t = TY_CACHE[key]
  if t == nil then
    t = { vec = inner }
    TY_CACHE[key] = t
  end
  return t
end

function same_type(a, b)
  if is_tup(a) or is_tup(b) then
    if not (is_tup(a) and is_tup(b) and #a.tup == #b.tup) then
      return false
    end
    for i = 1, #a.tup do
      if not same_type(a.tup[i], b.tup[i]) then
        return false
      end
    end
    return true
  end
  if is_opt(a) or is_opt(b) then
    return is_opt(a) and is_opt(b) and same_type(a.opt, b.opt)
  end
  if is_res(a) or is_res(b) then
    return is_res(a) and is_res(b) and same_type(a.reserr, b.reserr) and same_type(a.resok, b.resok)
  end
  if is_fn(a) or is_fn(b) then
    if not (is_fn(a) and is_fn(b)) then
      return false
    end
    if #a.fnp ~= #b.fnp then
      return false
    end
    if not same_type(a.fnr, b.fnr) then
      return false
    end
    for i = 1, #a.fnp do
      if not same_type(a.fnp[i], b.fnp[i]) then
        return false
      end
    end
    return true
  end
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

-- Collection literal emission (D-112): the element type is the
-- ANNOTATION's inner type; each element goes through the mirrored
-- vector prelude (named slots wrap like vector.push).
function emit_list_literal(ty, node, line, col)
  if not is_vec(ty) then
    fail(line, col, "a collection literal needs a [T]/Vector annotation (seed v0)")
  end
  local slot, prefix = vec_slot(ty.vec, line, col)
  local v = fresh()
  emit(v .. " = func.call @" .. prefix .. "_new() : () -> " .. vec_box_ir(ty.vec, line, col))
  for i = 1, #node.list do
    local xid = emit_expr(node.list[i])
    local xty = E.vtypes[xid]
    if not same_type(xty, ty.vec) then
      fail(node.line, node.col, "collection literal element type does not match the [T] annotation")
    end
    local stored = xid
    if is_named(ty.vec) then
      stored, _ = wrap_store(xid, xty, node.line, node.col)
    end
    emit("func.call @" .. prefix .. "_push(" .. v .. ", " .. stored .. ") : (" .. vec_box_ir(ty.vec, line, col) .. ", " .. slot .. ") -> ()")
  end
  E.vtypes[v] = ty
  return v
end

-- Typed expression emission: literals take their element type from
-- `ty`; `.Variant` constructors synthesize the qualified callee the
-- ordinary variant path resolves; everything else is the ordinary
-- path (no inference).
function emit_expr_typed(node, ty)
  if node.tuple ~= nil then
    local tv2, tty2 = emit_tuple(node, ty)
    return tv2, tty2
  end
  if node.list ~= nil then
    return emit_list_literal(ty, node, node.line, node.col)
  end
  if node.lam ~= nil and is_fn(ty) then
    -- Inferred closure parameters (D-086: `fn (x) { ... }` at a
    -- fn-typed position takes its signature from the expected type).
    local l = node.lam
    local nps = #ty.fnp
    if #l.params ~= nps then
      fail(node.line or 1, node.col or 1, "the closure's parameter count does not match the expected fn type")
    end
    for i = 1, #l.params do
      local lp = l.params[i]
      if lp.ty == nil then
        lp.ty = ty.fnp[i]
      elseif not same_type(lp.ty, ty.fnp[i]) then
        fail(lp.line or 1, lp.col or 1, "the closure's parameter type does not match the expected fn type")
      end
    end
    if l.ret == nil then
      l.ret = ty.fnr
    elseif not same_type(l.ret, ty.fnr) then
      fail(node.line or 1, node.col or 1, "the closure's return type does not match the expected fn type")
    end
    local lv = emit_lambda(node)
    if lv == nil then
      fail(node.line or 1, node.col or 1, "closure produced no value")
    end
    return lv, ty
  end
  if node.varcon ~= nil then
    if is_opt(ty) or is_res(ty) then
      local sd = synth_decl(ty, node.line, node.col)
      local v2 = emit_variant_make(sd, ty, node.varcon.name, node.varcon.args, node.line, node.col)
      if v2 == nil then
        fail(node.line, node.col, "`.Variant` produced no value")
      end
      return v2
    end
    if not is_named(ty) then
      fail(node.line, node.col, "`.Variant` needs a union-typed context (annotate the binding or return a union)")
    end
    local synth = { call = { name = node.varcon.name, mod = ty.named }, args = node.varcon.args, line = node.line, col = node.col }
    local v = emit_expr(synth)
    if v == nil then
      fail(node.line, node.col, "`.Variant` produced no value")
    end
    return v
  end
  return emit_expr(node)
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
  E.vtypes[res] = vec_ty(inner)
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
  if is_tup(ty) then
    return value_ir(ty, line, col)
  end
  if is_opt(ty) or is_res(ty) then
    return value_ir(ty, line, col)
  end
  if is_fn(ty) then
    return fn_ir(ty, line, col)
  end
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

function fn_ir(ty, line, col)
  local ps = {}
  for i = 1, #ty.fnp do
    ps[#ps + 1] = value_ir(ty.fnp[i], line, col)
  end
  return "!frk_closure.fn<[" .. join_parts(ps) .. "], [" .. value_ir(ty.fnr, line, col) .. "]>"
end

function value_ir(ty, line, col)
  if is_tup(ty) then
    local parts = {}
    for i = 1, #ty.tup do
      parts[#parts + 1] = storage_ir(ty.tup[i], line, col)
    end
    return "!frk_adt.product<[" .. join_strs(parts, ", ") .. "]>"
  end
  if is_opt(ty) then
    return "!frk_adt.sum<[[" .. storage_ir(ty.opt, line, col) .. "], []]>"
  end
  if is_res(ty) then
    return "!frk_adt.sum<[[" .. storage_ir(ty.resok, line, col) .. "], [" .. storage_ir(ty.reserr, line, col) .. "]]>"
  end
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
  if is_vec(ty) then
    -- Vector-typed terminators (calq0's emitter is the first b0 code
    -- with checked arithmetic inside Vector-returning functions): an
    -- empty vector via the mirrored intrinsic - the trap path never
    -- returns, this only keeps the IR well-formed.
    local slot, prefix = vec_slot(ty.vec, line, col)
    local v = fresh()
    emit(v .. " = func.call @" .. prefix .. "_new() : () -> " .. vec_box_ir(ty.vec, line, col))
    return v
  end
  if is_tup(ty) then
    local acc = nil
    local acc_ir = nil
    for i = 1, #ty.tup do
      local p = poison(ty.tup[i], line, col)
      if acc == nil then
        acc = fresh()
        emit(acc .. " = \"frk_adt.product_new\"() : () -> !frk_adt.product<[]>")
        acc_ir = "!frk_adt.product<[]>"
      end
      local nv = fresh()
      emit(nv .. " = \"frk_adt.product_snoc\"(" .. acc .. ", " .. p .. ") : (" .. acc_ir .. ", " .. storage_ir(ty.tup[i], line, col) .. ") -> !frk_adt.product<[" .. storage_ir(ty.tup[i], line, col) .. "]>")
      acc = nv
      acc_ir = nil
    end
    if acc == nil then
      acc = fresh()
      emit(acc .. " = \"frk_adt.product_new\"() : () -> !frk_adt.product<[]>")
    end
    E.vtypes[acc] = ty
    return acc
  end
  if is_opt(ty) or is_res(ty) then
    -- Structural unions (SPEC 6.5): variant 0 (Some/Ok) with a
    -- poisoned payload, or the payload-free first variant.
    local sd = synth_decl(ty, line, col)
    local v0 = sd.variants[1]
    local uir = value_ir(ty, line, col)
    if v0.ty == nil then
      local e = fresh()
      emit(e .. " = \"frk_adt.product_new\"() : () -> !frk_adt.product<[]>")
      local s = fresh()
      emit(s .. " = \"frk_adt.make_sum\"(" .. e .. ") {variant = 0 : i64} : (!frk_adt.product<[]>) -> " .. uir)
      E.vtypes[s] = ty
      return s
    end
    local p = poison(v0.ty, line, col)
    local stored, sty = wrap_store(p, v0.ty, line, col)
    local prod = emit_product_single(stored, sty)
    local s = fresh()
    emit(s .. " = \"frk_adt.make_sum\"(" .. prod.value .. ") {variant = 0 : i64} : (" .. prod.ty .. ") -> " .. uir)
    E.vtypes[s] = ty
    return s
  end
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
    -- frk_mem.rec_ref erases a box of a PRODUCT only (the mem
    -- dialect's verifier, D-074). Unions therefore nest: the sum is
    -- boxed, the box is wrapped in a single-field product box, and
    -- THAT box is erased: recref -> box<product<[box<sum>]>>.
    local sb = fresh()
    emit(sb .. " = \"frk_mem.box_new\"(" .. value .. ") : (" .. vty .. ") -> !frk_mem.box<" .. vty .. ">")
    local e = fresh()
    emit(e .. " = \"frk_adt.product_new\"() : () -> !frk_adt.product<[]>")
    local p1 = fresh()
    emit(p1 .. " = \"frk_adt.product_snoc\"(" .. e .. ", " .. sb .. ") : (!frk_adt.product<[]>, !frk_mem.box<" .. vty .. ">) -> !frk_adt.product<[!frk_mem.box<" .. vty .. ">]>")
    local pb = fresh()
    emit(pb .. " = \"frk_mem.box_new\"(" .. p1 .. ") : (!frk_adt.product<[!frk_mem.box<" .. vty .. ">]>) -> !frk_mem.box<!frk_adt.product<[!frk_mem.box<" .. vty .. ">]>>")
    box = pb
    ref_operand_ty = "!frk_mem.box<!frk_adt.product<[!frk_mem.box<" .. vty .. ">]>>"
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
    cast_ty = "!frk_mem.box<!frk_adt.product<[!frk_mem.box<" .. vty .. ">]>>"
  end
  local box = fresh()
  emit(box .. " = \"frk_mem.rec_cast\"(" .. stored .. ") : (!frk_mem.recref) -> " .. cast_ty)
  if d.kind == "union" then
    local prod = fresh()
    emit(prod .. " = \"frk_mem.box_get\"(" .. box .. ") : (" .. cast_ty .. ") -> !frk_adt.product<[!frk_mem.box<" .. vty .. ">]>")
    local inb = fresh()
    emit(inb .. " = \"frk_adt.get\"(" .. prod .. ") {field = 0 : i64} : (!frk_adt.product<[!frk_mem.box<" .. vty .. ">]>) -> !frk_mem.box<" .. vty .. ">")
    local s = fresh()
    emit(s .. " = \"frk_mem.box_get\"(" .. inb .. ") : (!frk_mem.box<" .. vty .. ">) -> " .. vty)
    return s
  end
  return box
end

-- Reads a binding to its VALUE ssa + logical type.
-- The env slot for a capture (the lazy env_load + adt.get pair).
function cap_slot(b, line, col)
  local slot = fresh()
  emit(slot .. " = \"frk_closure.env_load\"(%env) {index = " .. tostring(b.cap_idx) .. " : i64, env = " .. E.capenv_ty .. "} : (!frk_closure.envref) -> " .. b.cap_storage)
  return slot
end

-- The binding's raw slot (the value for ssa, the box for boxes, the
-- loaded slot for captures) - what assignment writes through.
function binding_slot(b, line, col)
  if b.kind == "cap" then
    return cap_slot(b, line, col)
  end
  return b.id
end

function read_binding(b, line, col)
  if b.kind == "ssa" then
    return b.id, b.ty
  end
  if b.kind == "cap" then
    local slot = cap_slot(b, line, col)
    if b.cap_kind == "box" then
      local v = fresh()
      emit(v .. " = \"frk_mem.box_get\"(" .. slot .. ") : (" .. box_ir(b.ty, line, col) .. ") -> " .. value_ir(b.ty, line, col))
      return v, b.ty
    end
    return slot, b.ty
  end
  local v = fresh()
  emit(v .. " = \"frk_mem.box_get\"(" .. b.id .. ") : (" .. box_ir(b.ty, line, col) .. ") -> " .. value_ir(b.ty, line, col))
  return v, b.ty
end

-- The virtual union decl of a structural Option/Result (SPEC 6.5);
-- the whole match/variant/try machinery reads decls uniformly.
function synth_decl(ty, line, col)
  if is_opt(ty) then
    local vs = {
      { name = "Some", ty = ty.opt, line = line, col = col },
      { name = "None", ty = nil, line = line, col = col },
    }
    return { name = "", kind = "union", variants = vs }
  end
  if is_res(ty) then
    local vs = {
      { name = "Ok", ty = ty.resok, line = line, col = col },
      { name = "Err", ty = ty.reserr, line = line, col = col },
    }
    return { name = "", kind = "union", variants = vs }
  end
  return nil
end

function decl_of(ty, line, col)
  if is_named(ty) then
    return find_type(ty.named, line, col)
  end
  local sd = synth_decl(ty, line, col)
  if sd == nil then
    fail(line, col, "internal: no decl for this type")
  end
  return sd
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
      value = emit_expr_typed(provided[field.name].expr, field.ty)
    elseif base_value ~= nil then
      local stored = fresh()
      emit(stored .. " = \"frk_mem.field_get\"(" .. base_value .. ") {field = " .. tostring(index - 1) .. " : i64} : (" .. value_ir(named_ty(m.ty), line, col) .. ") -> " .. storage_ir(field.ty, field.line, field.col))
      value = unwrap_load(stored, field.ty, field.line, field.col)
    elseif field.default ~= nil then
      value = emit_expr_typed(field.default, field.ty)
    else
      fail(line, col, "missing required field `" .. field.name .. "` in %" .. m.ty .. "{...} (no default declared)")
    end
    local stored, sty = wrap_store(value, field.ty, field.line, field.col)
    return stored, sty
  end)
  local box = fresh()
  emit(box .. " = \"frk_mem.box_new\"(" .. prod.value .. ") : (" .. prod.ty .. ") -> " .. value_ir(named_ty(m.ty), line, col))
  E.vtypes[box] = named_ty(m.ty)
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
      emit(stored .. " = \"frk_mem.field_get\"(" .. value .. ") {field = " .. tostring(idx - 1) .. " : i64} : (" .. value_ir(named_ty(d.name), fp.line, fp.col) .. ") -> " .. storage_ir(fld.ty, fld.line, fld.col))
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
  if sub.pat == "dot" then
    if not (is_named(lty) or is_opt(lty) or is_res(lty)) then
      fail(sub.line, sub.col, "`.Variant` patterns need a union subject")
    end
    local d = decl_of(lty, sub.line, sub.col)
    if d.kind ~= "union" then
      fail(sub.line, sub.col, "`.Variant` patterns need a union subject")
    end
    find_variant(d, sub.variant, sub.line, sub.col)
    local tag = fresh()
    emit(tag .. " = \"frk_adt.tag_of\"(" .. value .. ") : (" .. value_ir(lty, sub.line, sub.col) .. ") -> i64")
    local eq = fresh()
    emit(eq .. " = arith.cmpi eq, " .. tag .. ", " .. const_i64(tostring(find_variant(d, sub.variant, sub.line, sub.col) - 1)) .. " : i64")
    return eq
  end
  if sub.pat == "variant" then
    if not (is_named(lty) or is_opt(lty) or is_res(lty)) then
      fail(sub.line, sub.col, "variant patterns need a union subject")
    end
    local d = decl_of(lty, sub.line, sub.col)
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
  local brs = 0

  -- Emits one arm into the CURRENT block. With conditions: a
  -- cond_br splits body vs continuation (the caller proceeds in the
  -- created next block). Without: the body runs IN PLACE — the arm
  -- was already selected by a test, or it is the fall arm.
  local function emit_arm(conditions, bindings, body, pat, false_target)
    local function run_body()
      scope_push()
      collect_bindings(bindings)
      local v = nil
      -- An arm body written `{ ... }` arrives wrapped as
      -- `{ block = ... }`; value-position blocks lift their trailing
      -- `if ... else ...` statement to their value.
      local blk = body
      if body.block ~= nil then
        blk = body.block
      end
      if body.block ~= nil or body.stmts ~= nil then
        v = EMIT_VALUE_BLOCK(blk)
      else
        v = emit_expr(body)
      end
      local vty = v ~= nil and E.vtypes[v] or "unit"
      if not E.terminated then
        if vty == "unit" then
          brs = brs + 1
          terminate("cf.br ^" .. join_label)
        else
          if join_ty == nil then
            join_ty = vty
          elseif not same_type(join_ty, vty) then
            fail(pat.line, pat.col, "match arms produce different types (`" .. tostring(join_ty) .. "` vs `" .. tostring(vty) .. "`)")
          end
          brs = brs + 1
          terminate("cf.br ^" .. join_label .. "(" .. v .. " : " .. value_ir(vty, pat.line, pat.col) .. ")")
        end
      end
      scope_pop()
    end
    if conditions ~= nil then
      local body_label = fresh_label()
      local nlabel = false_target
      if nlabel == nil then
        nlabel = fresh_label()
      end
      emit("cf.cond_br " .. conditions .. ", ^" .. body_label .. ", ^" .. nlabel)
      new_block(body_label)
      run_body()
      new_block(nlabel)
      return "split"
    end
    run_body()
    return "inplace"
  end

  -- classify arms per scrutinee kind
  if is_named(sty) or is_opt(sty) or is_res(sty) then
    local d = decl_of(sty, line, col)
    if d.kind == "union" then
      local variant_arms = {}
      local default_arm = nil
      local covered = {}
      for i = 1, #m.arms do
        local a = m.arms[i]
        if a.pat.pat == "variant" or a.pat.pat == "dot" then
          if a.pat.pat == "variant" then
            if a.pat.ty ~= d.name then
              fail(a.pat.line, a.pat.col, "pattern `" .. a.pat.ty .. "` does not match scrutinee type `" .. d.name .. "`")
            end
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
        local step = emit_arm(conditions, bindings, a.body, a.pat, next_label)
        if step == "inplace" then
          new_block(next_label)
        end
      end
      -- The fall arm emits in the current block; its sub-pattern
      -- cannot fail (checked above).
      local a = fall
      local conditions = nil
      local bindings = {}
      if a.pat.pat == "variant" or a.pat.pat == "dot" then
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
      if brs == 0 then
        E.terminated = true
        return nil
      end
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
    if brs == 0 then
      E.terminated = true
      return nil
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
  if brs == 0 then
    E.terminated = true
    return nil
  end
  new_block(join_label)
  if join_ty ~= nil then
    E.blocks[#E.blocks].label = join_label .. "(" .. join_arg .. ": " .. value_ir(join_ty, line, col) .. ")"
    E.vtypes[join_arg] = join_ty
    return join_arg
  end
  return nil
end

-- `Type.Variant` construction (D-095) and the synthetic Option/Result
-- constructors: the sum value, wrapped at storage sites only.
function emit_variant_make(td, uty, vname, args, line, col)
  local idx = find_variant(td, vname, line, col)
  local v = td.variants[idx]
  local uir = value_ir(uty, line, col)
  if #args == 0 then
    if v.ty ~= nil then
      fail(line, col, "variant `" .. vname .. "` carries a payload (one argument)")
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
    fail(line, col, "variant `" .. vname .. "` has no payload")
  end
  local payload = emit_expr_typed(args[1], v.ty)
  local stored, sty = wrap_store(payload, v.ty, line, col)
  local prod = emit_product_single(stored, sty)
  local s = fresh()
  emit(s .. " = \"frk_adt.make_sum\"(" .. prod.value .. ") {variant = " .. tostring(idx - 1) .. " : i64} : (" .. prod.ty .. ") -> " .. uir)
  E.vtypes[s] = uty
  return s, uty
end

-- A call through a fn value: args ride a product into frk_closure.apply.
local function emit_apply(cb, args, line, col)
  local fv = read_binding(cb, line, col)
  if #args ~= #cb.ty.fnp then
    fail(line, col, "closure expects " .. tostring(#cb.ty.fnp) .. " argument(s), got " .. tostring(#args))
  end
  local e0 = fresh()
  emit(e0 .. " = \"frk_adt.product_new\"() : () -> !frk_adt.product<[]>")
  local acc = e0
  local acc_ty = "!frk_adt.product<[]>"
  local fields = {}
  for i = 1, #args do
    local av = emit_expr(args[i])
    local aty = E.vtypes[av]
    if not same_type(aty, cb.ty.fnp[i]) then
      fail(line, col, "closure argument type mismatch")
    end
    local st = value_ir(aty, line, col)
    fields[#fields + 1] = st
    local nxt = fresh()
    emit(nxt .. " = \"frk_adt.product_snoc\"(" .. acc .. ", " .. av .. ") : (" .. acc_ty .. ", " .. st .. ") -> !frk_adt.product<[" .. join_parts(fields) .. "]>")
    acc = nxt
    acc_ty = "!frk_adt.product<[" .. join_parts(fields) .. "]>"
  end
  local res = fresh()
  emit(res .. " = \"frk_closure.apply\"(" .. fv .. ", " .. acc .. ") : (" .. fn_ir(cb.ty, line, col) .. ", " .. acc_ty .. ") -> " .. value_ir(cb.ty.fnr, line, col))
  return res, cb.ty.fnr
end

local function emit_call(node)
  local callee = node.call
  local line = node.line
  local col = node.col
  local args = node.args
  local td = callee.mod ~= nil and E.types[callee.mod] or nil
  if td ~= nil then
    if td.kind ~= "union" then
      fail(line, col, "`" .. callee.mod .. "` is a struct type; construct it with `%" .. callee.mod .. "{...}`")
    end
    return emit_variant_make(td, named_ty(td.name), callee.name, args, line, col)
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
  if callee.mod == nil and callee.name == "__calque_input_name" then
    if #args ~= 1 then
      fail(line, col, "__calque_input_name takes exactly one i64")
    end
    local a = emit_expr(args[1])
    local res = fresh()
    emit(res .. " = func.call @__calque_input_name(" .. a .. ") : (i64) -> !frk_bstr.str")
    return res, "Str"
  end
  if callee.mod == nil and callee.name == "__calque_input_ns" then
    if #args ~= 1 then
      fail(line, col, "__calque_input_ns takes exactly one i64")
    end
    local a = emit_expr(args[1])
    local res = fresh()
    emit(res .. " = func.call @__calque_input_ns(" .. a .. ") : (i64) -> !frk_bstr.str")
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
  -- A fn-typed BINDING called as a value: closure apply (SPEC 6.4).
  if callee.mod == nil then
    local cb = lookup(callee.name)
    if cb ~= nil and is_fn(cb.ty) then
      return emit_apply(cb, args, line, col)
    end
  end
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
  if is_generic_fn(fn) then
    local inst, val_args = resolve_generic(fn, args, line, col)
    return emit_fn_call(inst, inst.sym, val_args, line, col)
  end
  return emit_fn_call(fn, sym, args, line, col)
end

-- Monomorphized generic calls (D-113): type arguments are consumed
-- per the fn's comptime parameters; each type tuple emits one
-- specialized function, memoized per compile.
function is_generic_fn(fn)
  for i = 1, #fn.params do
    if fn.params[i].is_comptime then
      return true
    end
  end
  return false
end

function mangle_ty(ty)
  if is_tup(ty) then
    local acc = "tup"
    for i = 1, #ty.tup do
      acc = acc .. "_" .. mangle_ty(ty.tup[i])
    end
    return acc
  end
  if ty == "i64" then
    return "i64"
  end
  if ty == "bool" then
    return "bool"
  end
  if ty == "Str" then
    return "str"
  end
  if ty == "unit" then
    return "unit"
  end
  if is_named(ty) then
    return ty.named
  end
  if is_vec(ty) then
    return "vec_" .. mangle_ty(ty.vec)
  end
  if is_opt(ty) then
    return "opt_" .. mangle_ty(ty.opt)
  end
  if is_res(ty) then
    return "res_" .. mangle_ty(ty.reserr) .. "_" .. mangle_ty(ty.resok)
  end
  if is_fn(ty) then
    return "fn"
  end
  return "t"
end

-- The only rewrite a monomorphization performs: type-position tables
-- ({named = "T"}) standing for a comptime parameter become the
-- concrete type; expressions are copied verbatim.
function subst_ast(node, sub)
  if type(node) ~= "table" then
    return node
  end
  if node.named ~= nil and sub[node.named] ~= nil then
    return sub[node.named]
  end
  local out = {}
  for k, v in pairs(node) do
    out[k] = subst_ast(v, sub)
  end
  return out
end

function is_tyarg_expr(a, line, col)
  if a.tyval ~= nil then
    return true
  end
  if a.ref ~= nil and a.ref.mod == nil then
    -- a bare NAME is a type argument iff it is not a local binding
    if lookup(a.ref.name) == nil then
      return true
    end
  end
  return false
end

function tyarg_value(a, line, col)
  local ty = nil
  if a.tyval ~= nil then
    ty = a.tyval
  elseif a.ref ~= nil and a.ref.mod == nil then
    if a.ref.name == "Str" then
      ty = "Str"
    else
      ty = { named = a.ref.name }
    end
  end
  if ty == nil then
    fail(a.line or line, a.col or col, "a comptime type argument must be a type name or type keyword")
  end
  if E.tvars ~= nil then
    ty = subst_ast(ty, E.tvars)
  end
  return ty
end

-- A TYPE-ONLY probe of an expression, for comptime inference (D-113's
-- revisit: the frozen corpus calls `seq.iter(v)` with no type
-- argument). Never emits: refs read their binding's type; calls
-- resolve (and memoize-queue) the callee instance for its ret; other
-- shapes are unknown.
function infer_expr_ty(expr, line, col)
  if expr.lit == "int" then
    return "i64"
  end
  if expr.tuple ~= nil then
    local tys = {}
    for i = 1, #expr.tuple do
      local et = infer_expr_ty(expr.tuple[i], line, col)
      if et == nil then
        return nil
      end
      if E.tvars ~= nil then
        et = subst_ast(et, E.tvars)
      end
      tys[#tys + 1] = et
    end
    return { tup = tys }
  end
  if expr.bin ~= nil then
    -- a binary expression: comparisons/logic are bool; arithmetic
    -- takes its operand's type (the left unless unknown).
    local op = expr.bin
    if op == "==" or op == "!=" or op == "<" or op == "<=" or op == ">" or op == ">=" or op == "and" or op == "or" then
      return "bool"
    end
    local lt = infer_expr_ty(expr.l, line, col)
    if lt ~= nil then
      return lt
    end
    return infer_expr_ty(expr.r, line, col)
  end
  if expr.lit == "str" then
    return "Str"
  end
  if expr.lit == "bool" then
    return "bool"
  end
  if expr.ref ~= nil and expr.ref.mod == nil then
    local b = lookup(expr.ref.name)
    if b ~= nil then
      return b.ty
    end
    -- a bare top-level fn name used as a value: its signature
    local f2 = E.fns[expr.ref.name]
    if f2 ~= nil and not is_generic_fn(f2) then
      local ps = {}
      for i = 1, #f2.params do
        ps[#ps + 1] = subst_ast(f2.params[i].ty, E.tvars or {})
      end
      local rt = subst_ast(f2.ret, E.tvars or {})
      return { fnp = ps, fnr = rt }
    end
    return nil
  end
  if expr.ref ~= nil and expr.ref.mod ~= nil then
    -- a field access: the receiver's named type's field, quietly
    local b = lookup(expr.ref.mod)
    if b == nil then
      return nil
    end
    if is_tup(b.ty) then
      local nidx = parse_slot(expr.ref.name)
      if nidx ~= nil and nidx >= 0 and nidx < #b.ty.tup then
        local sty = b.ty.tup[nidx + 1]
        if E.tvars ~= nil then
          return subst_ast(sty, E.tvars)
        end
        return sty
      end
      return nil
    end
    if not is_named(b.ty) then
      return nil
    end
    local td = E.types[b.ty.named]
    if td == nil or td.kind == "union" then
      return nil
    end
    local fd = nil
    for i = 1, #td.fields do
      if td.fields[i].name == expr.ref.name then
        fd = td.fields[i]
      end
    end
    if fd == nil then
      return nil
    end
    if E.tvars ~= nil then
      return subst_ast(fd.ty, E.tvars)
    end
    return fd.ty
  end
  if expr.call ~= nil then
    if expr.call.mod ~= nil then
      -- the builtin namespaces have no AST fns (the emit_call
      -- builtin branches' mirror)
      if expr.call.mod == "str" then
        if expr.call.name == "len" then
          return "i64"
        end
        if expr.call.name == "eq" then
          return "bool"
        end
        return "Str"
      end
      if expr.call.mod == "io" then
        return "unit"
      end
      if expr.call.mod == "vector" then
        if expr.call.name == "len" then
          return "i64"
        end
        return nil
      end
    end
    local fn2 = nil
    if expr.call.mod ~= nil then
      fn2 = E.fns_by_sym[expr.call.mod .. "__" .. expr.call.name]
    else
      fn2 = E.fns[expr.call.name]
    end
    if fn2 == nil then
      return nil
    end
    if is_generic_fn(fn2) then
      local inst = resolve_generic(fn2, expr.args, expr.line or line, expr.col or col)
      return inst.ret
    end
    return fn2.ret
  end
  return nil
end

function unify_ty(declared, actual, sub, names)
  if declared == nil or actual == nil then
    return
  end
  if is_named(declared) and names[declared.named] then
    if sub[declared.named] == nil then
      sub[declared.named] = actual
    end
    return
  end
  if is_tup(declared) then
    if is_tup(actual) then
      if #declared.tup == #actual.tup then
        for i = 1, #declared.tup do
          unify_ty(declared.tup[i], actual.tup[i], sub, names)
        end
      end
    end
    return
  end
  if is_vec(declared) then
    if is_vec(actual) then
      unify_ty(declared.vec, actual.vec, sub, names)
    end
    return
  end
  if is_opt(declared) then
    if is_opt(actual) then
      unify_ty(declared.opt, actual.opt, sub, names)
    end
    return
  end
  if is_res(declared) then
    if is_res(actual) then
      unify_ty(declared.reserr, actual.reserr, sub, names)
      unify_ty(declared.resok, actual.resok, sub, names)
    end
    return
  end
  if is_fn(declared) then
    if is_fn(actual) then
      if #declared.fnp == #actual.fnp then
        for i = 1, #declared.fnp do
          unify_ty(declared.fnp[i], actual.fnp[i], sub, names)
        end
        unify_ty(declared.fnr, actual.fnr, sub, names)
      end
    end
    return
  end
end

function has_unbound(ty, sub, names)
  if ty == nil then
    return false
  end
  if is_named(ty) and names[ty.named] then
    return sub[ty.named] == nil
  end
  if is_tup(ty) then
    for i = 1, #ty.tup do
      if has_unbound(ty.tup[i], sub, names) then
        return true
      end
    end
    return false
  end
  if is_vec(ty) then
    return has_unbound(ty.vec, sub, names)
  end
  if is_opt(ty) then
    return has_unbound(ty.opt, sub, names)
  end
  if is_res(ty) then
    return has_unbound(ty.reserr, sub, names) or has_unbound(ty.resok, sub, names)
  end
  if is_fn(ty) then
    for i = 1, #ty.fnp do
      if has_unbound(ty.fnp[i], sub, names) then
        return true
      end
    end
    return has_unbound(ty.fnr, sub, names)
  end
  return false
end

function resolve_generic(fn, args, line, col)
  local sub = {}
  local names = {}
  for i = 1, #fn.params do
    if fn.params[i].is_comptime then
      names[fn.params[i].name] = true
    end
  end
  local ai = 1
  for i = 1, #fn.params do
    local p = fn.params[i]
    if p.is_comptime then
      if ai <= #args and is_tyarg_expr(args[ai], line, col) then
        sub[p.name] = tyarg_value(args[ai], line, col)
        ai = ai + 1
      end
    end
  end
  -- infer the rest from the value arguments' known types against the
  -- declared parameter types; a closure argument with unbound names
  -- in its declared type gets its TAIL probed (a body read like
  -- `user.score` supplies `U`).
  local vi = ai
  for i = 1, #fn.params do
    local p = fn.params[i]
    if not p.is_comptime then
      if vi <= #args then
        local a = args[vi]
        if a.lam ~= nil and is_fn(p.ty) and has_unbound(p.ty, sub, names) then
          local decl_sig = subst_ast(p.ty, sub)
          local lam = a.lam
          local bound = true
          scope_push()
          for li = 1, #lam.params do
            local lp = lam.params[li]
            local lty = lp.ty
            if lty == nil then
              if li <= #decl_sig.fnp then
                lty = decl_sig.fnp[li]
              else
                bound = false
              end
            end
            if lty ~= nil and E.tvars ~= nil then
              lty = subst_ast(lty, E.tvars)
            end
            if lty == nil then
              bound = false
            else
              declare(lp.name, { kind = "ssa", ty = lty, id = "%" .. lp.name }, line, col)
            end
          end
          local pret = nil
          if bound and lam.body.tail ~= nil then
            pret = infer_expr_ty(lam.body.tail, line, col)
          end
          scope_pop()
          if pret ~= nil then
            unify_ty(p.ty, { fnp = decl_sig.fnp, fnr = pret }, sub, names)
          end
        else
          local actual = infer_expr_ty(a, line, col)
          if actual ~= nil then
            unify_ty(p.ty, actual, sub, names)
          end
        end
      end
      vi = vi + 1
    end
  end
  for i = 1, #fn.params do
    local p = fn.params[i]
    if p.is_comptime and sub[p.name] == nil then
      fail(line, col, "cannot infer the type argument for `" .. p.name .. "` of `" .. fn.name .. "` (pass it explicitly, D-113)")
    end
  end
  local key = fn.name
  for i = 1, #fn.params do
    if fn.params[i].is_comptime then
      key = key .. "__" .. mangle_ty(sub[fn.params[i].name])
    end
  end
  E.insts = E.insts or {}
  if E.insts[key] == nil then
    local copy = subst_ast(fn, sub)
    copy.sym = key
    copy.file = fn.file
    copy.is_entry = false
    local vps = {}
    for i = 1, #copy.params do
      if not copy.params[i].is_comptime then
        vps[#vps + 1] = copy.params[i]
      end
    end
    copy.params = vps
    copy.tvars = sub
    E.insts[key] = copy
    E.fns_by_sym[key] = copy
    E.pending_fns = E.pending_fns or {}
    E.pending_fns[#E.pending_fns + 1] = copy
  end
  local inst = E.insts[key]
  local val_args = {}
  for j = ai, #args do
    val_args[#val_args + 1] = args[j]
  end
  return inst, val_args
end

function emit_fn_call(fn, sym, args, line, col)
  if #args ~= #fn.params then
    fail(line, col, "`" .. sym .. "` expects " .. tostring(#fn.params) .. " argument(s), got " .. tostring(#args))
  end
  local parts = "%env"
  local types = "!frk_closure.envref"
  for i = 1, #args do
    local p = fn.params[i]
    if p.mut then
      -- D-112's exclusivity rules: the argument is a mutable binding
      -- (var local or mut parameter), passed as its box; one binding
      -- cannot occupy two mut positions in a call.
      local an = args[i]
      if an.ref == nil or an.ref.mod ~= nil then
        fail(an.line, an.col, "mut arguments must be names (a variable or a mut parameter)")
      end
      local b = lookup(an.ref.name)
      if b == nil then
        fail(an.line, an.col, "unknown name `" .. an.ref.name .. "`")
      end
      if b.kind ~= "box" then
        fail(an.line, an.col, "`" .. an.ref.name .. "` is not mutable (declare it with var, or take it as a mut parameter)")
      end
      local k = 1
      while k < i do
        if fn.params[k].mut and args[k].ref ~= nil and args[k].ref.name == an.ref.name then
          fail(an.line, an.col, "`" .. an.ref.name .. "` cannot occupy two mut positions in one call")
        end
        k = k + 1
      end
      parts = parts .. ", " .. b.id
      types = types .. ", " .. box_ir(p.ty, line, col)
    else
      local v = emit_expr_typed(args[i], p.ty)
      parts = parts .. ", " .. v
      types = types .. ", " .. value_ir(p.ty, line, col)
    end
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

-- A value-position block's trailing `if ... else ...` statement IS
-- its value (recursively - the fn-tail convention generalised to
-- match arms and if branches).
local function emit_value_block(blk)
  if blk.stmts ~= nil and blk.tail == nil and #blk.stmts > 0 then
    local last = blk.stmts[#blk.stmts]
    if last.ifstmt ~= nil and last.ifstmt.els ~= nil then
      local upto = #blk.stmts - 1
      for si = 1, upto do
        if not E.terminated then
          E.emit_stmt(blk.stmts[si])
        end
      end
      if E.terminated then
        return nil
      end
      return EMIT_IF_EXPR(last.ifstmt)
    end
  end
  return emit_block_expr(blk)
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
  local tv = emit_value_block(node.yes)
  local then_end = E.cur
  local then_term = E.terminated
  new_block(else_label)
  local ev = nil
  if node.els ~= nil then
    ev = emit_value_block(node.els)
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
  if tv ~= nil and ev ~= nil and not same_type(E.vtypes[tv], E.vtypes[ev]) then
    fail(line, col, "if arm types differ (`" .. tostring(E.vtypes[tv]) .. "` vs `" .. tostring(E.vtypes[ev]) .. "`)")
  end
  new_block(join_label)
  local join_arg = nil
  if ty ~= nil then
    -- the join arg is named from the join BLOCK itself: nested
    -- constructs advance the label counter, so reading it here would
    -- alias an inner join's arg.
    join_arg = "%j" .. string.sub(join_label, 2)
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

-- Short-circuiting and/or (SPEC 5.1): the right operand runs only on
-- the deciding edge; the join carries the deciding value (block args).
local function emit_shortcircuit(op, lnode, rnode, line, col)
  local a = emit_expr(lnode)
  if E.vtypes[a] ~= "bool" then
    fail(line, col, "`" .. op .. "` takes bool operands (SPEC 5.1)")
  end
  local rhs_label = fresh_label()
  local dec_label = fresh_label()
  local join_label = fresh_label()
  if op == "and" then
    emit("cf.cond_br " .. a .. ", ^" .. rhs_label .. ", ^" .. dec_label)
  else
    emit("cf.cond_br " .. a .. ", ^" .. dec_label .. ", ^" .. rhs_label)
  end
  new_block(rhs_label)
  local b = emit_expr(rnode)
  if E.vtypes[b] ~= "bool" then
    fail(line, col, "`" .. op .. "` takes bool operands (SPEC 5.1)")
  end
  terminate("cf.br ^" .. join_label .. "(" .. b .. " : i1)")
  new_block(dec_label)
  terminate("cf.br ^" .. join_label .. "(" .. a .. " : i1)")
  new_block(join_label)
  local jarg = "%j" .. string.sub(join_label, 2)
  E.blocks[#E.blocks].label = join_label .. "(" .. jarg .. ": i1)"
  E.vtypes[jarg] = "bool"
  return jarg
end

-- `try e` (SPEC 6.5): e must be an error union (variant 0 Ok with a
-- payload, variant 1 Err with a payload); on Err the ENCLOSING
-- function returns Err(x) (the same E: the return type must be an
-- error union whose Err payload matches structurally); on Ok the
-- expression yields the payload.
local function emit_try(node)
  local line = node.line or 1
  local col = node.col or 1
  local v = emit_expr(node.tryex)
  local oty = E.vtypes[v]
  if not (is_named(oty) or is_opt(oty) or is_res(oty)) then
    fail(line, col, "`try` needs an error-union operand")
  end
  local od = decl_of(oty, line, col)
  if od.kind ~= "union" or #od.variants ~= 2 then
    fail(line, col, "`try` needs an error union (Ok + Err variants)")
  end
  local v0 = od.variants[1]
  local v1 = od.variants[2]
  if v0.name ~= "Ok" or v1.name ~= "Err" or v0.ty == nil or v1.ty == nil then
    fail(line, col, "`try` needs an error union (Ok + Err variants)")
  end
  local rty = E.ret
  if not (is_named(rty) or is_res(rty)) then
    fail(line, col, "the enclosing function must return an error union for `try`")
  end
  local rd = decl_of(rty, line, col)
  if rd.kind ~= "union" or #rd.variants ~= 2 or rd.variants[2].name ~= "Err" or rd.variants[2].ty == nil then
    fail(line, col, "the enclosing function must return an error union for `try`")
  end
  if not same_type(rd.variants[2].ty, v1.ty) then
    fail(line, col, "`try` propagates a different error type (use result.mapError)")
  end
  local tag = fresh()
  emit(tag .. " = \"frk_adt.tag_of\"(" .. v .. ") : (" .. value_ir(oty, line, col) .. ") -> i64")
  local zero_t = const_i64("0")
  local is_ok = fresh()
  emit(is_ok .. " = arith.cmpi eq, " .. tag .. ", " .. zero_t .. " : i64")
  local okb = fresh_label()
  local errb = fresh_label()
  emit("cf.cond_br " .. is_ok .. ", ^" .. okb .. ", ^" .. errb)
  new_block(okb)
  local stored = fresh()
  emit(stored .. " = \"frk_adt.extract\"(" .. v .. ") {variant = 0 : i64, field = 0 : i64} : (" .. value_ir(oty, line, col) .. ") -> " .. storage_ir(v0.ty, line, col))
  local pv = unwrap_load(stored, v0.ty, line, col)
  local join = fresh_label()
  local jarg = "%j" .. tostring(E.label)
  terminate("cf.br ^" .. join .. "(" .. pv .. " : " .. value_ir(v0.ty, line, col) .. ")")
  new_block(errb)
  local estored = fresh()
  emit(estored .. " = \"frk_adt.extract\"(" .. v .. ") {variant = 1 : i64, field = 0 : i64} : (" .. value_ir(oty, line, col) .. ") -> " .. storage_ir(v1.ty, line, col))
  local ev = unwrap_load(estored, v1.ty, line, col)
  local rw = wrap_store(ev, v1.ty, line, col)
  local e0 = fresh()
  emit(e0 .. " = \"frk_adt.product_new\"() : () -> !frk_adt.product<[]>")
  local sn = fresh()
  emit(sn .. " = \"frk_adt.product_snoc\"(" .. e0 .. ", " .. rw .. ") : (!frk_adt.product<[]>, " .. storage_ir(v1.ty, line, col) .. ") -> !frk_adt.product<[" .. storage_ir(v1.ty, line, col) .. "]>")
  local sm = fresh()
  emit(sm .. " = \"frk_adt.make_sum\"(" .. sn .. ") {variant = 1 : i64} : (!frk_adt.product<[" .. storage_ir(v1.ty, line, col) .. "]>) -> " .. value_ir(rty, line, col))
  terminate("func.return " .. sm .. " : " .. value_ir(rty, line, col))
  new_block(join)
  E.blocks[#E.blocks].label = join .. "(" .. jarg .. ": " .. value_ir(v0.ty, line, col) .. ")"
  E.vtypes[jarg] = v0.ty
  return jarg
end

-- `expr catch |e| { ... }` (SPEC 6.5, level 0): the Ok path unwraps;
-- the Err path runs the replacement block (the error bound as an
-- ordinary local), whose value joins as the unwrapped type; a
-- diverging block (the playbook's `catch { return .Err(...) }`) needs
-- no join arm.
function emit_catch(node)
  local line = node.line or 1
  local col = node.col or 1
  local v = emit_expr(node.catchx)
  local oty = E.vtypes[v]
  if not (is_named(oty) or is_res(oty)) then
    fail(line, col, "`catch` needs an error-union operand")
  end
  local od = decl_of(oty, line, col)
  if od.kind ~= "union" or #od.variants ~= 2 then
    fail(line, col, "`catch` needs an error union (Ok + Err variants)")
  end
  local v0 = od.variants[1]
  local v1 = od.variants[2]
  if v0.name ~= "Ok" or v1.name ~= "Err" or v0.ty == nil or v1.ty == nil then
    fail(line, col, "`catch` needs an error union (Ok + Err variants)")
  end
  local tag = fresh()
  emit(tag .. " = \"frk_adt.tag_of\"(" .. v .. ") : (" .. value_ir(oty, line, col) .. ") -> i64")
  local is_ok = fresh()
  emit(is_ok .. " = arith.cmpi eq, " .. tag .. ", " .. const_i64("0") .. " : i64")
  local okb = fresh_label()
  local errb = fresh_label()
  emit("cf.cond_br " .. is_ok .. ", ^" .. okb .. ", ^" .. errb)
  new_block(okb)
  local stored = fresh()
  emit(stored .. " = \"frk_adt.extract\"(" .. v .. ") {variant = 0 : i64, field = 0 : i64} : (" .. value_ir(oty, line, col) .. ") -> " .. storage_ir(v0.ty, line, col))
  local pv = unwrap_load(stored, v0.ty, line, col)
  local join = fresh_label()
  local jarg = "%j" .. tostring(E.label)
  terminate("cf.br ^" .. join .. "(" .. pv .. " : " .. value_ir(v0.ty, line, col) .. ")")
  new_block(errb)
  local estored = fresh()
  emit(estored .. " = \"frk_adt.extract\"(" .. v .. ") {variant = 1 : i64, field = 0 : i64} : (" .. value_ir(oty, line, col) .. ") -> " .. storage_ir(v1.ty, line, col))
  local ev = unwrap_load(estored, v1.ty, line, col)
  scope_push()
  if node.bind ~= nil then
    declare(node.bind, { kind = "ssa", ty = v1.ty, id = ev }, line, col)
    E.vtypes[ev] = v1.ty
  end
  local bv = emit_block_expr(node.block)
  scope_pop()
  if not E.terminated then
    if bv == nil then
      fail(line, col, "the catch block must produce a value or diverge")
    end
    if not same_type(E.vtypes[bv], v0.ty) then
      fail(line, col, "the catch block yields a different type than the union's Ok payload")
    end
    terminate("cf.br ^" .. join .. "(" .. bv .. " : " .. value_ir(v0.ty, line, col) .. ")")
  end
  new_block(join)
  E.blocks[#E.blocks].label = join .. "(" .. jarg .. ": " .. value_ir(v0.ty, line, col) .. ")"
  E.vtypes[jarg] = v0.ty
  return jarg
end

emit_tuple = function(node, expected)
  local items = node.tuple
  local line = node.line or 1
  local col = node.col or 1
  local acc = fresh()
  emit(acc .. " = \"frk_adt.product_new\"() : () -> !frk_adt.product<[]>")
  local acc_ir = "!frk_adt.product<[]>"
  local el_irs = {}
  local el_tys = {}
  for i = 1, #items do
    local want = nil
    if expected ~= nil and is_tup(expected) and i <= #expected.tup then
      want = expected.tup[i]
    end
    local v = nil
    if want ~= nil then
      v = emit_expr_typed(items[i], want)
    else
      v = emit_expr(items[i])
    end
    local vty = E.vtypes[v]
    if vty == nil then
      fail(items[i].line or line, items[i].col or col, "internal: a tuple element produced an untyped value")
    end
    el_tys[#el_tys + 1] = vty
    el_irs[#el_irs + 1] = storage_ir(vty, line, col)
    local stored = v
    if is_named(vty) then
      stored = wrap_store(v, vty, line, col)
    end
    local nv = fresh()
    emit(nv .. " = \"frk_adt.product_snoc\"(" .. acc .. ", " .. stored .. ") : (" .. acc_ir .. ", " .. storage_ir(vty, line, col) .. ") -> !frk_adt.product<[" .. join_strs(el_irs, ", ") .. "]>")
    acc = nv
    acc_ir = "!frk_adt.product<[" .. join_strs(el_irs, ", ") .. "]>"
  end
  local ty = { tup = el_tys }
  E.vtypes[acc] = ty
  return acc, ty
end

emit_expr = function(node)
  local line = node.line or 1
  local col = node.col or 1
  if node.tuple ~= nil then
    local v2 = emit_tuple(node, nil)
    return v2
  end
  if node.list ~= nil then
    fail(node.line, node.col, "a collection literal needs a [T]/Vector annotation (bind it: `const v: [i64] = [...]`)")
  end
  if node.varcon ~= nil then
    fail(node.line, node.col, "`.Variant` needs a union-typed context (annotate the binding or return a union)")
  end
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
        local uty = named_ty(td.name)
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
      if is_tup(owner.ty) then
        -- `t.0` / `t.1` (D-114): a structural product slot.
        local nidx = parse_slot(node.ref.name)
        if nidx == nil then
          fail(line, col, "a tuple's members are numeric (`t.0`); `" .. node.ref.name .. "` is not")
        end
        if nidx < 0 or nidx >= #owner.ty.tup then
          fail(line, col, "tuple index out of range: " .. node.ref.name)
        end
        local slot_ty = owner.ty.tup[nidx + 1]
        local value = read_binding(owner, line, col)
        local stored = fresh()
        emit(stored .. " = \"frk_adt.get\"(" .. value .. ") {field = " .. tostring(nidx) .. " : i64} : (" .. value_ir(owner.ty, line, col) .. ") -> " .. storage_ir(slot_ty, line, col))
        local out = unwrap_load(stored, slot_ty, line, col)
        E.vtypes[out] = slot_ty
        return out
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
      local f2 = E.fns[node.ref.name]
      if f2 ~= nil then
        if is_generic_fn(f2) then
          fail(line, col, "a generic fn value needs explicit instantiation (D-113)")
        end
        local e0 = fresh()
        emit(e0 .. " = \"frk_adt.product_new\"() : () -> !frk_adt.product<[]>")
        local fty = { fnp = {}, fnr = f2.ret }
        for i = 1, #f2.params do
          fty.fnp[#fty.fnp + 1] = f2.params[i].ty
        end
        local cl = fresh()
        emit(cl .. " = \"frk_closure.make\"(" .. e0 .. ") {callee = @" .. f2.sym .. "} : (!frk_adt.product<[]>) -> " .. value_ir(fty, line, col))
        E.vtypes[cl] = fty
        return cl
      end
      fail(line, col, "unknown name `" .. node.ref.name .. "`")
    end
    if b.kind == "ssa" then
      E.vtypes[b.id] = b.ty
      return b.id
    end
    local v, rty = read_binding(b, line, col)
    E.vtypes[v] = rty
    return v
  end
  if node.lam ~= nil then
    local v, ty = emit_lambda(node)
    E.vtypes[v] = ty
    return v
  end
  if node.tryex ~= nil then
    return emit_try(node)
  end
  if node.catchx ~= nil then
    return emit_catch(node)
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
    if op == "and" or op == "or" then
      return emit_shortcircuit(op, node.l, node.r, node.line, node.col)
    end
    local a = emit_expr(node.l)
    local b = emit_expr(node.r)
    if op == "==" or op == "!=" or op == "<" or op == "<=" or op == ">" or op == ">=" then
      local v = emit_cmp(op, a, b, node.line, node.col)
      E.vtypes[v] = "bool"
      return v
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
      v = emit_expr_typed(c.expr, c.ty)
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
  if node.tdestr ~= nil then
    local d = node.tdestr
    local v = emit_expr(d.expr)
    local vty = E.vtypes[v]
    if not is_tup(vty) then
      fail(d.line, d.col, "tuple destructuring needs a tuple-typed initializer")
    end
    if #d.names ~= #vty.tup then
      fail(d.line, d.col, "tuple destructuring arity: " .. tostring(#d.names) .. " name(s) for a " .. tostring(#vty.tup) .. "-tuple")
    end
    local j = 1
    while j < #d.names do
      local k = j + 1
      while k <= #d.names do
        if d.names[j].name == d.names[k].name then
          fail(d.names[k].line, d.names[k].col, "duplicate name `" .. d.names[k].name .. "` in destructuring")
        end
        k = k + 1
      end
      j = j + 1
    end
    for i = 1, #d.names do
      local nm = d.names[i]
      local slot_ty = vty.tup[i]
      local stored = fresh()
      emit(stored .. " = \"frk_adt.get\"(" .. v .. ") {field = " .. tostring(i - 1) .. " : i64} : (" .. value_ir(vty, d.line, d.col) .. ") -> " .. storage_ir(slot_ty, d.line, d.col))
      local value = unwrap_load(stored, slot_ty, d.line, d.col)
      if d.mutable then
        local b = fresh()
        emit(b .. " = \"frk_mem.box_new\"(" .. value .. ") : (" .. value_ir(slot_ty, d.line, d.col) .. ") -> " .. box_ir(slot_ty, d.line, d.col))
        declare(nm.name, { kind = "box", ty = slot_ty, id = b, box = box_ir(slot_ty, nm.line, nm.col) }, nm.line, nm.col)
      else
        declare(nm.name, { kind = "ssa", ty = slot_ty, id = value }, nm.line, nm.col)
      end
    end
    return
  end
  if node.destr ~= nil then
    local d = node.destr
    local v = emit_expr(d.expr)
    local bty = E.vtypes[v]
    if bty == nil or is_named(bty) == false then
      fail(d.line, d.col, "destructuring needs a struct-typed initializer (seed v0)")
    end
    local decl = find_type(bty.named, d.line, d.col)
    if decl.kind ~= "struct" then
      fail(d.line, d.col, "`" .. bty.named .. "` is a union; destructure with `match`")
    end
    for i = 1, #d.names do
      local nm = d.names[i]
      local j = 1
      while j < i do
        if d.names[j].name == nm.name then
          fail(nm.line, nm.col, "duplicate name `" .. nm.name .. "` in destructuring")
        end
        j = j + 1
      end
      local idx = find_field(decl, nm.field, nm.line, nm.col)
      local fld = decl.fields[idx]
      local stored = fresh()
      emit(stored .. " = \"frk_mem.field_get\"(" .. v .. ") {field = " .. tostring(idx - 1) .. " : i64} : (" .. value_ir(bty, d.line, d.col) .. ") -> " .. storage_ir(fld.ty, fld.line, fld.col))
      local value = unwrap_load(stored, fld.ty, fld.line, fld.col)
      if d.mutable then
        local b = fresh()
        emit(b .. " = \"frk_mem.box_new\"(" .. value .. ") : (" .. value_ir(fld.ty, fld.line, fld.col) .. ") -> " .. box_ir(fld.ty, fld.line, fld.col))
        declare(nm.name, { kind = "box", ty = fld.ty, id = b, box = box_ir(fld.ty, fld.line, fld.col) }, nm.line, nm.col)
      else
        E.vtypes[value] = fld.ty
        declare(nm.name, { kind = "ssa", ty = fld.ty, id = value }, nm.line, nm.col)
      end
    end
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
      v = emit_expr_typed(d.expr, d.ty)
    end
    local dty = d.ty
    if dty == nil then
      dty = E.vtypes[v]
      if dty == nil then
        fail(d.line, d.col, "internal: untyped var initializer")
      end
    end
    local b = fresh()
    emit(b .. " = \"frk_mem.box_new\"(" .. v .. ") : (" .. value_ir(dty, d.line, d.col) .. ") -> " .. box_ir(dty, d.line, d.col))
    declare(d.name, { kind = "box", ty = dty, id = b, box = box_ir(dty, d.line, d.col) }, d.line, d.col)
    return
  end
  if node.fassign ~= nil then
    local fa = node.fassign
    local b = lookup(fa.name)
    if b == nil then
      fail(fa.line, fa.col, "unknown name `" .. fa.name .. "` in field assignment")
    end
    local bcap = b.kind == "cap" and b.cap_kind == "box"
    if b.kind ~= "box" and not bcap then
      fail(fa.line, fa.col, "cannot set a field through immutable `" .. fa.name .. "` (use a var binding or a mut parameter)")
    end
    if not is_named(b.ty) then
      fail(fa.line, fa.col, "field assignment needs a record binding (seed v0)")
    end
    local drec = find_type(b.ty.named, fa.line, fa.col)
    if drec.kind ~= "struct" then
      fail(fa.line, fa.col, "`" .. b.ty.named .. "` is a union; assign through `match`")
    end
    local idx = find_field(drec, fa.field, fa.line, fa.col)
    local fld = drec.fields[idx]
    local v = emit_expr_typed(fa.expr, fld.ty)
    if not same_type(E.vtypes[v], fld.ty) then
      fail(fa.line, fa.col, "field `" .. fa.field .. "` does not take this value's type")
    end
    local rec = fresh()
    emit(rec .. " = \"frk_mem.box_get\"(" .. binding_slot(b, fa.line, fa.col) .. ") : (" .. box_ir(b.ty, fa.line, fa.col) .. ") -> " .. value_ir(b.ty, fa.line, fa.col))
    local stored = v
    if is_named(fld.ty) then
      stored, _ = wrap_store(v, fld.ty, fa.line, fa.col)
    end
    emit("\"frk_mem.field_set\"(" .. rec .. ", " .. stored .. ") {field = " .. tostring(idx - 1) .. " : i64} : (" .. value_ir(b.ty, fa.line, fa.col) .. ", " .. storage_ir(fld.ty, fa.line, fa.col) .. ") -> ()")
    return
  end
  if node.assign ~= nil then
    local a = node.assign
    local b = lookup(a.name)
    if b == nil then
      fail(a.line, a.col, "unknown name `" .. a.name .. "` in assignment")
    end
    local bcap = b.kind == "cap" and b.cap_kind == "box"
    if b.kind ~= "box" and not bcap then
      fail(a.line, a.col, "cannot assign to immutable `" .. a.name .. "`")
    end
    local bid = binding_slot(b, a.line, a.col)
    local bty = b.box
    if b.kind == "cap" then
      bty = b.cap_box
    end
    local v = emit_expr_typed(a.expr, b.ty)
    emit("\"frk_mem.box_set\"(" .. bid .. ", " .. v .. ") : (" .. bty .. ", " .. value_ir(b.ty, a.line, a.col) .. ") -> ()")
    return
  end
  if node.ret ~= nil then
    if node.ret.expr ~= nil then
      local v = emit_expr_typed(node.ret.expr, E.ret)
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

local function emit_fn_core(fn)
  local params = "%env: !frk_closure.envref"
  scope_push()
  for i = 1, #fn.params do
    local p = fn.params[i]
    if p.name == "env" then
      fail(p.line, p.col, "`env` is reserved in the seed's parameter namespace")
    end
    if p.mut then
      -- D-112: a mut parameter is the value's box passed by handle;
      -- reads box_get, field writes field_set, caller-visible.
      params = params .. ", %" .. p.name .. ": " .. box_ir(p.ty, p.line, p.col)
      declare(p.name, { kind = "box", ty = p.ty, id = "%" .. p.name, box = box_ir(p.ty, p.line, p.col) }, p.line, p.col)
    else
      params = params .. ", %" .. p.name .. ": " .. value_ir(p.ty, p.line, p.col)
      declare(p.name, { kind = "ssa", ty = p.ty, id = "%" .. p.name }, p.line, p.col)
    end
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
      if not E.terminated then
        if fn.ret == "unit" then
          -- A unit fn's tail ran for its effects; the return is bare.
          terminate("func.return")
        elseif v == nil then
          fail(body.tail.line or 1, body.tail.col or 1, "internal: valueless tail in a value-returning function")
        else
          terminate("func.return " .. v .. " : " .. value_ir(fn.ret, fn.line, fn.col))
        end
      end
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
  emit_fn_core(fn)
end

-- ------------------------------------------------ closures (SPEC 6.4)

-- The lift context save/restore (a lambda emits its body twice: a
-- discovery pass that records the captures, then the real pass with
-- the env's product type known).
local function lift_snapshot()
  return {
    out = E.out, tmp = E.tmp, label = E.label, blocks = E.blocks,
    cur = E.cur, terminated = E.terminated, vtypes = E.vtypes,
    ret = E.ret, capbar = E.capbar, caps = E.caps, capenv = E.capenv,
    capenv_ty = E.capenv_ty, depth = #E.scopes,
  }
end

local function lift_restore(s)
  E.out = s.out
  E.tmp = s.tmp
  E.label = s.label
  E.blocks = s.blocks
  E.cur = s.cur
  E.terminated = s.terminated
  E.vtypes = s.vtypes
  E.ret = s.ret
  E.capbar = s.capbar
  E.caps = s.caps
  E.capenv = s.capenv
  E.capenv_ty = s.capenv_ty
  while #E.scopes > s.depth do
    E.scopes[#E.scopes] = nil
  end
end

local function capenv_ir(caps)
  local parts = {}
  for i = 1, #caps do
    parts[#parts + 1] = caps[i].proxy.cap_storage
  end
  return "!frk_adt.product<[" .. join_parts(parts) .. "]>"
end

local function lift_prepare(flike, env_ty)
  E.out = {}
  E.tmp = 0
  E.label = 0
  E.blocks = {}
  E.cur = 0
  E.terminated = false
  E.vtypes = {}
  E.fns = FNS
  E.fns_by_sym = FNS_BY_SYM
  E.ret = flike.ret
  E.caps = {}
  E.capenv = nil
  E.capenv_ty = env_ty
  E.capbar = #E.scopes + 1
  new_block(nil)
end

emit_lambda = function(node)
  local l = node.lam
  for i = 1, #l.params do
    if l.params[i].ty == nil then
      fail(l.params[i].line or node.line or 1, l.params[i].col or node.col or 1, "cannot infer this closure's parameter types; annotate them, or place the closure where a fn type is expected")
    end
  end
  if l.ret == nil then
    fail(node.line or 1, node.col or 1, "cannot infer this closure's return type; annotate it, or place the closure where a fn type is expected")
  end
  local n = E.lam or 0
  E.lam = n + 1
  local sym = "__calque_lam_" .. tostring(n)
  local flike = {
    sym = sym, params = l.params, ret = l.ret, body = l.body,
    file = CURRENT_FILE, line = node.line or 1, col = node.col or 1,
  }
  -- PASS 1: discovery (the emission into a discarded buffer records
  -- the captures in first-encounter order).
  local s1 = lift_snapshot()
  lift_prepare(flike, "!frk_adt.product<[]>")
  emit_fn_core(flike)
  local caps = E.caps
  lift_restore(s1)
  local env_ty = capenv_ir(caps)
  -- PASS 2: the real lift, with the env's product type known.
  local s2 = lift_snapshot()
  lift_prepare(flike, env_ty)
  emit_fn_core(flike)
  local lifted = E.out
  lift_restore(s2)
  E.pending = E.pending or {}
  E.pending[#E.pending + 1] = lifted
  -- The site: the capture env from the OUTER bindings, then `make`.
  local ln = node.line or 1
  local lc = node.col or 1
  local e0 = fresh()
  emit(e0 .. " = \"frk_adt.product_new\"() : () -> !frk_adt.product<[]>")
  local acc = e0
  local acc_ty = "!frk_adt.product<[]>"
  local acc_fields = {}
  for i = 1, #caps do
    local cap = caps[i]
    local slot = binding_slot(cap.orig, ln, lc)
    local st = cap.proxy.cap_storage
    acc_fields[#acc_fields + 1] = st
    local nxt = fresh()
    emit(nxt .. " = \"frk_adt.product_snoc\"(" .. acc .. ", " .. slot .. ") : (" .. acc_ty .. ", " .. st .. ") -> !frk_adt.product<[" .. join_parts(acc_fields) .. "]>")
    acc = nxt
    acc_ty = "!frk_adt.product<[" .. join_parts(acc_fields) .. "]>"
  end
  local fnty = { fnp = {}, fnr = l.ret }
  for i = 1, #l.params do
    fnty.fnp[i] = l.params[i].ty
  end
  local cl = fresh()
  emit(cl .. " = \"frk_closure.make\"(" .. acc .. ") {callee = @" .. sym .. "} : (" .. acc_ty .. ") -> " .. fn_ir(fnty, ln, lc))
  return cl, fnty
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
      elseif not is_generic_fn(fn) then
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
  local pi = 1
  local fi = 1
  while true do
    local moved = false
    while E.pending ~= nil and pi <= #E.pending do
      local q = E.pending[pi]
      pi = pi + 1
      for i = 1, #q do
        E.out[#E.out + 1] = q[i]
      end
      moved = true
    end
    while E.pending_fns ~= nil and fi <= #E.pending_fns do
      local q2 = E.pending_fns[fi]
      fi = fi + 1
      local saved = E.tvars
      E.tvars = q2.tvars
      emit_fn(q2)
      E.tvars = saved
      moved = true
    end
    if not moved then
      break
    end
  end
  for i = 1, #E.out do
    print(E.out[i])
  end
end

-- forward hook: the match machinery's arm-value lift calls the if
-- expression emitter through this alias (declared later as a local).
EMIT_IF_EXPR = emit_if_expr
EMIT_VALUE_BLOCK = emit_value_block
