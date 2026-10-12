#!/bin/sh
# calque charter + acceptance-matrix checker (M36/C0, D-085; law L1:
# the acceptance matrix is the artifact, this is its verifier).
#
# Validates STRUCTURE, not prose:
#   1. the required Calque artifacts exist;
#   2. docs/calque/GOAL.md matches its pin (the goal is the contract:
#      changing it is deliberate and updates the pin in the same commit);
#   3. the acceptance matrix is well-formed: unique AC-IDs, legal status
#      keywords, every goal section §1..§23 covered;
#   4. `done` rows point at evidence that EXISTS in this tree: any
#      backticked path-like token (contains "/", no glob/brace/placeholder)
#      must resolve as a repo-relative path, and any backticked
#      `make <target>` must name a target in the Makefile;
#   5. the frozen b0 examples are ASCII (SPEC F18);
#   6. the `calque-test` target and the book chapter are wired.
set -eu
cd "$(dirname "$0")/.."

CALQUE_GOAL_SHA=09c9ee8ddb7134fd222e580a908ff7d8e3ff7017a7ceb92a0433576659feff30
# The steering amendment (D-095..D-097) is a permanent record too; its
# pin moves only with a deliberate amendment edit (never to accommodate
# new requirements — the goal pin above is likewise untouchable).
CALQUE_STEERING_SHA=87b4fe7c308830d41254020fe4086a7188ee0ffffd8128a1b9bc9834e6bb24a8
# The second steering amendment (D-099..D-102: docs, packages, fuzzing)
# is a permanent record too; same pin discipline.
CALQUE_STEERING2_SHA=a372428e51df80c51fe8f39c57c89cfd327b2bfb65cc0aab61618b303c1a64bc
# The third steering amendment (D-103: the language server) — same pin
# discipline.
CALQUE_STEERING3_SHA=5fdaa2c124c0b66be18f57561c97e8ac976bf29533eb709ff6decfc57dc9c7dd
# The fourth steering amendment (D-104: diagnostics, lint, agent help)
# - same pin discipline.
CALQUE_STEERING4_SHA=952d8860fc8e8c8abd7382f5eb887d2a335549f64da8ceb1ebb8ee01792843bf
# The fifth steering amendment (D-105: concurrency, modern CPUs) -
# same pin discipline.
CALQUE_STEERING5_SHA=7ffc46a95565ef563c70b63c010da9efc8d0acda666ad91ec1e8bcfec75a63ee
# The sixth steering amendment (D-108: vector/array/SIMD syntax) and the
# seventh (D-109: Silat Style) - same pin discipline.
CALQUE_STEERING6_SHA=5e422a6b8c23c5063f29b9e0fdf919d21ad331e01ad5b00103756192c9b272fc
CALQUE_SILAT_SHA=9b317c01c3d0d18c7b56b9f62f5367c50dc6f9c5a99f08a2d993c1bafa28140f
# The eighth steering amendment (D-120: modules, companion functions,
# collection access) - same pin discipline.
CALQUE_STEERING8_SHA=99c9b939277f0c30b4b4c701abd3df44d69072f0fe81ffe594bab04a065fd67b
# The FROZEN seed (D-119; BOOTSTRAP.md section 1.3): byte-pinned from
# the freeze point onwards. New language growth rides calq0 and the
# snapshot chain, never this file.
CALQUE_SEED_SHA=75206d8319d88442df0eb7fce92819a833a1eb4acc711f4b5f5a952bb1a78cd2

fail=0
err() { echo "calque-check: $1" >&2; fail=1; }

