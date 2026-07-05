// asha-perf CLI — runs the launchable-voxel perf baseline and logs structured
// results for same-host trend tracking (#2460).
//
//   pnpm --filter @asha/smoke dev:asha-perf                       → reference (mock) baseline
//   ASHA_PERF_MODE=authority pnpm --filter @asha/smoke dev:asha-perf → native authority path
//
// Output (machine-readable, under harness/perf-out/ — gitignored):
//   • launch-voxel-perf.jsonl       — one JSON line appended per run (trend history)
//   • launch-voxel-perf.latest.json — the latest run, pretty-printed
//
// Exit code reflects STRUCTURAL invariants only (leaks / preview remesh / bounded
// render ops / replay divergence / command acceptance). Timings are logged and
// trended, never a CI-failing gate — so this stays out of harness/ci/check-all.sh.
//
// Set ASHA_PERF_HOST to a stable label per baseline machine; commit/branch are read
// from env (ASHA_PERF_COMMIT/ASHA_PERF_BRANCH) or git, falling back to 'unknown'.
import { execFileSync } from 'node:child_process';
import { appendFileSync, mkdirSync, writeFileSync } from 'node:fs';
import { hostname } from 'node:os';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { formatPerf, runPerf } from './perf.js';
/** Repo-relative artifact directory (harness convention; Den can link to it). */
function artifactDir() {
    // dist/perf-cli.js → package dir is two up; repo root is four up from there.
    const here = dirname(fileURLToPath(import.meta.url));
    return resolve(here, '../../../../harness/perf-out');
}
function git(args) {
    try {
        return execFileSync('git', args, { encoding: 'utf8' }).trim() || null;
    }
    catch {
        return null;
    }
}
function selectedMode() {
    return process.env['ASHA_PERF_MODE'] === 'authority' ? 'authority' : 'reference';
}
async function main() {
    const mode = selectedMode();
    const result = await runPerf({
        mode,
        meta: {
            commit: process.env['ASHA_PERF_COMMIT'] ?? git(['rev-parse', '--short', 'HEAD']) ?? 'unknown',
            branch: process.env['ASHA_PERF_BRANCH'] ??
                git(['rev-parse', '--abbrev-ref', 'HEAD']) ??
                'unknown',
            hostLabel: process.env['ASHA_PERF_HOST'] ?? hostname(),
        },
    });
    process.stdout.write(formatPerf(result));
    const dir = artifactDir();
    mkdirSync(dir, { recursive: true });
    appendFileSync(resolve(dir, 'launch-voxel-perf.jsonl'), JSON.stringify(result) + '\n');
    writeFileSync(resolve(dir, 'launch-voxel-perf.latest.json'), JSON.stringify(result, null, 2) + '\n');
    process.stdout.write(`artifacts: ${dir}/launch-voxel-perf.{jsonl,latest.json}\n`);
    // Structural invariants fail the run hard; timings never do.
    process.exit(result.ok ? 0 : 1);
}
void main();
//# sourceMappingURL=perf-cli.js.map