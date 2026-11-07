# corpus

Frozen captures from a real MySQL instance.

Rules:

1. Every file carries `provenance` with `serverVersion`, `capturedAt`, the
   driver and its version, the container image tag and its digest.
2. Once frozen, this implementation never writes back into the corpus. If a
   capability changes, add a new capture with a new provenance block.
3. If the real driver or the real server is unavailable, the run reports
   `driver unavailable: <reason>` and fails. It never compares this
   implementation against itself.

## Freezing a corpus

`stages/truth.mjs capture` refuses to overwrite `truth.json` unless
`TRUTH_CAPTURE_REFRESH=1` is set. The capture needs Docker with `mysql:8.4`.

The `Truth corpus` workflow captures on a Docker-capable runner and uploads
`truth-corpus`, so a maintainer can commit `truth.json` without local Docker:

```shell
gh run download <run-id> -n truth-corpus -D <temp-dir>
```

`stages/truth.mjs verify` only reads the corpus. Until `truth.json` exists,
`stages/truth.mjs` captures into a temporary file, verifies against it and says
the corpus is not frozen yet.
