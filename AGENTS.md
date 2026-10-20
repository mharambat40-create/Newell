# Newell — Codex entry point

Codex may perform **Architect, Challenger, Executor, Auditor or Explorer**, with
the same responsibilities and limits as Claude. The human selects the role;
the provider does not determine it.

Before acting, read `docs/agents/README.md`, `ARCHITECTURE.md`, then **only the
selected role** under `docs/agents/`. Read task-specific references as that role
requires. These repository files carry context across conversations.

Perform only the requested stage. A request for a prompt authorizes returning
the prompt only. Never dispatch another agent or advance the pipeline
automatically. For a question without an assigned role, answer in read-only mode.

Claude adapters under `.claude/agents/` configure Claude only. They do not
configure Codex tools, models or permissions. Use the shared role documents.

For parallel work, follow `docs/agents/README.md`: separate writable checkouts,
an identified baseline and plan revision, and no changes to another task's work
or Git index. Never merge into `main`; the human controls integration.
