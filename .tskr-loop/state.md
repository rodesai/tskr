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
- [~] scripts/smoke.sh end-to-end test (iter 8 dropped `aws` dep; iter 9 fixes regressions: vector-writer needs bucket pre-created, and Client::new must not network on construction)
- [~] README.md "5 minutes to first search" (landed in iter 8; final verification with passing smoke run pending)

## Notes
- Iter 1–7 committed. Iter 8 not yet committed (Reviewer requested changes).
- Iter 9 plan: w1 (single worker) — (a) move ensure_bucket out of `Client::new` into a public method called explicitly from main.rs; (b) add `minio-init` one-shot compose service using `minio/mc:latest` running `mc mb --ignore-existing local/tskr`; (c) make vector-writer/vector-reader/tskr-writer depend on `minio-init: condition: service_completed_successfully`; (d) Worker MUST run `cargo test --workspace`.
- `head_bucket`'s NotFound predicate in aws_sdk_s3 v1.x is `is_not_found()`.
- Deferred: multi-tenancy/auth, E2E encryption, embedder bake-off, MCP server, ratatui TUI.

## Last reviewer rationale
(iter 8) request_changes — Two real regressions:
(1) `cargo test --workspace` fails: tskr-writer/tests/health.rs panics because `Client::new` now eagerly calls `head_bucket` against the test's fake endpoint `http://127.0.0.1:1`.
(2) `./scripts/smoke.sh` fails: vector-writer also uses MinIO bucket `tskr` for SlateDB. tskr-writer depends_on vector-writer healthy. Bucket-create moved into tskr-writer means vector-writer never gets the bucket, never becomes healthy, tskr-writer never starts.