# 1. Required artifacts.
for f in docs/calque/GOAL.md docs/calque/SPEC.md docs/calque/BOOTSTRAP.md \
         docs/calque/ACCEPTANCE.md docs/calque/PERFORMANCE.md \
         docs/calque/STEERING-TESTS-STRUCTS-FORMATS.md \
         docs/calque/STEERING-DOCS-PACKAGES-FUZZING.md \
         docs/calque/STEERING-LSP.md \
         docs/calque/STEERING-DIAGNOSTICS-LINT-AGENT-HELP.md \
         docs/calque/STEERING-CONCURRENCY-MODERN-CPUS.md \
         docs/calque/STEERING-VECTOR-SUGAR.md \
         docs/calque/SILAT_STYLE.md \
         docs/calque-steering-modules-access.md \
         tools/calque/bootstrap/snapshots/c2v1/README.md \
         tools/calque/bootstrap/snapshots/c2v1/MANIFEST.sha256 \
         tools/calque/bootstrap/snapshots/c3v1/README.md \
         tools/calque/bootstrap/snapshots/c3v1/MANIFEST.sha256 \
         tools/calque/bootstrap/snapshots/c4v1/README.md \
         tools/calque/bootstrap/snapshots/c4v1/MANIFEST.sha256 \
         tools/calque/bootstrap/snapshots/c5v1/README.md \
         tools/calque/bootstrap/snapshots/c5v1/MANIFEST.sha256 \
         tools/calque/bootstrap/snapshots/c6v1/README.md \
         tools/calque/bootstrap/snapshots/c6v1/MANIFEST.sha256 \
         tools/calque/bootstrap/snapshots/c7v1/README.md \
         tools/calque/bootstrap/snapshots/c7v1/MANIFEST.sha256 \
         specimens/calque/MANIFEST.md \
         specimens/calque/examples/README.md book/src/specimens/calque.md; do
  [ -f "$f" ] || err "missing artifact: $f"
done
for e in active_score identity_pair fib squares; do
  [ -f "specimens/calque/examples/$e.calq" ] || err "missing example: $e.calq"
done

# 2. Goal pin (historical contract; NEVER weakened for new requirements).
if [ -f docs/calque/GOAL.md ]; then
  got=$(sha256sum docs/calque/GOAL.md | cut -d' ' -f1)
  [ "$got" = "$CALQUE_GOAL_SHA" ] \
    || err "GOAL.md sha256 drifted: $got (pin $CALQUE_GOAL_SHA)"
fi
# 2b. Steering amendment pin (D-095..D-097).
if [ -f docs/calque/STEERING-TESTS-STRUCTS-FORMATS.md ]; then
  got=$(sha256sum docs/calque/STEERING-TESTS-STRUCTS-FORMATS.md | cut -d' ' -f1)
  [ "$got" = "$CALQUE_STEERING_SHA" ] \
    || err "STEERING-TESTS-STRUCTS-FORMATS.md sha256 drifted: $got (pin $CALQUE_STEERING_SHA)"
fi
# 2c. Second steering amendment pin (D-099..D-102).
if [ -f docs/calque/STEERING-DOCS-PACKAGES-FUZZING.md ]; then
  got=$(sha256sum docs/calque/STEERING-DOCS-PACKAGES-FUZZING.md | cut -d' ' -f1)
  [ "$got" = "$CALQUE_STEERING2_SHA" ] \
    || err "STEERING-DOCS-PACKAGES-FUZZING.md sha256 drifted: $got (pin $CALQUE_STEERING2_SHA)"
fi
# 2d. Third steering amendment pin (D-103).
if [ -f docs/calque/STEERING-LSP.md ]; then
  got=$(sha256sum docs/calque/STEERING-LSP.md | cut -d' ' -f1)
  [ "$got" = "$CALQUE_STEERING3_SHA" ] \
    || err "STEERING-LSP.md sha256 drifted: $got (pin $CALQUE_STEERING3_SHA)"
fi
# 2e. Fourth steering amendment pin (D-104).
if [ -f docs/calque/STEERING-DIAGNOSTICS-LINT-AGENT-HELP.md ]; then
  got=$(sha256sum docs/calque/STEERING-DIAGNOSTICS-LINT-AGENT-HELP.md | cut -d' ' -f1)
  [ "$got" = "$CALQUE_STEERING4_SHA" ] \
    || err "STEERING-DIAGNOSTICS-LINT-AGENT-HELP.md sha256 drifted: $got (pin $CALQUE_STEERING4_SHA)"
