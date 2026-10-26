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

All roles work sequentially in this Newell checkout on `main`. Do not create
task branches, worktrees or separate execution checkouts. Preserve existing
work and the Git index. The human controls integration and external publication.
