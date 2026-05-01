# tskr Milestone 1 — loop state

## Deliverables (from PLAN.md §Milestone 1)
- [ ] docker-compose.yml with minio, embedding-server, vector-writer, vector-reader, tskr-writer
- [ ] tskr-writer service (Rust / axum) with /sessions/upload, /healthz, /-/ready
- [ ] Chunking + embedding + S3 segment write + vector upsert pipeline
- [ ] S3 layout: sessions/<id>/manifest.json + seg-NNNNN.jsonl
- [ ] Vector schema (session_id, event_index, segment_index, author, repo, model, role, timestamp, text)
- [ ] tskr CLI: search / list / show / daemon / backfill
- [ ] tskr-daemon scanning ~/.claude/projects with ~/.tskr/state.json
- [ ] Fixture sessions under tests/fixtures/sessions/
- [ ] scripts/smoke.sh end-to-end test
- [ ] README.md "5 minutes to first search"

## Notes
- Iteration 1: scaffold-only. Removed stale Python attempt under `cli/`, `services/`, `.smokevenv/`. Created Cargo workspace with empty crate skeletons for `tskr-core`, `tskr-writer`, `tskr-cli`, `tskr-daemon`. Created `docker-compose.yml` skeleton wiring minio, embedding-server, vector-writer, vector-reader, tskr-writer; `services/tskr-writer/Dockerfile` is a multi-stage Rust build for the writer crate.
- Embedding server image is reused from `../opendata` per PLAN; compose references it via build context `../opendata` and existing `Dockerfile.embedding-server`.
- `vector-writer` / `vector-reader` images: reuse opendata vector quickstart definitions (compose `extends` or replicate the `build` lines from `../opendata/vector/quickstart/docker-compose.yml`). Worker should mirror what the quickstart does.
- Real writer logic (axum routes, pipeline) deferred to later iterations; iteration 1 ships placeholder `main.rs`/`lib.rs` only.

## Last reviewer rationale
(none — iteration 1)
