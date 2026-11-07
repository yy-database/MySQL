# corpus

Frozen captures from a real MySQL instance live here from batch 3 onward.

Rules:

1. Every file carries `provenance` with `serverVersion`, `capturedAt`, the
   driver and its version, and the container image tag.
2. Once frozen, this implementation never writes back into the corpus. If a
   capability changes, add a new capture with a new provenance block.
3. If the real driver or the real server is unavailable, the run reports
   `driver unavailable: <reason>` and fails. It never compares this
   implementation against itself.
