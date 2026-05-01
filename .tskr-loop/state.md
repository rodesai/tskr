# tskr Milestone 1 — loop state

## Deliverables (from PLAN.md §Milestone 1)
- [x] docker-compose.yml with minio, embedding-server, vector-writer, vector-reader, tskr-writer (skeleton iter 1; tskr-specific Aws-mode vector configs iter 2)
- [~] tskr-writer service (Rust / axum) with /sessions/upload, /healthz, /-/ready (HTTP skeleton + stub pipeline pending commit from iter 3; env-var fix folded into iter 4 w1; real /-/ready dep probe deferred to iter 5)
- [~] Chunking + embedding + S3 segment write + vector upsert pipeline (tskr-core landed iter 2; module stubs pending commit from iter 3; real S3/embed/vector clients in flight via iter 4 w1/w2/w3; pipeline wiring iter 5)
- [~] S3 layout: sessions/<id>/manifest.json + seg-NNNNN.jsonl (manifest type landed iter 2; writer-side persistence in flight via iter 4 w1)
- [~] Vector schema (writer-side upsert client in flight via iter 4 w3; metadata fields already declared in config/vector/writer.yaml)
- [ ] tskr CLI: search / list / show / daemon / backfill (iter 6)
- [ ] tskr-daemon scanning ~/.claude/projects with ~/.tskr/state.json (iter 7)
- [x] Fixture sessions under tests/fixtures/sessions/
- [~] scripts/smoke.sh end-to-end test (skeleton w/ CLI steps stubbed pending commit from iter 3; real CLI wiring iter 7)
- [ ] README.md "5 minutes to first search" (iter 8)

## Notes
- Iter 1: scaffold-only Cargo workspace + docker-compose skeleton.
- Iter 2: tskr-core (typed events, classifier, renderer, segmenter, manifest); Aws-mode vector configs; 4 fixtures. 16 tests pass.
- Iter 3: HTTP skeleton + smoke.sh skeleton. NOT committed (request_changes); landing with iter 4.
- Iter 3 reviewer rationale (request_changes): config.rs requires `TSKR_EMBEDDING_URL` while docker-compose sets `TSKR_EMBED_URL`. Fix: change config.rs to read `TSKR_EMBED_URL`. Field name `embedding_url` stays. Folded into iter 4 w1.
- Iter 4 plan: w1 = TSKR_EMBED_URL fix + real S3 client (aws-sdk-s3 with endpoint_url + force_path_style + Credentials from TSKR_S3_ACCESS_KEY/TSKR_S3_SECRET_KEY env vars; put_segment/put_manifest/get_manifest/list_segment_indices). w2 = real embed client (reqwest POST /embed with {texts:[...]} -> {embeddings:[[..]]}). w3 = real vector client (reqwest POST /api/v1/vector/write with {upsertVectors:[...]}, Content-Type application/protobuf+json).
- Iter 4 worker file boundaries: w1=config.rs + s3.rs + Cargo.toml + Cargo.lock + tests/s3.rs (optional); w2=embed.rs only; w3=vector.rs only. Only w1 may touch Cargo.toml/Cargo.lock.
- Vector writer endpoint: POST http://vector-writer:8080/api/v1/vector/write with Content-Type: application/protobuf+json. Body: {"upsertVectors":[{"id":"<sid>:<event_index>","attributes":{"vector":[..f32..],"session_id":"..","event_index":N,"segment_index":N,"author":"..","repo":"..","model":"..","role":"..","timestamp":"..","text":".."}}]}.
- Embedding server endpoint: POST http://embedding-server:9000/embed with body {"texts":["..","..."]}. Response: {"embeddings":[[f32..]]}. Output dim = 384.
- Deferred for iter 5: pipeline wiring into /sessions/upload route + real /-/ready dep probes + integration test against running stack + tskr-writer healthcheck in compose (re-enable once /-/ready probes deps).
- Deferred for iter 6: tskr CLI (search/list/show/backfill).
- Deferred for iter 7: tskr-daemon + final smoke.sh CLI wiring (drop --skip-cli default).
- Deferred for iter 8: README.

## Last reviewer rationale
(iter 3) request_changes. config.rs requires `TSKR_EMBEDDING_URL` while docker-compose.yml sets `TSKR_EMBED_URL`. Fix: change `crates/tskr-writer/src/config.rs` to read `TSKR_EMBED_URL`. Field name `embedding_url` stays. One-line edit; do not modify docker-compose.yml.
