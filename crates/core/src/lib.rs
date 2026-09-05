//! gas2-core：純 Rust 實作的神經網絡（手寫 MLP）＋ Press-to-Handstand 課表規劃器。
//!
//! 設計原則：
//! - 不依賴任何 AI/ML 框架；矩陣運算、前向推論、反向傳播全部手寫
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
