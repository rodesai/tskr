# tskr Milestone 1 — loop state

## Deliverables (from PLAN.md §Milestone 1)
- [x] docker-compose.yml with minio, embedding-server, vector-writer, vector-reader, tskr-writer
- [x] tskr-writer service (Rust / axum) with /sessions/upload, /healthz, /-/ready
- [x] Chunking + embedding + S3 segment write + vector upsert pipeline
- [x] S3 layout: sessions/<id>/manifest.json + seg-NNNNN.jsonl
- [x] Vector schema (writer-side upsert client iter 4; rows produced+upserted by pipeline iter 5)
- [~] tskr CLI: search / list / show / backfill (iter 6 in flight); daemon stubbed (iter 7)
- [ ] tskr-daemon scanning ~/.claude/projects with ~/.tskr/state.json (iter 7)
- [x] Fixture sessions under tests/fixtures/sessions/
- [~] scripts/smoke.sh end-to-end test (skeleton iter 3; real CLI wiring iter 7)
- [ ] README.md "5 minutes to first search" (iter 8)

## Notes
- Iter 1–5 committed. tskr-writer pipeline fully wired and unit-tested.
- Iter 6 plan: tskr CLI single-worker build — Cargo.toml deps, clap derive subcommands (search/list/show/backfill/daemon), config loader with localhost defaults, embed/vector/s3 client modules, command implementations, parser unit tests. daemon subcommand prints 'tskr daemon: deferred to iter 7' and exits 2.
- Verified vector reader endpoint: POST `/api/v1/vector/search` Content-Type `application/protobuf+json`. Request: `{vector: [f32], k: u32, filter?: JsonFilter, include_fields?: [str]}`. JsonFilter: `{eq:{field,value}}`, `{neq:...}`, `{in:{field,values:[]}}`, `{and:[...]}`, `{or:[...]}`. No gte/lte; `--since` is a client-side filter.
- Response shape: `{status, results: [{score, vector: {id, attributes: {flat_metadata}}}]}`.
- Defaults so the CLI works locally: TSKR_WRITER_URL=http://localhost:8090, TSKR_EMBED_URL=http://localhost:9000, TSKR_VECTOR_READER_URL=http://localhost:8081, TSKR_S3_ENDPOINT=http://localhost:9100, TSKR_S3_BUCKET=tskr, TSKR_S3_ACCESS_KEY=minioadmin, TSKR_S3_SECRET_KEY=minioadmin, TSKR_S3_REGION=us-east-1.
- Search output line: `[<author>/<repo>] <ts> "<text first 100>" (score=...) — session=<session_id> event=<event_index>`. Smoke.sh in iter 7 will grep for the short-bug session_id `00000000-0000-0000-0000-000000000001`.
- Show is headless-only. TUI (ratatui) deferred — milestone 1 calls for it but smoke test only needs headless.
- Deferred for iter 7: tskr-daemon + smoke.sh CLI wiring.
- Deferred for iter 8: README.

## Last reviewer rationale
(iter 5) approve — pipeline + AppState + healthcheck. Committed as 6663bb4.
