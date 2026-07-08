#!/usr/bin/env node
import { ASHA_BROWSER_HOST_COMMAND, launchNativeBrowserHost, } from './index.js';
async function main() {
    const options = parseArgs(process.argv.slice(2));
    if (options.uiRoot === undefined) {
        throw new Error(`Missing --ui-root. Example: ${ASHA_BROWSER_HOST_COMMAND}`);
    }
    const host = await launchNativeBrowserHost({
        uiRoot: options.uiRoot,
        ...(options.host !== undefined ? { host: options.host } : {}),
        ...(options.port !== undefined ? { port: options.port } : {}),
        ...(options.healthProject !== undefined ? { healthProject: options.healthProject } : {}),
    });
    console.log(JSON.stringify({
        kind: host.kind,
        compatibilityVersion: host.compatibilityVersion,
        url: host.url,
        provider: host.provider,
    }, null, 2));
    process.on('SIGINT', () => {
        void host.close().then(() => process.exit(0));
    });
    process.on('SIGTERM', () => {
        void host.close().then(() => process.exit(0));
    });
}
function parseArgs(argv) {
    const parsed = {};
    for (let index = 0; index < argv.length; index += 1) {
        const arg = argv[index];
        if (arg === '--host') {
            parsed.host = readValue(argv, index, arg);
            index += 1;
        }
        else if (arg === '--port') {
            parsed.port = Number(readValue(argv, index, arg));
            if (!Number.isSafeInteger(parsed.port) || parsed.port < 0 || parsed.port > 65535) {
                throw new Error('--port must be an integer from 0 to 65535');
            }
            index += 1;
        }
        else if (arg === '--ui-root') {
            parsed.uiRoot = readValue(argv, index, arg);
            index += 1;
        }
        else if (arg === '--health-project') {
            parsed.healthProject = readValue(argv, index, arg);
            index += 1;
        }
        else if (arg === '--help') {
            console.log(`Usage: ${ASHA_BROWSER_HOST_COMMAND}`);
            process.exit(0);
        }
        else {
            throw new Error(`Unknown argument: ${arg}`);
        }
    }
    return parsed;
}
function readValue(argv, index, flag) {
    const value = argv[index + 1];
    if (value === undefined || value.startsWith('--')) {
        throw new Error(`Missing value for ${flag}`);
    }
    return value;
}
main().catch((error) => {
    console.error(error instanceof Error ? error.message : String(error));
    process.exit(1);
});
//# sourceMappingURL=cli.js.map