fi
# 2f. Fifth steering amendment pin (D-105).
if [ -f docs/calque/STEERING-CONCURRENCY-MODERN-CPUS.md ]; then
  got=$(sha256sum docs/calque/STEERING-CONCURRENCY-MODERN-CPUS.md | cut -d' ' -f1)
  [ "$got" = "$CALQUE_STEERING5_SHA" ] \
    || err "STEERING-CONCURRENCY-MODERN-CPUS.md sha256 drifted: $got (pin $CALQUE_STEERING5_SHA)"
fi
# 2g/2h. Sixth amendment + Silat Style pins (D-108/D-109).
if [ -f docs/calque/STEERING-VECTOR-SUGAR.md ]; then
  got=$(sha256sum docs/calque/STEERING-VECTOR-SUGAR.md | cut -d' ' -f1)
  [ "$got" = "$CALQUE_STEERING6_SHA" ] \
    || err "STEERING-VECTOR-SUGAR.md sha256 drifted: $got (pin $CALQUE_STEERING6_SHA)"
fi
if [ -f docs/calque/SILAT_STYLE.md ]; then
  got=$(sha256sum docs/calque/SILAT_STYLE.md | cut -d' ' -f1)
  [ "$got" = "$CALQUE_SILAT_SHA" ] \
    || err "SILAT_STYLE.md sha256 drifted: $got (pin $CALQUE_SILAT_SHA)"
fi
# 2i. Eighth amendment pin (D-120).
if [ -f docs/calque-steering-modules-access.md ]; then
  got=$(sha256sum docs/calque-steering-modules-access.md | cut -d' ' -f1)
  [ "$got" = "$CALQUE_STEERING8_SHA" ] \
    || err "calque-steering-modules-access.md sha256 drifted: $got (pin $CALQUE_STEERING8_SHA)"
fi
# 2j. The frozen seed + the c2v1 snapshot manifest (D-119).
if [ -f tools/calque/bootstrap/seed.lua ]; then
  got=$(sha256sum tools/calque/bootstrap/seed.lua | cut -d' ' -f1)
  [ "$got" = "$CALQUE_SEED_SHA" ] \
    || err "the frozen seed drifted: $got (pin $CALQUE_SEED_SHA; growth rides calq0 + snapshots, D-119)"
fi
if [ -d tools/calque/bootstrap/snapshots/c2v1 ]; then
  ( cd tools/calque/bootstrap/snapshots/c2v1 && sha256sum -c --status MANIFEST.sha256 ) \
    || err "the c2v1 snapshot drifted from MANIFEST.sha256 (snapshots are frozen, D-119)"
fi
if [ -d tools/calque/bootstrap/snapshots/c3v1 ]; then
  ( cd tools/calque/bootstrap/snapshots/c3v1 && sha256sum -c --status MANIFEST.sha256 ) \
    || err "the c3v1 snapshot drifted from MANIFEST.sha256 (snapshots are frozen, D-123)"
fi
if [ -d tools/calque/bootstrap/snapshots/c4v1 ]; then
  ( cd tools/calque/bootstrap/snapshots/c4v1 && sha256sum -c --status MANIFEST.sha256 ) \
    || err "the c4v1 snapshot drifted from MANIFEST.sha256 (snapshots are frozen, D-125)"
fi
if [ -d tools/calque/bootstrap/snapshots/c5v1 ]; then
  ( cd tools/calque/bootstrap/snapshots/c5v1 && sha256sum -c --status MANIFEST.sha256 ) \
    || err "the c5v1 snapshot drifted from MANIFEST.sha256 (snapshots are frozen, D-129)"
fi
if [ -d tools/calque/bootstrap/snapshots/c6v1 ]; then
  ( cd tools/calque/bootstrap/snapshots/c6v1 && sha256sum -c --status MANIFEST.sha256 ) \
    || err "the c6v1 snapshot drifted from MANIFEST.sha256 (snapshots are frozen, D-134)"
