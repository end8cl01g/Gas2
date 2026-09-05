// 部署後煙測：對線上 URL（SMOKE_URL，或本地 preview）跑完整產品閉環。
import { expect, test } from '@playwright/test';

test('體測 → 課表 → 週回報 → 課表自動微調', async ({ page }) => {
  // 用相對路徑：baseURL 為 GitHub Pages 子路徑（/Gas2/）時，'/' 會解析到根網域 404；'./' 兩種環境皆正確
  await page.goto('./');

  // 等待 wasm 引擎載入完成（歡迎頁出現）
  const start = page.getByTestId('start-assessment');
  await expect(start).toBeVisible({ timeout: 30_000 });
  await start.click();

  // 精靈：3 個「下一步」＋ 1 個「生成我的課表」（欄位皆有預設值）
  for (let i = 0; i < 3; i++) {
    await page.getByTestId('wizard-next').click();
  }
  await page.getByTestId('wizard-submit').click();

  // 課表出現
  await expect(page.getByTestId('week-title')).toContainText('第 1 週', { timeout: 30_000 });

  // 本週回報 → 送出 → 變更說明出現（微調閉環）
  await page.getByTestId('open-log').click();
  await page.getByTestId('log-submit').click();
  await expect(page.getByTestId('changes-banner')).toBeVisible({ timeout: 30_000 });
  await expect(page.getByTestId('changes-banner')).toContainText('課表已更新');
});
