import { defineProject } from 'vite-plus';

export default defineProject({
    run: {
        tasks: {
            build: { cache: false, command: 'node scripts/build.mjs' },
            check: {
                cache: false,
                command: 'cargo clippy -p webfont-browser --target wasm32-unknown-unknown --locked -- -D warnings && cargo fmt --all -- --check',
                dependsOn: ['build'],
            },
            test: { cache: false, command: 'vp test --root ../.. --project=webfont-wasm-browser --project=webfont-browser-build', dependsOn: ['check'] },
        },
    },
    test: { include: ['tests/bundle.test.ts'], benchmark: { include: [] }, name: 'webfont-browser-build', testTimeout: 30_000 },
});
