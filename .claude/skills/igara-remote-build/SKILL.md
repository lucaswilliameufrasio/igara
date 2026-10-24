---
name: igara-remote-build
description: Use Igará's MCP tools to inspect registered remote hosts and run a user-requested project build over SSH.
---

# Igará remote builds

Use this skill only when the user asks to build project source on a registered
remote host. `remote_build` transfers source files and executes a command on that
host as the saved SSH user; this is remote code execution, not a read-only check.

## Workflow

1. Read the project's build instructions and identify the intended source
   directory and exact command. Ask the user if the host or command is unclear.
2. Call `list_hosts` to confirm that the requested host is registered. The tool
   intentionally returns only its name, connection mode, and default status.
3. Review the source directory for credentials and project-specific secret
   files. Igará excludes common sensitive files, but these patterns are not a
   complete `.gitignore` parser. Add `excludes` for any additional paths that
   must not be transferred. Never pass secrets as command arguments.
4. Call `remote_build` only after the user has explicitly requested or confirmed
   the remote build. Pass the command as an argument array, not as a shell
   string. Use `source` only when the default `.` is not the intended directory.
5. Keep `keep` false unless the user asks to retain remote artifacts. State the
   result and whether Igará removed or retained the temporary workspace.

## Boundaries

- Do not start a build because a hook ran, a host was listed, or a project was
  opened. Hooks may validate or notify only; they must never start builds.
- Do not assume the current/default host is the requested host.
- Do not use `sh -c`, shell interpolation, or redirection unless the user
  specifically needs it and has approved the exact command.
- MCP server output uses stdout for protocol messages; remote command output is
  routed to stderr. Do not change that routing or print diagnostics to stdout.
- Keep the OpenCode permission for `igara_remote_build` set to `ask`; do not add
  broad auto-approval for this tool.
