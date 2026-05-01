# tskr Milestone 1 — loop state

## Deliverables (from PLAN.md §Milestone 1)
- [x] docker-compose.yml with minio, embedding-server, vector-writer, vector-reader, tskr-writer
- [x] tskr-writer service (Rust / axum) with /sessions/upload, /healthz, /-/ready
- [x] Chunking + embedding + S3 segment write + vector upsert pipeline
- [x] S3 layout: sessions/<id>/manifest.json + seg-NNNNN.jsonl
- [x] Vector schema (writer-side upsert client iter 4; rows produced+upserted by pipeline iter 5)
- [x] tskr CLI: search / list / show / backfill (iter 6); daemon wired iter 7
- [x] tskr-daemon scanning ~/.claude/projects with ~/.tskr/state.json (iter 7)
- [x] Fixture sessions under tests/fixtures/sessions/
- [x] scripts/smoke.sh end-to-end test (PASSES end-to-end as of iter 10)
- [x] README.md "5 minutes to first search" (iter 8)

## Termination
**MET** at iter 10. All deliverables ticked. `./scripts/smoke.sh` exits 0:
- Stack comes up cleanly (minio→minio-init→embedding-server/vector-writer/vector-reader→tskr-writer all healthy).
- `tskr backfill` ingests 4 fixtures: 48 events accepted, 31 indexed.
- `tskr search "Linux"` returns the short-bug session (session=00000000-0000-0000-0000-000000000001 event=1) with score=1.187.
- `tskr show 00000000-0000-0000-0000-000000000001 --at-event 1` renders the user message with "Linux".
- 25 workspace tests pass.

## Iteration history
- iter 1: workspace + docker-compose skeleton.
- iter 2: tskr-core (16 tests) + Aws-mode vector configs + 4 fixture sessions.
- iter 3+4: tskr-writer HTTP skeleton + smoke.sh skeleton + real S3/embed/vector clients (iter 3 request_changes, folded into iter 4).
- iter 5: full pipeline wired + AppState + real /-/ready dep probes + tskr-writer healthcheck.
- iter 6: tskr CLI (search/list/show/backfill + daemon stub) (6 tests).
- iter 7: tskr-daemon (poll-based, atomic state, backoff) + smoke.sh real CLI wiring.
- iter 8: README + drop aws CLI dep (request_changes).
- iter 9: minio-init compose service + ensure_bucket pub method (fixes iter 8 regressions).
- iter 10: vector-reader host port 8081→18081 + smoke flush sleep 15s; smoke.sh passes.

## Notes
- Deferred (post-milestone-1): multi-tenancy/auth (M2), E2E encryption (M2), embedder bake-off + sub-event chunking + dedup (M3), MCP server / `/tskr` slash command (M4), ratatui TUI for `show`.
- Vector reader host port: 18081 (changed from 8081 to avoid conflict with another local service on this host; PLAN.md was not updated since it doesn't reference a specific host port).
- Search ranking is imperfect (per PLAN.md §Milestone 3, embedder bake-off is deferred). Smoke test asserts session appears, not that it's #1.
