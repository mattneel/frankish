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

fail=0
err() { echo "calque-check: $1" >&2; fail=1; }

# 1. Required artifacts.
for f in docs/calque/GOAL.md docs/calque/SPEC.md docs/calque/BOOTSTRAP.md \
         docs/calque/ACCEPTANCE.md docs/calque/PERFORMANCE.md \
         specimens/calque/MANIFEST.md \
         specimens/calque/examples/README.md book/src/specimens/calque.md; do
  [ -f "$f" ] || err "missing artifact: $f"
done
for e in active_score identity_pair fib squares; do
  [ -f "specimens/calque/examples/$e.calq" ] || err "missing example: $e.calq"
done

# 2. Goal pin.
if [ -f docs/calque/GOAL.md ]; then
  got=$(sha256sum docs/calque/GOAL.md | cut -d' ' -f1)
  [ "$got" = "$CALQUE_GOAL_SHA" ] \
    || err "GOAL.md sha256 drifted: $got (pin $CALQUE_GOAL_SHA)"
fi

# 3. Matrix structure.
m=docs/calque/ACCEPTANCE.md
if [ ! -f "$m" ]; then
  err "no acceptance matrix"
  exit 1
fi

ids=$(sed -n 's/^| \(AC-[0-9][0-9]*\.[0-9][0-9]*\) |.*$/\1/p' "$m")
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
bad_status=$(awk -F'|' '/^\| AC-[0-9]+\.[0-9]+ / { s=$(NF-1); gsub(/^ +| +$/, "", s); split(s, w, " "); print w[1] }' "$m" \
  | sort -u | grep -vxE 'done|pending|ongoing|never' || true)
[ -z "$bad_status" ] || err "illegal status keyword(s): $bad_status"

# 4. `done` rows: evidence must resolve. (No pipes around the loops that
# call err: POSIX subshells would swallow the failure flag.)
m4=$(mktemp)
awk -F'|' '/^\| AC-[0-9]+\.[0-9]+ / { s=$(NF-1); gsub(/^ +| +$/, "", s); if (s ~ /^done/) print }' "$m" > "$m4"
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

# 5. Frozen examples are ASCII (SPEC F18).
for e in specimens/calque/examples/*.calq; do
  if LC_ALL=C grep -q '[^ -~]' "$e"; then
    err "non-ASCII bytes in $e (SPEC F18)"
  fi
done

# 6. Wiring.
grep -q '^calque-test:' Makefile || err "Makefile is missing the calque-test target"
grep -q 'specimens/calque.md' book/src/SUMMARY.md || err "book SUMMARY.md is missing the calque chapter"

ntotal=$(printf '%s\n' $ids | grep -c '.' || true)
if [ "$fail" -eq 0 ]; then
  echo "calque-check: ok — artifacts present, matrix well-formed ($ndone/$ntotal rows done)"
else
  echo "calque-check: FAILED" >&2
  exit 1
fi
