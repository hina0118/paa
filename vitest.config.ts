import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';
import path from 'path';

export default defineConfig({
  plugins: [react()],
  test: {
    globals: true,
    environment: 'jsdom',
    setupFiles: './src/test/setup.ts',
    // afterEach が setupFiles で実行される際の「failed to find the runner」を回避
    // https://github.com/vitest-dev/vitest/issues/7465
    sequence: {
      hooks: 'list',
      setupFiles: 'list',
    },
    include: ['src/**/*.{test,spec}.{js,ts,jsx,tsx}'],
    exclude: [
      'node_modules',
      'dist',
      'build',
      'tests/e2e/**',
      'tests/e2e-tauri/**',
    ],
    css: true,
    coverage: {
      provider: 'v8',
      reporter: ['text', 'json', 'html', 'lcov'],
      exclude: [
        'node_modules/',
        'src/test/',
        '**/*.config.{js,ts}',
        '**/dist/**',
        '**/build/**',
        '**/*.d.ts',
        '**/vite-env.d.ts',
      ],
      // カバレッジ閾値 85%
      thresholds: {
        lines: 85,
        functions: 85,
        branches: 85,
        statements: 85,
      },
    },
  },
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },
});
