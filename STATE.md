# STATE — frankish live handoff

Updated: 2026-07-02 (bootstrap; no agent session yet)
Phase: pre-M0
Tree: docs only; no code, no CI, suite does not exist yet.

## Next action
Execute M0 per docs/SPEC.md §13: workspace skeleton (§12 layout),
versions.env, `make setup|build|test`, melior smoke (add(i64,i64) via
ExecutionEngine), plain-shell CI script. Read AGENTS.md session protocol
first.

## In flight
Nothing.

## For the human
- Review ⚑ D-005 (host stack ruling) in docs/DECISIONS.md — made on your
  behalf; strike with a superseding entry if wrong before M0 lands.

## Milestone log
(agents append: `mN-done — <one-paragraph note: shipped / learned / cheats
awaiting promotion>`)

## Handoff template (copy for every session end)
    Session end: <date>
    Milestone/step: <where>
    Green? <yes/no — if no, why and where>
    Did: <bullets>
    Next: <single concrete action>
    Landmines: <anything the next agent must not step on>
