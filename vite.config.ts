import { defineConfig } from 'vitest/config';
import type { UserConfig } from 'vite';

const config: UserConfig = defineConfig({
    test: {
        ...(process.env.CI_TEST_REPORT
            ? {
                  reporters: [
                      'default',
                      [
                          'junit',
                          {
                              outputFile: process.env.CI_TEST_REPORT,
                              suiteName: process.env.CI_TEST_SUITE ?? 'webfont-tests',
                              // Keep test identities stable across environments; upload flags identify the CI matrix.
                              classnameTemplate: '{displayName}::{filename}',
                              addFileAttribute: true,
                          },
                      ],
                  ],
              }
            : {}),
        fsModuleCache: true,
        coverage: {
            provider: 'v8',
            reporter: ['text', 'lcov'],
            reportOnFailure: true,
            exclude: ['**/*.test-d.ts', 'packages/example/**', 'scripts/fixtures/**', 'packages/vite-svg-2-webfont/src/fixtures/**', 'packages/webfont-generator/binding.*'],
        },
        projects: [
            'packages/!(example)/vite.config.ts',
            'packages/!(example)/vite.browser.config.ts',
            {
                test: {
                    name: 'scripts',
                    include: ['scripts/**/*.test.ts'],
                    // Benchmark discovery has its own default include, independent of test.include.
                    benchmark: { include: [] },
                    testTimeout: 30_000,
                    hookTimeout: 30_000,
                },
            },
            {
                test: {
                    name: 'compat',
                    include: ['tests/**/*.compat.test.ts'],
                    benchmark: { include: ['tests/**/*.bench.ts'] },
                },
            },
        ],
    },
});
export default config;
