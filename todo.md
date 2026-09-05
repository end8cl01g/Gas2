# TODO — Press to Handstand 個人化訓練網頁（Rust + TypeScript）

> 規劃方法：**從最終結果出發逆向推導**——先定義「完成時看到的東西」，再一層層反推前置條件，直到今天還不存在的東西。
> 狀態圖示：⬜ 未開始　🔶 進行中　✅ 完成
> 🔒 §6 規格已凍結（2026-09-05），實作進行中。
>
> **目前唯一阻塞（2026-09-05）**：GitHub App 權限缺 `workflows`，無法推送 `.github/workflows/*`。
> 已完成：全部程式碼（Rust 核心/訓練器/WASM 綁定/Preact PWA）＋本地驗證（tsc 0 錯誤、vitest 5/5、build 通過）。
> 待使用者重連 GitHub 後：推 workflows → CI 編譯測試 → 匯出 toolchain → 訓練真權重 → merge → Pages 部署＋線上煙測。

---

## 0. 最終結果（北極星，先定義「什麼叫做完」）

一支部署在 GitHub Pages 的手機網頁：

1. 使用者手機打開網頁，完成約 5 分鐘**體能測試**（自評＋計時/計數項，不需攝影機）
2. **Rust 演算法神經網絡**（手寫 MLP，編譯成 WASM，100% 瀏覽器本地運算，**不呼叫任何 AI 服務**）推論出個人化的 Press to Handstand 訓練路徑
3. 顯示週期化課表：階段／動作／組數次數／退階與進階條件
4. **產品閉環**：使用者每週回報訓練結果 → 重新推論 → 課表自動微調並說明「改了什麼、為什麼」
5. **CI 閉環**：每次 git push 自動 lint → test → build → deploy → 部署後煙測 → 失敗自動回饋開 issue；CI 全綠＝結果可交付

---

## 1. 逆向路徑圖（結果 ← 一層層反推前置）

```
L6 結果驗收：手機開頁 → 5 分鐘體測 → 12 週課表；每週回饋後自動微調；離線可用
   ↑ 需要
L5 部署閉環：Actions push → build → deploy Pages → 煙測線上 URL → 失敗重跑/開 issue
   ↑ 需要
L4 產品閉環：體測 UI → WASM 推論 → 課表渲染 → 進度記錄(localStorage) → 重算
   ↑ 需要
L3 前端層：TypeScript + Vite + PWA，呼叫 WASM API
   ↑ 需要
L2 橋接層：wasm-pack + wasm-bindgen，型別安全（體測 JSON ⇄ 課表 JSON）
   ↑ 需要
L1 核心層：純 Rust 神經網絡（前向推論＋路徑規劃器後處理）＋ cargo test
   ↑ 需要
L0 腳手架：cargo workspace、Vite TS 模板、Actions workflow、CI 綠燈基線
```

**兩個環的定義**
- 產品閉環：測 → 練 → 回報 → 重算（在網頁內）
- CI 閉環：提交 → 測試 → 部署 → 煙測 → 回饋（在 GitHub 內）

---

## 2. 技術選型（草案，隨 §6 回應凍結）

| 層 | 選擇 | 理由 |
|---|---|---|
| 核心 | 純 Rust 手寫 MLP（矩陣運算自己寫） | 「用演算法不用 AI」：可測、可審計、可編譯 WASM、零外部依賴 |
| 權重 | 規則引擎生成標籤資料 → 離線訓練腳本（手寫梯度下降）→ 權重 embed 進 binary | 版控可重現；線上零 AI 依賴 |
| 橋接 | wasm-pack + wasm-bindgen | 業界標準 |
| 前端 | Vite + TypeScript（vanilla 或 Preact，待 §6 確認） | 手機網頁、體積小 |
| 部署 | GitHub Pages + GitHub Actions | 免費、閉環全自動 |
| 測試 | cargo test + vitest + Playwright | 三層品質門 |

**神經網絡範圍界定**：小型多層感知器（例：12 體測輸入 → 16 → 8 → 輸出能力評分/所處階段），權重由規則式資料離線訓練後硬編碼；線上只做**前向推論**，再交給**規劃器**做安全約束後處理（不跳級、漸進超載、退階優先）。「不用 AI」＝不用任何外部 AI API／機器學習框架，演算法全部手寫。

---

## 3. 里程碑與任務

### M0 需求確認 ✅
- [x] 使用者回應 §6 待確認問題（2026-09-05：第 1 題選 b，其餘授權以機會成本法決策）
- [x] 凍結 v1 規格（體測項目、課表結構、週數）

### M1 腳手架 ✅（2026-09-05）
- [x] cargo workspace：`crates/core`（演算法）、`crates/wasm`（綁定）、`crates/train`（離線訓練）
- [x] `web/`：Vite + TypeScript + Preact 前端（zh-TW、手機優先、深色主題）
- [x] GitHub Actions 三件套：rust.yml（fmt+clippy+test+權重檢查）、web.yml（build→deploy→煙測→失敗開 issue）、toolchain-export.yml（沙盒開發用）
  - ⚠️ workflow 檔已寫好於 `.github/workflows/`，**因 GitHub App 缺 `workflows` 權限暫未推送**，待重連 GitHub 後補上
- [x] wasm 建置腳本 `scripts/build-wasm.sh`（web + nodejs 雙 target）

