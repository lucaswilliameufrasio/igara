# Agent integration

Igará's MCP server is an optional local stdio process. It reads the same local
SQLite host registry and SSH configuration as the CLI; it does not expose a
network listener or accept remote connections.

## Capabilities and boundaries

The server exposes two tools:

- `list_hosts` returns only a host's registered name, connection mode, and
  default-host status. It omits addresses, usernames, SSH aliases, key paths,
  tags, and credentials.
- `remote_build` transfers the selected local directory and runs a supplied
  argument vector on the selected host as the registered SSH user. The build
  command has that user's permissions and is remote code execution, not a
  read-only operation. The source is relative to the MCP process working
  directory unless an absolute path is supplied. The default source is `.`;
  `keep` defaults to `false` and `excludes` defaults to an empty list.

Only an explicit call to `remote_build` starts a build. Hooks below only reject
malformed calls or report that a call was requested. They do not invoke the CLI,
call the MCP tool, or start builds.

The stdio protocol owns stdout. Igará routes remote build stdout to the server's
stderr in MCP mode, and suppresses stdout from transfer/cleanup SSH processes.
Errors and build status are returned as tool results; progress output remains on
stderr. The regular CLI keeps its existing output behavior.

## Claude Code

Add this server entry to the project's `.mcp.json` (or merge it into the existing
file). Use an absolute executable path if `igara` is not on Claude Code's `PATH`.

```json
{
  "mcpServers": {
    "igara": {
      "command": "igara",
      "args": ["mcp", "serve"]
    }
  }
}
```

The example settings in `examples/hooks/claude-code/settings.json` include an
`ask` rule for every `mcp__igara__remote_build` call and an optional `PreToolUse`
validation hook. Merge both entries into `.claude/settings.json` to enable them.
Do not add this tool to an allow-list. The hook requires `jq` and only rejects
malformed input; the `ask` rule is what requests approval before each build.

## OpenCode V2

Add the local server and the `ask` rule to the project's `opencode.jsonc`:

```jsonc
{
  "$schema": "https://opencode.ai/config.json",
  "mcp": {
    "servers": {
      "igara": {
        "type": "local",
        "command": ["igara", "mcp", "serve"]
      }
    }
  },
  "permissions": [
    { "action": "igara_remote_build", "resource": "*", "effect": "ask" }
  ]
}
```

Use an absolute path for the executable if `igara` is not available on
OpenCode's `PATH`.

OpenCode normalizes MCP tool permissions as `<server>_<tool>`, so the rule asks
before every `igara_remote_build` invocation. Keep the rule at `ask`; do not
change it to `allow` for convenience. The optional notification plugin in
`examples/hooks/opencode/` logs the target host before a call. Copy it into
`.opencode/plugins/` to enable it. It does not change arguments or execute
commands.

## Shared skill

`.claude/skills/igara-remote-build/SKILL.md` is the shared Claude Code/OpenCode
skill. Both clients can discover the `.claude/skills` layout. It guides the
agent to inspect project build instructions and source exclusions, confirm the
host and command with the user, and call `remote_build` only when requested.

## Safety checklist

Before calling `remote_build`:

1. Confirm the user asked for a remote build and identify the intended registered
   host and exact command. Ask if either is ambiguous.
2. Review the project build instructions and source directory. Default tar
   exclusions cover common credentials and build outputs but are not a complete
   `.gitignore` parser; add project-specific exclusions before transfer.
3. Remember that the command runs on the remote machine with the SSH user's
   permissions. Do not send secrets as command arguments or transfer credentials
   just to make a build pass.
4. Leave `keep` false unless the user specifically needs the remote artifacts.
5. Report whether the build succeeded and whether its remote workspace was
   removed or retained. Do not claim that a local build or a hook ran remotely.

The MCP server and hooks are opt-in. No build is run when Igará starts, when a
hook runs, or when a host is listed.
