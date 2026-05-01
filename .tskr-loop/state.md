# tskr Milestone 1 — loop state

## Deliverables (from PLAN.md §Milestone 1)
- [x] docker-compose.yml with minio, embedding-server, vector-writer, vector-reader, tskr-writer (skeleton iter 1; tskr-specific Aws-mode vector configs iter 2; tskr-writer healthcheck re-enabled iter 5)
- [~] tskr-writer service (Rust / axum) with /sessions/upload, /healthz, /-/ready (iter 4 landed HTTP skeleton + real client modules; iter 5 wires pipeline + real /-/ready dep probes + AppState)
- [~] Chunking + embedding + S3 segment write + vector upsert pipeline (tskr-core landed iter 2; clients iter 4; full pipeline wired in iter 5)
- [~] S3 layout: sessions/<id>/manifest.json + seg-NNNNN.jsonl (manifest type iter 2; segment + manifest persistence wired iter 5)
- [~] Vector schema (writer-side upsert client iter 4; rows produced+upserted by pipeline iter 5)
- [ ] tskr CLI: search / list / show / daemon / backfill (iter 6)
- [ ] tskr-daemon scanning ~/.claude/projects with ~/.tskr/state.json (iter 7)
- [x] Fixture sessions under tests/fixtures/sessions/
- [~] scripts/smoke.sh end-to-end test (skeleton iter 3; real CLI wiring iter 7)
- [ ] README.md "5 minutes to first search" (iter 8)

## Notes
- Iter 1: scaffold-only Cargo workspace + docker-compose skeleton.
- Iter 2: tskr-core (typed events, classifier, renderer, segmenter, manifest); Aws-mode vector configs; 4 fixtures. 16 tests pass.
- Iter 3: HTTP skeleton + smoke.sh skeleton. NOT committed (request_changes); landed with iter 4.
- Iter 4: TSKR_EMBED_URL fix + real S3/embed/vector client modules. Approved (a483691). Pipeline still stubbed.
- Iter 5 plan: w1 = pipeline.rs (full per-upload algorithm) + routes.rs (header parsing, AppState, real /-/ready dep probes) + main.rs (build AppState) + s3.rs (add ready() head_bucket + get_segment()) + embed.rs (add ready() GET /health) + vector.rs (add ready_writer()/ready_reader() GET /-/ready) + lib.rs (export AppState) + tests/health.rs (single test). w2 = re-enable docker-compose tskr-writer healthcheck against http://localhost:8090/healthz.
- Pipeline contract: route reads X-Tskr-Author/Repo/Host (req'd, 400 if missing) + X-Tskr-Start-Event-Index (opt, default 0). Pipeline gets manifest, filters incoming events by global_idx > last_persisted, groups into 10-event segments, fetches+merges the lowest extending partial segment, classifies+renders survivors, embeds in one batch, builds UpsertRows (id=`{session_id}:{event_index}`), upserts via vector client, PUTs every affected segment, PUTs updated manifest preserving started_at. Returns {accepted, indexed}.
- Verified upstream endpoints: embedding-server `/health` (Flask); opendata vector-writer/reader `/-/ready`. MinIO readiness via head_bucket.
- AppState design: struct AppState { cfg, s3, embed, vector }. routes::app(Arc<AppState>) -> Router uses State<Arc<AppState>>. main.rs awaits S3 ctor, builds the rest sync.
- Deferred for iter 6: tskr CLI.
- Deferred for iter 7: tskr-daemon + smoke.sh CLI wiring.
- Deferred for iter 8: README.

## Last reviewer rationale
(iter 4) approve — tskr-writer HTTP skeleton + smoke.sh + real S3/embed/vector clients; env-var fix. Committed as a483691.
