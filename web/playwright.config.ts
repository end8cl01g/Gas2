import { defineConfig, devices } from '@playwright/test';

// SMOKE_URL：部署後煙測的目標（線上 URL 或本地 preview）
export default defineConfig({
  testDir: 'tests',
  testMatch: /.*\.spec\.ts/,
  timeout: 90_000,
  retries: 1,
  use: {
    baseURL: process.env.SMOKE_URL ?? 'http://127.0.0.1:4173',
    ...devices['Pixel 7'],
  },
  reporter: [['list']],
});
