# Remote build reliability

## Scope and decisions (2026-10-25)

The CLI user or explicitly authorized MCP caller selects a registered host,
source, command, optional artifact files, and time budgets. No retry re-executes
a build automatically. Hosts and build commands remain trusted; SSH-user
permissions are not a sandbox.

Lifecycle: validate inputs → connect/preflight → transfer → execute → retrieve
selected artifacts → cleanup. A failed download retains the remote workspace
for recovery. `--keep` always retains it. Interrupted connections make remote
cleanup uncertain; errors report the workspace and never claim removal without
a successful cleanup response.

Python 3 on remote Linux hosts supervises a new process group, imposes an
execution deadline, and terminates that group on cancellation or stdin EOF.
The client has separate connection, transfer, execution and cleanup budgets.
Cancellation is cooperative and bounded; force-killing the client, disconnected
networks, daemonized children or commands escaping their process group cannot
provide an unconditional remote-termination guarantee. The remote deadline is
an additional safety net, not durable job management.

Local SSH/tar children have owned lifetimes. Unix clients use a new process
group and kill it before reaping a cancelled leader; Windows clients attempt a
bounded `taskkill /T` while retaining the child handle, then kill/reap the
leader. Linux behavior is covered by local process-tree and real SSH tests;
native macOS/Windows validation belongs to the CI matrix. Detached local
descendants can also escape these ordinary process-tree mechanisms.

Artifacts are explicit workspace-relative regular files (no globs/directories).
Python opens each path component with directory descriptors and `O_NOFOLLOW`,
rejects hard links and special files, and streams only the opened descriptor.
The stream has a fixed-size length/executable header; length and byte limits
are checked, and executable files are published with owner-only execute
permissions on Unix (never setuid/setgid bits).
The client bounds total bytes, stages downloads privately, and publishes files
by basename without replacement; duplicate basenames are rejected. Existing
output directories are allowed, but existing files are never overwritten.
Multi-file promotion is not transactional; a publication error reports partial
local results and retains the remote workspace. Recursive tar extraction was
rejected to avoid path/link traversal and uncontrolled tree expansion.

`doctor` is a separate read-only journey. Local diagnostics do not create the
database, fix permissions, or contact hosts. An explicit host selects bounded,
noninteractive SSH checks, remote prerequisites and free disk space. Optional
Docker absence is a warning, not failure for SSH-only users.

The build module owns orchestration, the remote helper owns supervision and
safe file reads, and doctor owns diagnostics. CLI and MCP share those contracts.
Tests use synthetic keys and isolated local OpenSSH, never registered hosts.

Release preparation must validate the actual prepared commit, not merely rely
on a bot-created PR event triggering downstream workflows. Explicit workflow
dispatch is the supported `GITHUB_TOKEN` exception; a read-only validation
workflow runs on the prepared branch and reports failures before tagging.
Revisit this design if resumable jobs, untrusted builds, directory artifacts or
atomic multi-file publication become requirements.
