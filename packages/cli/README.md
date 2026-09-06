# @fiducial/cli

> **fid** — the Fiducial platform CLI

`fid` is the command-line interface for the [Fiducial](https://github.com/AleksaZCodes/fiducial)
platform. It scaffolds product repositories, manages capabilities, runs derive
pipelines, and enforces platform guard rules.

## Install

The binary is written in Rust and distributed via both cargo and npm:

```sh
# Via cargo (recommended for development)
cargo install fiducial-cli

# Via npm (for JS-first teams or CI environments)
npm install -g @fiducial/cli
```

> **Note:** The npm package is a thin shim that exec-replaces into the native
> `fid` binary. You still need the Rust binary on PATH (or set `FID_BIN`).

## Commands

| Command | Does |
| --- | --- |
| `fid new <name>` | Scaffold a new product repository |
| `fid add app <target>` | Add a Next / SvelteKit / Tauri / mobile / worker app |
| `fid add firmware <target>` | Add firmware support (rp2040, stm32, nrf52, …) |
| `fid add module <name>` | Add a bounded-context Rust+TS module |
| `fid doctor` | Check for drift: stale templates, outdated deps |
| `fid guard-check` | Shell-aware PreToolUse guard hook (used by Claude Code) |

## Guard

The guard runs as a Claude Code `PreToolUse` hook. Products scaffolded with
`fid new` get `.claude/settings.json` pre-configured to call `fid guard-check`
before every Bash tool call.

The guard is **shell-aware**: it tokenizes the command and only fires when a
forbidden word appears in command position (argv[0]). Running
`echo "install npm packages"` never triggers the package-manager rule because
`npm` is inside a quoted string argument — not a command.

This eliminates the false positives that blocked legitimate work in the
original regex-based guards.

## License

MIT
