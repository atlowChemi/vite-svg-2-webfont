import { env } from 'node:process';
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';

const updates = readFileSync(env.RENOVATE_POST_UPGRADE_COMMAND_DATA_FILE, 'utf8').trim();
assert.ok(updates, 'Expected Renovate update data');
const id = createHash('sha256').update(updates).digest('hex');
mkdirSync('.changeset', { recursive: true });
writeFileSync(
    `.changeset/renovate-${id}.md`,
    `---
"@atlowchemi/webfont-generator": patch
"@atlowchemi/webfont-browser-generator": patch
"vite-svg-2-webfont": patch
---

Update dependencies and tooling.
`,
);
