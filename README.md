# Gas2 — PTH 倒立教練

以 **Rust 神經網絡（手寫 MLP，WASM）** 依使用者體能測試結果，規劃個人化 **Press-to-Handstand（PTH）** 12 週訓練路徑的手機網頁。全程在瀏覽器本地運算——**不使用任何 AI 服務**（矩陣運算、前向推論、反向傳播皆為手寫演算法）。

[![Rust CI](https://github.com/end8cl01g/Gas2/actions/workflows/rust.yml/badge.svg)](https://github.com/end8cl01g/Gas2/actions/workflows/rust.yml)
[![Web CI/CD](https://github.com/end8cl01g/Gas2/actions/workflows/web.yml/badge.svg)](https://github.com/end8cl01g/Gas2/actions/workflows/web.yml)

## 線上使用

merge 到 `main` 後自動部署至 **GitHub Pages**：
`https://end8cl01g.github.io/Gas2/`

- 完成約 5 分鐘體能測試（自評＋計時/計數，無攝影機）
- 神經網絡推論五項能力評分 → 規劃器產出 12 週週期化課表
- 每週回報訓練結果 → 網絡線上微調 → 課表自動調整並說明原因
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
│    ├── nn.rs        手寫 MLP（12→24→12→5）前向＋反向傳播
│    ├── finetune.rs  線上微調（僅輸出層＋雙重安全夾限）
│    ├── planner.rs   課表規劃器（階段門檻、漸進超載、強制減載）
│    ├── exercises.rs 27 個動作（文字要點＋退階/進階）
│    └── weights/baseline.json  離線訓練產生的基線權重
└─ crates/train/  離線訓練器（規則引擎標籤 → 手寫 SGD）
```

### 「神經網絡但不用 AI」的界定

- 網絡為約 700 參數的小型 MLP（12→24→12→5）；權重由 `crates/train` 以**可審計的專家規則**生成標籤（飽和曲線），再以手寫 SGD 訓練
- 基線權重 `include_str!` 內嵌；瀏覽器內只做前向推論與輸出層微調
- 安全設計：微調僅動輸出層；單週每項評分變動上限 +0.12/−0.15；課表由規劃器硬約束生成（不跳級、每 4 週減載）

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
