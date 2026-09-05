# Gas2 — PTH 倒立教練

以 **Rust 神經網絡（手寫 MLP，WASM）** 依使用者體能測試結果，規劃個人化 **Press-to-Handstand（PTH）** 12 週訓練路徑的手機網頁。全程在瀏覽器本地運算——**不使用任何 AI 服務**（矩陣運算、前向推論、反向傳播皆為手寫演算法）。

[![Rust CI](https://github.com/end8cl01g/Gas2/actions/workflows/rust.yml/badge.svg)](https://github.com/end8cl01g/Gas2/actions/workflows/rust.yml)
[![Web CI/CD](https://github.com/end8cl01g/Gas2/actions/workflows/web.yml/badge.svg)](https://github.com/end8cl01g/Gas2/actions/workflows/web.yml)

## 線上使用

merge 到 `main` 後自動部署至 **GitHub Pages**：
`https://end8cl01g.github.io/Gas2/`

- 完成約 5 分鐘體能測試（自評＋計時/計數，無攝影機）
- 神經網絡推論 **8 維輸出**：五項能力評分（階段與動作變化式）＋三項劑量參數（工作容量／恢復力／進步速率 → 組數、次數落點、組間休息、每週漸進斜率、減載深度、跨週預計升階）
- 規劃器只保留安全硬約束 → 12 週週期化課表
- 每週回報訓練結果 → 網絡線上微調全部 8 維 → 從下一週起重算課表（疼痛／過難 → 下一週強制減載）並說明原因
- 支援 PWA 離線瀏覽；資料僅存於裝置（localStorage），可匯出/匯入 JSON

## 兩個閉環

```
產品閉環：體能測試 → NN 推論 → 課表 → 訓練 → 週回報 → 線上微調 →（回到課表）
CI 閉環： push → lint/test/build → deploy Pages → 線上煙測(Playwright) → 失敗自動開 issue
```

## 架構

```
┌─ web/ (TypeScript + Preact + Vite, PWA)        手機優先 UI
│    └── src/wasm/  ← wasm-bindgen 產物
├─ crates/wasm/   引擎綁定（assess / recalibrate / 權重載入匯出）
├─ crates/core/   純 Rust 核心
│    ├── nn.rs        手寫 MLP（12→24→12→8）前向＋反向傳播
│    ├── finetune.rs  線上微調（僅輸出層＋雙重安全夾限；評分＋劑量一起微調）
│    ├── planner.rs   課表規劃器（劑量參數 → 組數/次數/休息/斜率/減載深度/升階投影；硬約束）
│    ├── exercises.rs 26 個動作（文字要點＋退階/進階＋次數區間與休息基準）
│    └── weights/baseline.json  離線訓練產生的基線權重
└─ crates/train/  離線訓練器（規則引擎標籤 → 手寫 SGD）
```

### 「神經網絡但不用 AI」的界定

- 網絡為約 700 參數的小型 MLP（12→24→12→8）；權重由 `crates/train` 以**可審計的專家規則**生成標籤（飽和曲線），再以手寫 SGD 訓練
- 基線權重 `include_str!` 內嵌；瀏覽器內只做前向推論與輸出層微調
- 安全設計：微調僅動輸出層；單週每維輸出變動上限 +0.12/−0.15；課表由規劃器硬約束生成（組數 ≤ 基準+2 且 ≤ 6、次數不超出動作區間、休息 40–240 秒、每週斜率 ≤ +10%、累積 ≤ ×1.5、每 4 週 block 最多升一階、第 4/8/12 週減載）

### 神經網絡決定什麼（v2）

| NN 輸出 | 課表上的效果 |
|---|---|
| 5 項能力評分 | 目前階段、每個 block 的動作變化式 |
| 工作容量 | 起始組數係數 ×0.8–1.2、次數落在動作區間的哪一段 |
| 恢復力 | 組間休息 ×0.8–1.3、減載深度 ×0.5–0.7 |
| 進步速率 | 每負荷週漸進 +3%–10%、跨週能力投影 → 後段週次預先呈現升階動作 |

每週回報後 8 維一起微調，課表從「下一週」重算；回報疼痛或「太難＋出席 <50%」→ 下一週強制減載（實際反映在組數上）。

## 開發

```bash
# 依賴：Rust stable + wasm32-unknown-unknown target + wasm-bindgen-cli 0.2.100 + Node 20+

cargo test --workspace                       # Rust 測試
cargo run -p gas2-train                      # 重新訓練基線權重（寫入 crates/core/weights/）
cargo run -p gas2-train -- --check           # 驗證權重回歸門檻
./scripts/build-wasm.sh                      # 建置 wasm 綁定（web + nodejs target）

cd web && npm ci
npm test                                     # vitest（Node 內跑 wasm）
npm run dev                                  # 本地開發
npm run build && npm run preview             # 本地生產預覽
SMOKE_URL=http://127.0.0.1:4173 npm run smoke  # Playwright 煙測
```

## CI/CD

| Workflow | 觸發 | 內容 |
|---|---|---|
| `Rust CI` | PR / push main | fmt、clippy -D warnings、全測試、權重回歸檢查、wasm 編譯 |
| `Web CI/CD` | PR / push main | wasm 建置、typecheck、vitest、vite build；**main → 部署 Pages → 煙測 → 失敗自動開 issue** |

規劃文件見 [`todo.md`](todo.md)。
