### iter 1 — approve — scaffold Rust workspace + docker-compose skeleton; cleared stale Python tree
### iter 2 — approve — tskr-core (events/classifier/renderer/segmenter/manifest, 16 tests); Aws-mode vector configs; canned fixtures
### iter 3 — request_changes — TSKR_EMBEDDING_URL vs TSKR_EMBED_URL mismatch; folded into iter 4
### iter 4 — approve — tskr-writer HTTP skeleton + smoke.sh + real S3/embed/vector clients; env-var fix
### iter 5 — approve — wired pipeline (parse→filter→render→embed→upsert→S3), real /-/ready probes, AppState, compose healthcheck
