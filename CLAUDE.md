# Newell — Claude entry point

Claude may perform **Architect, Challenger, Executor, Auditor or Explorer**, with
the same responsibilities and limits as Codex. The human selects the role;
the provider does not determine it.

Before acting, read `docs/agents/README.md`, `ARCHITECTURE.md`, then **only the
selected role** under `docs/agents/`. Read task-specific references as that role
requires. These repository files carry context across conversations.

Perform only the requested stage. A request for a prompt authorizes returning
the prompt only. Never dispatch another agent or advance the pipeline
automatically. For a question without an assigned role, answer in read-only mode.

`.claude/agents/` contains Claude adapters to the shared role documents.
`.claude/settings.json` applies only to Claude; it does not configure Codex.
Written project instructions do not replace tool permissions or CI checks.

All roles work sequentially in this Newell checkout on `main`. Do not create
task branches, worktrees or separate execution checkouts. Preserve existing
work and the Git index. The human controls integration and external publication.
