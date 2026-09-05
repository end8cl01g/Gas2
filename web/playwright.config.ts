import { defineConfig, devices } from '@playwright/test';

// SMOKE_URL：部署後煙測的目標（線上 URL 或本地 preview）
export default defineConfig({
  testDir: 'tests',
  testMatch: /.*\.spec\.ts/,
  timeout: 90_000,
  retries: 1,
  use: {
    baseURL: process.env.SMOKE_URL ?? 'http://127.0.0.1:4173',
  },
  // workflow 以 --project=chromium 執行；先前未定義 projects 導致 Playwright 直接報錯（煙測 2 秒即失敗）
  projects: [{ name: 'chromium', use: { ...devices['Pixel 7'] } }],
  reporter: [['list']],
});