fi
if [ -d tools/calque/bootstrap/snapshots/c7v1 ]; then
  ( cd tools/calque/bootstrap/snapshots/c7v1 && sha256sum -c --status MANIFEST.sha256 ) \
    || err "the c7v1 snapshot drifted from MANIFEST.sha256 (snapshots are frozen, D-136)"
fi

# 3. Matrix structure.
m=docs/calque/ACCEPTANCE.md
if [ ! -f "$m" ]; then
  err "no acceptance matrix"
  exit 1
fi

ids=$(sed -E -n 's/^\| (AC-[0-9]+\.[0-9]+|AC-S[0-9]+) \|.*$/\1/p' "$m")
[ -n "$ids" ] || err "no AC rows parsed from $m"

dups=$(printf '%s\n' $ids | sort | uniq -d)
[ -z "$dups" ] || err "duplicate AC ids: $dups"

n=1
while [ "$n" -le 23 ]; do
  printf '%s\n' $ids | grep -q "^AC-$n\." || err "goal section §$n has no matrix row"
  n=$((n + 1))
done

# Statuses: first word must be one of the four legal keywords. Parse the
# LAST cell ($NF) so a stray pipe inside an earlier cell cannot shift it.
bad_status=$(awk -F'|' '/^\| AC-([0-9]+\.[0-9]+|S[0-9]+) / { s=$(NF-1); gsub(/^ +| +$/, "", s); split(s, w, " "); print w[1] }' "$m" \
  | sort -u | grep -vxE 'done|pending|ongoing|never' || true)
[ -z "$bad_status" ] || err "illegal status keyword(s): $bad_status"

# 4. `done` rows: evidence must resolve. (No pipes around the loops that
# call err: POSIX subshells would swallow the failure flag.)
m4=$(mktemp)
awk -F'|' '/^\| AC-([0-9]+\.[0-9]+|S[0-9]+) / { s=$(NF-1); gsub(/^ +| +$/, "", s); if (s ~ /^done/) print }' "$m" > "$m4"
while IFS= read -r row; do
  [ -n "$row" ] || continue
  tokens=$(printf '%s\n' "$row" | tr '`' '\n' | awk 'NR % 2 == 0')
  while IFS= read -r tok; do
    [ -n "$tok" ] || continue
    case "$tok" in
      make\ *)
        tgt=$(printf '%s\n' "$tok" | sed -n 's/^make \([A-Za-z0-9_-]*\).*$/\1/p')
        [ -z "$tgt" ] || grep -q "^$tgt:" Makefile || err "done-row target missing: make $tgt"
        ;;
      */*)
        case "$tok" in
          *'*'*|*'{'*|*'<'*) ;;  # globs/braces/placeholders: not checkable
          *)
            p=$(printf '%s\n' "$tok" | sed 's/[;,:]*$//')
            [ -e "$p" ] || err "done-row path missing: $p"
            ;;
        esac
        ;;
    esac
  done <<EOF
$tokens
EOF
done < "$m4"
ndone=$(grep -c '^|' "$m4" || true)
rm -f "$m4"

# 5. Frozen examples and the compiler sources are ASCII (SPEC F18:
# the seed input channel is ASCII-only - a stray UTF-8 byte dies there,
# loudly, at translation time; catch it in the charter pass first).
for e in specimens/calque/examples/*.calq tools/calque/compiler/*.calq tools/calque/std/*.calq; do
  if LC_ALL=C grep -q '[^ -~]' "$e"; then
    err "non-ASCII bytes in $e (SPEC F18)"
  fi
done

# 6. Wiring.
grep -q '^calque-test:' Makefile || err "Makefile is missing the calque-test target"
grep -q '^calque-selfhost:' Makefile || err "Makefile is missing the calque-selfhost target"
grep -q 'specimens/calque.md' book/src/SUMMARY.md || err "book SUMMARY.md is missing the calque chapter"

ntotal=$(printf '%s\n' $ids | grep -c '.' || true)
if [ "$fail" -eq 0 ]; then
  echo "calque-check: ok — artifacts present, matrix well-formed ($ndone/$ntotal rows done)"
else
  echo "calque-check: FAILED" >&2
  exit 1
fi
