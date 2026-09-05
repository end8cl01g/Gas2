//! gas2-core：純 Rust 實作的神經網絡（手寫 MLP）＋ Press-to-Handstand 課表規劃器。
//!
//! 設計原則：
//! - 不依賴任何 AI/ML 框架；矩陣運算、前向推論、反向傳播全部手寫
//! - 神經網絡輸出 8 維：5 項能力評分（決定階段與動作）＋ 3 項劑量參數
//!   （工作容量／恢復力／進步速率 → 組數、次數、休息、漸進斜率、減載深度、跨週升階投影）
//! - 規劃器只保留安全硬約束（組數／次數／休息上下限、每 block 最多升一階、排程與強制減載）
//! - 基線權重由 `gas2-train` 以規則引擎產生的資料離線訓練，`include_str!` 內嵌
//! - 線上微調（越用越準）只更新輸出層，並受安全夾限約束

pub mod exercises;
pub mod finetune;
pub mod model;
pub mod nn;
pub mod planner;

/// 離線訓練產生的基線權重（由 `cargo run -p gas2-train` 重新生成）
pub const BASELINE_WEIGHTS_JSON: &str = include_str!("../weights/baseline.json");

pub const APP_NAME: &str = "gas2-core";
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
