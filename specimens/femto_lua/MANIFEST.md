# specimen: femto_lua — stub (ratify at M10)

Pin: Lua 5.1.5 (PUC-Rio C sources = readable spec; official test suite =
corpus; LuaJIT = perf yardstick, informational only).
Role: wakes the runtime — frk.dyn (tagging D-entry due at M10), strings
(Lua = 8-bit clean byte strings + UTF-8 convention), tables, metatable
dispatch (map onto itab plan or rule divergence), coroutines (frk.ctl),
incremental GC pressure → the M10 GC gate (rc+cycle-report vs MMTk spike).
Initial fences (to ratify): no `load`/`loadstring`, no full stdlib, no
weak tables v0, no `%` string-format exotica until canon rules exist.
Status: not started; MANIFEST ratification is an M10 exit item.
