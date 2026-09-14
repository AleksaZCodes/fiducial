# Fiducial — Claude Code context

> **Read [`AGENTS.md`](AGENTS.md) first.** It is the agent context for this
> repository — layout, rules, build commands, what not to do — and it is written
> to the portable `AGENTS.md` convention that Codex, Copilot Workspace, Cursor
> and others read.
>
> This file holds **only what is specific to Claude Code.** Everything that is
> not Claude-specific lives in `AGENTS.md`, once. It used to be restated here,
> and the two overlapped on seven of eight sections.

---

## Skills (`/skill-name`, or via the Skill tool)

| Skill | When to use |
|---|---|
| `claude-api` | Any Claude/Anthropic API question — model IDs, pricing, streaming, tool use |
| `code-review` | Review the current diff or a PR for bugs and simplifications |
| `commit-commands:commit` | Create a well-formed commit |
| `commit-commands:commit-push-pr` | Commit, push, and open a PR |
| `fiducial:harvest` | Extract reusable work from another codebase into this one |
| `run` | Run and screenshot the app to verify a change works |
| `security-review` | Security audit of changed code |
| `update-config` | Modify Claude Code settings, hooks, permissions |

## Subagents (`.claude/agents/`)

| Agent | For |
|---|---|
| `fiducial-design` | Architecture and design brainstorming (Opus) |
| `fiducial-review` | Code review — bugs, simplifications, rule violations (Opus) |
| `fiducial-implement` | Coding, refactoring, debugging (Sonnet) |

All three are namespaced on purpose. A subagent's **filename is its identity**,
so an agent called `design` collides with any other `design` agent — from another
plugin, another project, or global config — and the loser of that collision is
silently unavailable rather than an error.

## MCP servers available here

- **`mcp__plugin_github_github__*`** — GitHub API: PRs, issues, branches, files
- **`mcp__context7__*`** — live library documentation (see `AGENTS.md` for when
  you are required to use it)
- **`mcp__ide__getDiagnostics`** — current IDE errors and warnings
- **`DesignSync`** — read and write claude.ai design-system projects. Needs
  `/design-login` first.

## The plugin this repo publishes

`.claude-plugin/plugin.json` ships the `fiducial` plugin: the `PreToolUse` guard
hook (`fid guard-check` before every Bash call) and the `/fiducial:*` commands.
Scaffolded products activate it through a six-line block in
`.claude/settings.json`.

Capability skills (`.claude/skills/*.md`) are installed by `fid add` and
overwritten on every `fid upgrade` — they are platform-owned, so do not edit
them in a product.
