#!/bin/sh
[ "${ALLOW_PHASE:-}" = "1" ] && exit 0
[ "$(git symbolic-ref --short HEAD 2>/dev/null)" = "main" ] && exit 0
remote=${1:-origin}
base=$(git merge-base "$remote/main" HEAD 2>/dev/null) || {
  echo "pre-push: phases: no merge-base with $remote/main (fetch it?); phase check skipped." >&2
  exit 0
}

# A spec: commit touches only the spec, an arch: commit only the model and ADRs, any other neither.
# A merge is held to the same rule for what its conflict resolution changed.
features=${FEATURES_DIR:-features}
mirror=${SPEC_MIRROR_DIR:-docs/spec}
model=${ARCHITECTURE_DIR:-docs/architecture}
adrs=${ADRS_DIR:-docs/adrs}
found=$(mktemp "${TMPDIR:-/tmp}/nvphase.XXXXXX")
git rev-list "$base"..HEAD | while read -r sha; do
  case "$(git log -1 --format=%s "$sha")" in
    spec:*|spec\(*) phase=spec ;;
    arch:*|arch\(*) phase=arch ;;
    *) phase=code ;;
  esac
  if [ "$(git rev-list --parents -n 1 "$sha" | wc -w)" -gt 2 ]; then
    files=$(git -c core.quotePath=false show --remerge-diff --format= --name-only "$sha")
  else
    files=$(git -c core.quotePath=false diff-tree --no-commit-id --name-only -r "$sha")
  fi
  printf '%s\n' "$files" | while IFS= read -r file; do
    [ -n "$file" ] || continue
    case "$file" in
      "$features"/*|"$mirror"/*) kind=spec ;;
      "$model"/*|"$adrs"/*) kind=arch ;;
      *) kind=code ;;
    esac
    [ "$kind" = "$phase" ] || echo "  $(git log -1 --format='%h %s' "$sha")  ->  $file" >>"$found"
  done
done
if [ -s "$found" ]; then
  echo "pre-push: these commits touch files outside their phase:" >&2
  cat "$found" >&2
  rm -f "$found"
  echo "pre-push: spec: for $features and $mirror, arch: for $model and $adrs, any other commit for the rest." >&2
  echo "pre-push: split the commit, or override: ALLOW_PHASE=1 git push" >&2
  exit 1
fi
rm -f "$found"
exit 0
