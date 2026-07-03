# specimen: c_oracle — stub (rig, not a frontend)

Role: oracle infrastructure, early and parallel (D-009). Import path is
clang → LLVM bitcode (no C parser ever); uses: (1) per-target ABI/struct-
layout diffing against clang to keep every frontend's FFI honest across the
grid; (2) csmith + creduce differential rig feeding the harness; (3) a
corpus source for exercising the aot runner. Explicit non-goal: hosting C
as a language profile.
Status: not started; slot it when M7's grid exists to consume it.
