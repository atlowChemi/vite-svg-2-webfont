import { defineProject } from 'vite-plus';

export default defineProject({
    run: {
        tasks: {
            check: {
                command: 'cargo clippy -- -D warnings && cargo fmt -- --check',
            },
            test: {
                command: 'cargo t',
                dependsOn: ['check'],
                env: ['UPDATE_SVG_FIXTURES'],
            },
            build: {
                command: 'napi build --platform --esm --js binding.js --dts binding.d.ts',
            },
            'build:release': {
                command: 'napi build --platform --esm --js binding.js --dts binding.d.ts --release',
                dependsOn: ['test'],
            },
        },
    },
    test: {
        experimental: {
            fsModuleCache: true,
        },
        projects: [
            {
                test: {
                    name: 'webfont-generator',
                    include: ['tests/**/*.test.ts'],
                    benchmark: { include: [] },
                },
            },
        ],
    },
});
