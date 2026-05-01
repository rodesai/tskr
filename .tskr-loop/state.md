# tskr Milestone 1 — loop state

## Deliverables (from PLAN.md §Milestone 1)
- [~] docker-compose.yml with minio, embedding-server, vector-writer, vector-reader, tskr-writer (skeleton landed iter 1; tskr-specific vector configs in flight via w2)
- [ ] tskr-writer service (Rust / axum) with /sessions/upload, /healthz, /-/ready
- [~] Chunking + embedding + S3 segment write + vector upsert pipeline (event taxonomy / classifier / segmenter / manifest types in flight via w1; embed + S3 + vector clients deferred)
- [ ] S3 layout: sessions/<id>/manifest.json + seg-NNNNN.jsonl (manifest type in flight via w1; writer-side persistence deferred)
- [ ] Vector schema (session_id, event_index, segment_index, author, repo, model, role, timestamp, text)
- [ ] tskr CLI: search / list / show / daemon / backfill
- [ ] tskr-daemon scanning ~/.claude/projects with ~/.tskr/state.json
- [~] Fixture sessions under tests/fixtures/sessions/ (in flight via w3)
- [ ] scripts/smoke.sh end-to-end test
- [ ] README.md "5 minutes to first search"

## Notes
- Iter 1: scaffold-only Cargo workspace + docker-compose skeleton. Stale Python tree removed. Embedding-server / vector-writer / vector-reader images reuse opendata.
- Iter 1 Reviewer flagged two follow-ups (non-blocking): (a) vector configs are still opendata's quickstart `Local`-mode YAML — needs Aws-mode tskr configs and a compose remount; (b) tskr-writer healthcheck deferred until /healthz lands.
- Iter 2 plan: w1 builds `tskr-core` (event taxonomy types, classifier, text renderer, 10-event segmenter, manifest); w2 swaps in tskr-specific Aws-mode `config/vector/{writer,reader}.yaml` and remounts in compose; w3 drops 3–5 canned fixture sessions covering every taxonomy row.
- Aws ObjectStoreConfig schema (verified in /home/ubuntu/opendata/common/src/storage/config.rs): `{type: Aws, region: <str>, bucket: <str>}` — endpoint/credentials come from `AWS_ENDPOINT_URL` / `AWS_ACCESS_KEY_ID` / `AWS_SECRET_ACCESS_KEY` / `AWS_REGION` / `AWS_ALLOW_HTTP` env vars (already plumbed in docker-compose.yml).
- Real Claude session jsonl shapes (sampled from ~/.claude/projects): event types observed include `user`, `assistant`, `queue-operation`, `attachment`, `last-prompt`, `ai-title`. Top-level fields commonly present: `type`, `sessionId`, `uuid`, `parentUuid`, `timestamp`, plus role-specific bodies.
- Deferred for iter 3+: tskr-writer axum routes + pipeline (S3/embed/vector clients), CLI, daemon, smoke.sh, healthcheck on tskr-writer.
- MinIO bucket creation (`tskr`) belongs in smoke.sh, not compose — defer.

## Last reviewer rationale
(iter 1) Iteration 1 scaffold is correct. Two deferrals to flag for the next Orchestrator turn, neither blocking iter 1:
(a) vector-writer/reader currently mount opendata's quickstart `config/{writer,reader}.yaml` which use a Local object store at `/data/store` rather than S3/MinIO. The smoke test will need vector to actually round-trip through MinIO, so a future iteration must drop tskr-specific Aws-mode vector configs into the repo (e.g. `config/vector/{writer,reader}.yaml`) and remount those instead of the opendata files. The AWS_* env vars are already plumbed, so this is a config swap, not a compose rewrite.
(b) tskr-writer healthcheck is intentionally omitted with an inline comment; must be added when the real `/healthz` lands.