### M2 Rust 核心 ✅（程式碼＋測試完成；沙盒無 Rust 編譯環境，首次編譯由 CI 驗證）
- [x] 體測輸入 schema（輸入資料模型，13 欄位 → 12 維標準化特徵）
- [x] 課表輸出 schema（Plan/PlanWeek/Session/Block/Prescription，camelCase JSON）
- [x] MLP 前向推論（手寫矩陣乘法 12→16→8→5，ReLU/Sigmoid）＋完整反向傳播
- [x] 離線訓練腳本：規則引擎標籤（飽和曲線＋體重懲罰）→ 手寫 SGD（400 epochs）→ 權重 embed
  - 權重檔暫為未訓練佔位（trained=false，CI 的 --check 會跳過）；待 toolchain 匯出後本地訓練真權重
- [x] PTH 課表規劃器（5 階段門檻推進、2–5 次訓練夾限、第 4/8/12 週減載、退階/進階）
- [x] cargo test：邊界、課表合理性、微調夾限、端到端流程（19＋測試）

### M3 WASM 橋接 ✅（程式碼完成；CI 驗證）
- [x] wasm-bindgen API：`assess`、`recalibrate`、`load_weights/export_weights/reset_weights`
- [x] TS 型別對應（web/src/types.ts ↔ Rust model，camelCase）
- [x] CI 內 Node 端整合測試（wasm-pack 改用 wasm-bindgen-cli GitHub Releases，因沙盒僅 GitHub/npm 可達）

### M4 前端 ✅（2026-09-05 本地驗證通過：tsc 零錯誤、vitest 5/5、vite build 成功 13KB gzip）
- [x] 手機優先 UI（深色主題、大觸控目標、safe-area、320px 起步）
- [x] 體測流程：4 步精靈（關於你 → 活動度 → 支撐與推力 → 倒立專項），全欄位預設值＋測試方式提示
- [x] 課表顯示：能力評分條、週次選擇、階段目標、訓練卡（要點/退階/進階）、減載標記
- [x] localStorage 進度＋每週回報 UI（出席、難度、疼痛、備註）
- [x] PWA：manifest + 手寫 service worker（stale-while-revalidate、離線看課表）＋圖示

### M5 產品閉環 ✅（實作完成）
- [x] 每週回報 → `recalibrate`（只微調輸出層；評分變動夾限 +0.12/−0.15；疼痛→強制減載）→ 新課表＋「改了什麼、為什麼」清單
- [x] 匯出/匯入 JSON（含微調後權重）＋重置網絡

### M6 CI/CD 閉環 🔶（阻塞：GitHub 權限）
- [x] workflow 檔案撰寫完成（rust.yml / web.yml / toolchain-export.yml）
- [ ] **等待使用者重連 GitHub（需 workflows 權限）** → 推送 workflows
- [ ] CI 首次編譯驗證 → toolchain 匯出 → 本地訓練真權重 → 提交
- [ ] merge main：自動部署 Pages → 線上煙測（Playwright）→ 失敗自動開 issue
- [ ] README 徽章生效

### M7 驗收 ⬜（待 M6 解鎖後執行）
- [ ] 手機實機完整走一遍 L1→L6
- [ ] Lighthouse：Performance ≥ 90、PWA 可安裝
- [ ] 斷網測試：離線仍可查看課表
- [ ] 閉環示範：改一行程式 → CI 全綠 → 線上自動更新

---

## 4. 風險與對策
| 風險 | 對策 |
|---|---|
| WASM + Pages 子路徑（base path）設定踩雷 | M1 就用空專案驗證部署，不等最後才試 |
| NN 推薦不合理／不安全 | 規劃器硬約束後處理：漸進超載上限、退階優先、白名單動作庫 |
| 範圍蔓延（攝影機姿態估計、帳號、後端） | v1 凍結：無攝影機、無帳號、無後端、無伺服器 |
| 神經網絡被誤解為「接 AI 服務」 | 全手寫演算法＋測試覆蓋，README 說明權重來源 |

---

## 5. 依賴與工具鏈
Rust stable、wasm-pack、wasm32-unknown-unknown target、Node LTS、Playwright（CI 用）。

---

## 6. 規格決策（已凍結 ✅ 2026-09-05 — 方法：機會成本法，除非另有標註）
| # | 問題 | 決策 | 機會成本理由（放棄了什麼 / 換到什麼） |
|---|---|---|---|
| 1 | 神經網絡定位 | **b) 線上輕量微調**（使用者指定） | 放棄「實作簡單、行為永遠不變」，換取越用越準；微調風險用三道閘壓低：只微調最後一層、每週每項分數變動上限 ±0.15、規劃器硬約束後處理 |
| 2 | 體測項目 | 自評＋計時/計數（無攝影機），8 類項目全採 | 放棄攝影機姿態估計的客觀度（開發/隱私/裝置相容成本遠超收益），換取 5 分鐘即可完成、零隱私風險 |
| 3 | 課表長度 | 12 週（第 4/8 週減載、第 12 週重測） | 8 週太短、體驗不到微調閉環價值；16 週的規劃在無回饋下陳舊風險高；12 週＋每週重算是資訊新鮮度與規劃深度的最優交換 |
| 4 | 前端 | Preact | 放棄 vanilla TS 的極簡（~4KB 差距），換取狀態型 UI（測試精靈/課表/回報/diff）大幅省下的開發時間與較低 bug 面 |
| 5 | UI 語言 | 繁體中文（字串集中，保留日後 i18n 空間） | 放棄雙語的觸及面（目前無此需求），換取內容製作與維護成本減半 |
| 6 | 部署 | GitHub Pages + GitHub Actions | 放棄其他 PaaS 的附加功能，換取與 CI 閉環同平台、零額外供應商依賴 |
| 7 | 動作庫 | 文字要點＋退階/進階說明（無圖片） | 放棄圖片的直觀性（製作/版權/體積成本高），文字承載約九成教學價值 |

---

## 7. 明確不做（v1）
攝影機/視覺姿態估計、使用者帳號與雲端同步、後端伺服器、任何外部 AI API、社交/分享功能。
