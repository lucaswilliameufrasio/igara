#!/usr/bin/env bash
set -euo pipefail

payload=$(cat)
tool_name=$(jq -r '.tool_name // ""' <<<"$payload")

if [[ "$tool_name" != "mcp__igara__remote_build" ]]; then
  exit 0
fi

if jq -e '
  (.tool_input.host | type == "string" and length > 0)
  and (.tool_input.command | type == "array" and length > 0 and (.[0] | type == "string" and length > 0))
' >/dev/null <<<"$payload"; then
  exit 0
fi

jq -cn '{
  hookSpecificOutput: {
    hookEventName: "PreToolUse",
    permissionDecision: "deny",
    permissionDecisionReason: "Igará remote builds require a host and a non-empty command."
  }
}'
