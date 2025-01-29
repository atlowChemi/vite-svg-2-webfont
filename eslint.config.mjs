import eslint from '@eslint/js';
import tseslint from 'typescript-eslint';
import eslintConfigPrettier from "eslint-config-prettier";

export default tseslint.config(
  {ignores: ['eslint.config.mjs', 'dist', 'node_modules', 'src/fixtures/*', 'coverage/*', 'example/src/webfont/icons.ts', 'example/dist/*', 'example/vite.config.ts'] },
  eslint.configs.recommended,
  tseslint.configs.recommendedTypeChecked,
  eslintConfigPrettier,
  {
    languageOptions: {
      parserOptions: {
        projectService: true,
        tsconfigRootDir: import.meta.dirname,
      },
    },
  },
  {
    rules: {
      '@typescript-eslint/unbound-method': 'off',
      '@typescript-eslint/no-unsafe-return': 'off',
      '@typescript-eslint/member-delimiter-style': 'off',
      '@typescript-eslint/no-unsafe-member-access': 'off',
    },
  },
  {
    files: ['*.{js,cjs}'],
    ignores: ['*/**/*.{js,cjs}'],
    rules: {
      '@typescript-eslint/no-var-requires': 'off',
    },
  },
  {
    linterOptions: {
      reportUnusedDisableDirectives: true,
    }
  },
);