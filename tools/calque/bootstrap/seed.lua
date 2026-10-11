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
-- explicit types (i64/bool/Str/unit), const/var/assignment, if
-- (statement and expression), while, return, calls (unqualified and
-- the str/io preludes), the checked-arithmetic subset + - / %,
-- comparisons, string/int/bool literals, `pub`, comments. Everything
-- else fails LOUDLY (the seed never guesses).

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
      if two == "==" or two == "!=" or two == "<=" or two == ">=" or two == "|>" then
        push("punct", two, line, col)
        i = i + 2
        col = col + 2
      else
        local one = string.sub(source, i, i)
        if one == "(" or one == ")" or one == "{" or one == "}" or one == ","
          or one == ";" or one == ":" or one == "." or one == "=" or one == "<"
          or one == ">" or one == "+" or one == "-" or one == "*" or one == "/"
          or one == "%" or one == "!" then
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
  fail(t.line, t.col, "unsupported type (seed v0: i64, bool, Str, unit)")
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
      expect_punct(":")
      local ty = parse_type()
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
  while peek().k ~= "eof" do
    local t = peek()
    if t.k == "kw" and (t.v == "pub" or t.v == "fn") then
      fns[#fns + 1] = parse_fn()
    else
      fail(t.line, t.col, "seed v0 accepts only top-level fn declarations (got `" .. t.v .. "`)")
    end
  end
  return fns
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
  local mapped = TY_IR[ty]
  if mapped == nil then
    fail(line, col, "type `" .. ty .. "` has no IR mapping (seed v0)")
  end
  return mapped
end

local function poison(ty)
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
  fail(1, 1, "internal: no poison for " .. ty)
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
    terminate("func.return " .. poison(E.ret) .. " : " .. ir_ty(E.ret, line, col))
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

local function emit_call(node)
  local callee = node.call
  local line = node.line
  local col = node.col
  local args = node.args
  if callee.mod == "io" and callee.name == "print" then
    if #args ~= 1 then
      fail(line, col, "io.print takes exactly one Str (seed v0)")
    end
    local a = emit_expr(args[1])
    emit("func.call @frk_rt_calque_print(" .. a .. ") : (!frk_bstr.str) -> ()")
    return nil
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
    types = types .. ", " .. ir_ty(fn.params[i].ty, line, col)
  end
  if fn.ret == "unit" then
    emit("func.call @" .. sym .. "(" .. parts .. ") : (" .. types .. ") -> ()")
    return nil
  end
  local res = fresh()
  emit(res .. " = func.call @" .. sym .. "(" .. parts .. ") : (" .. types .. ") -> " .. ir_ty(fn.ret, line, col))
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
    E.blocks[#E.blocks].label = join_label .. "(" .. join_arg .. ": " .. ir_ty(ty, line, col) .. ")"
  end
  E.cur = then_end
  E.terminated = then_term
  if not E.terminated and tv ~= nil then
    terminate("cf.br ^" .. join_label .. "(" .. tv .. " : " .. ir_ty(ty, line, col) .. ")")
  elseif not E.terminated then
    terminate("cf.br ^" .. join_label)
  end
  E.cur = else_end
  E.terminated = else_term
  if not E.terminated and ev ~= nil then
    terminate("cf.br ^" .. join_label .. "(" .. ev .. " : " .. ir_ty(ty, line, col) .. ")")
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
    local b = lookup(node.ref.name)
    if b == nil then
      fail(line, col, "unknown name `" .. node.ref.name .. "`")
    end
    if b.kind == "ssa" then
      E.vtypes[b.id] = b.ty
      return b.id
    end
    local v = fresh()
    emit(v .. " = \"frk_mem.box_get\"(" .. b.id .. ") : (" .. b.box .. ") -> " .. ir_ty(b.ty, line, col))
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
    local v = emit_expr(c.expr)
    if v == nil then
      fail(c.line, c.col, "const initializer must be a value")
    end
    declare(c.name, { kind = "ssa", ty = c.ty, id = v }, c.line, c.col)
    return
  end
  if node.var ~= nil then
    local d = node.var
    local v = emit_expr(d.expr)
    local b = fresh()
    emit(b .. " = \"frk_mem.box_new\"(" .. v .. ") : (" .. ir_ty(d.ty, d.line, d.col) .. ") -> " .. TY_BOX[d.ty])
    declare(d.name, { kind = "box", ty = d.ty, id = b, box = TY_BOX[d.ty] }, d.line, d.col)
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
    emit("\"frk_mem.box_set\"(" .. b.id .. ", " .. v .. ") : (" .. b.box .. ", " .. ir_ty(b.ty, a.line, a.col) .. ") -> ()")
    return
  end
  if node.ret ~= nil then
    if node.ret.expr ~= nil then
      local v = emit_expr(node.ret.expr)
      terminate("func.return " .. v .. " : " .. ir_ty(E.ret, node.ret.line, node.ret.col))
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
    params = params .. ", %" .. p.name .. ": " .. ir_ty(p.ty, p.line, p.col)
    declare(p.name, { kind = "ssa", ty = p.ty, id = "%" .. p.name }, p.line, p.col)
  end
  local ret_ir = ""
  if fn.ret ~= "unit" then
    ret_ir = " -> " .. ir_ty(fn.ret, fn.line, fn.col)
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
      terminate("func.return " .. v .. " : " .. ir_ty(fn.ret, fn.line, fn.col))
    elseif body.tail ~= nil then
      local v = emit_expr(body.tail)
      terminate("func.return " .. v .. " : " .. ir_ty(fn.ret, fn.line, fn.col))
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
  for i = 1, #files do
    local f = files[i]
    CURRENT_FILE = f.name
    local fns = parse_file(f.text, f.name)
    parsed[#parsed + 1] = { file = f, fns = fns }
  end
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
