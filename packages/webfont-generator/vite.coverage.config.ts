import { dirname, resolve } from 'node:path';
import { defineConfig } from 'vite-plus';
import project from './vite.config';

const output = process.env.WEBFONT_NATIVE_COVERAGE_DIR;
if (!output) throw new Error('Run the test:coverage:native task to configure the instrumented addon.');
const packageRoot = import.meta.dirname;
const binding = resolve(packageRoot, 'binding.js');

export default defineConfig({
    plugins: [
        {
            name: 'instrumented-webfont-binding',
            enforce: 'pre',
            resolveId(source, importer) {
                if (importer && source.startsWith('.') && resolve(dirname(importer.split('?')[0]), source) === binding) {
                    return resolve(output, 'addon/binding.js');
                }
                return null;
            },
        },
    ],
    test: {
        ...project.test,
        root: packageRoot,
        pool: 'threads',
        reporters: [
            'default',
            [
                'junit',
                {
                    outputFile: resolve(output, 'junit.xml'),
                    suiteName: 'napi-vitest-instrumented',
                    // Match normal test runs; the upload flag identifies native instrumentation.
                    classnameTemplate: '{displayName}::{filename}',
                    addFileAttribute: true,
                },
            ],
        ],
        coverage: {
            provider: 'v8',
            enabled: true,
            reportOnFailure: true,
            reporter: ['text', 'lcov'],
            reportsDirectory: resolve(output, 'js'),
            include: ['index.js', 'validations.js', 'templates.js'],
        },
    },
});
