#!/bin/sh
[ "${ALLOW_LINT:-}" = "1" ] && exit 0
[ -x node_modules/.bin/gherkin-lint-plus ] || exit 0
features=${FEATURES_DIR:-features}
git cat-file -e "HEAD:$features" 2>/dev/null || exit 0
if ! git cat-file -e "HEAD:.gherkin-lintrc" 2>/dev/null; then
  echo "pre-push: $features is committed, but no .gherkin-lintrc is, so the specification cannot be linted." >&2
  echo "pre-push: commit the lint rules, or override: ALLOW_LINT=1 git push" >&2
  exit 1
fi

# Lint the specification as committed, by the lint rules as committed, as feedback here and a gate in CI.
tree=$(mktemp -d "${TMPDIR:-/tmp}/nvlint.XXXXXX")
found=$(mktemp "${TMPDIR:-/tmp}/nvlint.XXXXXX")
git archive HEAD "$features" .gherkin-lintrc | tar -x -C "$tree"
node_modules/.bin/gherkin-lint-plus -c "$tree/.gherkin-lintrc" \
  "$tree/$features" >"$found" 2>&1
status=$?
if [ "$status" -ne 0 ] || [ -s "$found" ]; then
  [ -s "$found" ] || echo "pre-push: the linter failed to run (exit $status)." >&2
  [ -s "$found" ] && echo "pre-push: the specification does not lint." >&2
  sed "s#$tree/##g" "$found" >&2
  rm -rf "$tree" "$found"
  echo "pre-push: fix it, or override: ALLOW_LINT=1 git push" >&2
  exit 1
fi
rm -rf "$tree" "$found"

# Local addition: the Czech mirror (not executed, not linted) must stay in sync,
# file for file, with the same @id and the same number of scenarios.
mirror=${SPEC_MIRROR_DIR:-docs/spec}/cs
out=$(git -c core.quotePath=false ls-tree -r --name-only HEAD "$features" | grep '\.feature$' | while IFS= read -r f; do
  m="$mirror/${f#"$features"/}"
  if ! git cat-file -e "HEAD:$m" 2>/dev/null; then echo "  missing Czech mirror: $m"; continue; fi
  id_en=$(git show "HEAD:$f" | grep -o '@id:[^ ]*' | head -1)
  id_cs=$(git show "HEAD:$m" | grep -o '@id:[^ ]*' | head -1)
  [ "$id_en" = "$id_cs" ] || echo "  different @id: $f ($id_en) vs $m ($id_cs)"
  n_en=$(git show "HEAD:$f" | grep -cE '^[[:space:]]*Scenario( Outline)?:')
  n_cs=$(git show "HEAD:$m" | grep -cE '^[[:space:]]*(Scénář|Náčrt Scénáře):')
  [ "$n_en" = "$n_cs" ] || echo "  different number of scenarios: $f ($n_en) vs $m ($n_cs)"
done)
if [ -n "$out" ]; then
  echo "pre-push: the Czech specification is out of sync with $features:" >&2
  printf '%s\n' "$out" >&2
  echo "pre-push: update $mirror, or override: ALLOW_LINT=1 git push" >&2
  exit 1
fi
exit 0
