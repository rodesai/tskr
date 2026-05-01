# tskr test fixtures

This directory holds canned Claude session JSONL files used by `tskr-core`
unit tests, the writer's pipeline tests, and `scripts/smoke.sh`. The
sessions are **synthetic** — they are not real conversations from anyone's
`~/.claude/projects/` directory; they only mimic the *shape* of those
files (top-level fields like `type`, `sessionId`, `uuid`, `parentUuid`,
`timestamp`, and the role-specific `message` content blocks). One session
per file under `sessions/`, named `<scenario>.jsonl`. Each file's events
share a single `sessionId` of the form
`00000000-0000-0000-0000-00000000000N`, where `N` matches the fixture's
position so smoke tests can reference them by literal string. Together the
four fixtures cover every row of the event taxonomy in `PLAN.md` §Components
step 2: indexable `assistant`/`user`/`system.away_summary` events,
`<local-command-caveat>` user echoes that must be skipped, `system` events
with non-`away_summary` subtypes, and the boring-but-persisted bucket
(`permission-mode`, `last-prompt`, `attachment`, `file-history-snapshot`,
`queue-operation`). `tool-walk.jsonl` includes an oversized tool_result so
the renderer's truncation path is exercised.
