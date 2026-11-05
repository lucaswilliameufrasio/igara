#!/usr/bin/env bash
set -euo pipefail

branch="${1:?prepared release branch required}"
commit="${2:?prepared commit required}"
if [[ ! "$branch" =~ ^release/v[0-9]+\.[0-9]+\.[0-9]+-[0-9]+-[0-9]+$ ]] ||
   [[ ! "$commit" =~ ^[0-9a-f]{40}$ ]]; then
  echo "error: invalid prepared branch or commit" >&2
  exit 1
fi

gh workflow run ci.yml --ref "$branch" >&2
run_id=""
for attempt in {1..30}; do
  run_id=$(gh run list --workflow ci.yml --branch "$branch" \
    --event workflow_dispatch --commit "$commit" --limit 1 \
    --json databaseId --jq '.[0].databaseId // empty')
  if [[ -n "$run_id" ]]; then
    break
  fi
  sleep 2
done
if [[ ! "$run_id" =~ ^[0-9]+$ ]]; then
  echo "error: dispatched validation run was not found; keep the branch for investigation" >&2
  exit 1
fi

gh run watch "$run_id" --exit-status --interval 10 >&2
# Do not trust a successful command alone: bind success to the prepared SHA.
actual=$(gh run view "$run_id" --json headSha,status,conclusion \
  --jq '[.headSha,.status,.conclusion] | join(" ")')
if [[ "$actual" != "$commit completed success" ]]; then
  echo "error: validation did not succeed on the prepared commit" >&2
  exit 1
fi
gh run view "$run_id" --json url --jq '.url'
