### iter 1 — approve — scaffold Rust workspace + docker-compose skeleton; cleared stale Python tree
### iter 2 — approve — tskr-core (events/classifier/renderer/segmenter/manifest, 16 tests); Aws-mode vector configs; canned fixtures
### iter 3 — request_changes — TSKR_EMBEDDING_URL vs TSKR_EMBED_URL mismatch; folded into iter 4
### iter 4 — approve — tskr-writer HTTP skeleton + smoke.sh + real S3/embed/vector clients; env-var fix
### iter 5 — approve — wired pipeline (parse→filter→render→embed→upsert→S3), real /-/ready probes, AppState, compose healthcheck
### iter 6 — approve — tskr CLI (search/list/show/backfill + daemon stub), 6 parser tests
### iter 7 — approve — tskr-daemon (poll-based, atomic state, backoff); smoke.sh drives real CLI end-to-end (25 tests pass)
### iter 8 — request_changes — Client::new networked breaks tests; smoke.sh deadlocks because vector-writer also needs the bucket; folded into iter 9
### iter 9 — approve — bucket bootstrap moved to minio-init compose service + ensure_bucket pub method called from main; tests restored; static checks clean
### iter 10 — approve — vector-reader host port 8081→18081 (host conflict); smoke flush sleep 3s→15s; smoke.sh PASSES end-to-end; TERMINATION MET
