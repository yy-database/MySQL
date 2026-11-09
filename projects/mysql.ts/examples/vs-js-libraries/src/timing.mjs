// Timing helpers for the cross-library benchmark. Median and p95 are computed
// by `database.harness` through the napi primitive `harnessBenchSummary`
// (middle-two median, nearest-rank p95 — the same formulas this file carried
// before, now pinned to the shared golden vectors).

import { createRequire } from "node:module";

const require = createRequire(import.meta.url);
const timingAddon = require(`../../../platforms/mysql-${process.platform}-${process.arch}`);

export function elapsedMs(start) {
  return Number(process.hrtime.bigint() - start) / 1_000_000;
}

export function median(samples) {
  return timingAddon.harnessBenchSummary(samples).medianMs;
}

export function p95(samples) {
  return timingAddon.harnessBenchSummary(samples).p95Ms;
}

export function throughput(ops, ms) {
  if (ms <= 0) return "n/a";
  return `${Math.round((ops / ms) * 1_000).toLocaleString("en-US")}/s`;
}
