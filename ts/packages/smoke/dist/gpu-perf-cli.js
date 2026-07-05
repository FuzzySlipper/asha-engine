// asha-gpu-perf CLI — optional discrete-GPU/WebGL render perf lane (#2461).
//
// Manual, opt-in, and NON-GATING. Without explicit GPU context it writes a
// classified skipped artifact and exits 0 so normal CI/developer machines are not
// blocked by discrete-GPU availability.
import { execFileSync } from 'node:child_process';
import { appendFileSync, mkdirSync, writeFileSync } from 'node:fs';
import { hostname } from 'node:os';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { formatGpuPerf, runGpuPerf } from './gpu-perf.js';
import { runPerf } from './perf.js';
/** Repo-relative artifact directory (harness convention; Den can link to it). */
function artifactDir() {
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
    const env = {
        ...process.env,
        ASHA_PERF_COMMIT: process.env['ASHA_PERF_COMMIT'] ?? git(['rev-parse', '--short', 'HEAD']) ?? 'unknown',
        ASHA_PERF_BRANCH: process.env['ASHA_PERF_BRANCH'] ?? git(['rev-parse', '--abbrev-ref', 'HEAD']) ?? 'unknown',
        ASHA_PERF_HOST: process.env['ASHA_PERF_HOST'] ?? hostname(),
    };
    const result = await runGpuPerf({
        env,
        runBasePerf: async () => runPerf({
            mode,
            meta: {
                commit: env.ASHA_PERF_COMMIT,
                branch: env.ASHA_PERF_BRANCH,
                hostLabel: env.ASHA_PERF_HOST,
            },
        }),
    });
    process.stdout.write(formatGpuPerf(result));
    const dir = artifactDir();
    mkdirSync(dir, { recursive: true });
    appendFileSync(resolve(dir, 'launch-voxel-gpu-perf.jsonl'), JSON.stringify(result) + '\n');
    writeFileSync(resolve(dir, 'launch-voxel-gpu-perf.latest.json'), JSON.stringify(result, null, 2) + '\n');
    process.stdout.write(`artifacts: ${dir}/launch-voxel-gpu-perf.{jsonl,latest.json}\n`);
    // The lane itself is non-gating, but malformed calibration or failed ASHA structural
    // invariants are still honest local failures for an operator-invoked run.
    process.exit(result.ok ? 0 : 1);
}
void main();
//# sourceMappingURL=gpu-perf-cli.js.map