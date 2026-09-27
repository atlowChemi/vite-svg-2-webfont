import { cpSync } from 'node:fs';

cpSync(new URL('../crates/webfont-generator/templates/', import.meta.url), new URL('../packages/webfont-generator/templates/', import.meta.url), { recursive: true });
