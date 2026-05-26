#!/usr/bin/env node
import { Buffer } from 'node:buffer';
import { open } from 'node:fs/promises';
import { exit } from 'node:process';
import { inspect, parseArgs, styleText } from 'node:util';

const {
    values: { file = 'package.json' },
    positionals: [version],
} = parseArgs({
    allowPositionals: true,
    options: {
        file: {
            type: 'string',
            short: 'f',
        },
    },
});

if (!version) {
    console.error(styleText('red', 'Usage: set-pnpm-package-manager <version> [--file package.json]'));
    exit(2);
}

if (!/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/.test(version) && version !== 'latest') {
    console.error(styleText('red', `Invalid version: ${version}. Expected a full semver version or "latest".`));
    exit(2);
}

const packageName = `pnpm@${version}`;

try {
    await using handle = await open(file, 'r+');
    const packageJsonRaw = await handle.readFile('utf8');
    const packageJson = JSON.parse(packageJsonRaw);

    const metadataUrl = `https://registry.npmjs.org/pnpm/${encodeURIComponent(version)}`;
    const response = await fetch(metadataUrl);

    if (response.status === 404) {
        throw new Error(`Version not found in npm registry: ${packageName}`);
    }

    if (!response.ok) {
        throw new Error(`Could not resolve ${packageName}: ${response.status} ${response.statusText}`);
    }

    const metadata = await response.json();
    const integrity = metadata?.dist?.integrity;
    const resolvedVersion = version === 'latest' ? metadata.version : version;
    const resolvedPackageName = `pnpm@${resolvedVersion}`;

    if (typeof resolvedVersion !== 'string') {
        throw new Error(`Missing resolved version for ${packageName}`);
    }

    if (typeof integrity !== 'string') {
        throw new Error(`Missing dist.integrity for ${resolvedPackageName}`);
    }

    const match = integrity.match(/^([a-z0-9]+)-(.+)$/i);

    if (!match) {
        throw new Error(`Invalid integrity format for ${resolvedPackageName}: ${integrity}`);
    }

    const [, algorithm, base64Digest] = match;
    const hexDigest = Buffer.from(base64Digest!, 'base64').toString('hex');
    const packageManager = `${resolvedPackageName}+${algorithm}.${hexDigest}`;

    packageJson.packageManager = packageManager;

    const indent = detectIndent(packageJsonRaw);
    const trailingNewline = packageJsonRaw.endsWith('\n') ? '\n' : '';

    const nextFileContent = `${JSON.stringify(packageJson, null, indent)}${trailingNewline}`;

    await handle.write(nextFileContent, 0, 'utf8');
    await handle.truncate(Buffer.byteLength(nextFileContent, 'utf8'));

    console.log(styleText('green', 'Updated packageManager:'));
    console.log(packageManager);
} catch (error) {
    console.error(styleText('red', 'update failed: '), inspect(error, { colors: true, compact: false }));
    exit(1);
}

function detectIndent(source: string): string | number {
    const match = source.match(/^[ \t]+"/m);
    return match ? match[0].slice(0, -1) : 4;
}
