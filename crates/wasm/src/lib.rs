//! WASM 綁定：瀏覽器端有狀態引擎（載入權重 → 體測 → 回報微調）。
//! JSON 字串跨越邊界（schema 見 gas2-core::model，camelCase）。

use gas2_core::model::{Assessment, Plan, RecalibrateResponse, Scores, WeeklyLog};
use gas2_core::nn::Mlp;
use gas2_core::{finetune, planner, APP_NAME, APP_VERSION, BASELINE_WEIGHTS_JSON};
use wasm_bindgen::prelude::*;

fn to_js_value_err(e: String) -> JsError {
    JsError::new(&e)
}

#[wasm_bindgen]
pub fn app_info() -> String {
    format!("{APP_NAME} v{APP_VERSION}")
}

/// 瀏覽器端引擎：保存當前權重、體測結果與課表。
#[wasm_bindgen]
pub struct Engine {
    nn: Mlp,
    assessment: Option<Assessment>,
    scores: Option<Scores>,
    plan: Option<Plan>,
}

#[wasm_bindgen]
impl Engine {
    /// 以內嵌基線權重建立引擎
    #[wasm_bindgen(constructor)]
    pub fn new() -> Engine {
        let nn = Mlp::from_json(BASELINE_WEIGHTS_JSON).expect("baseline weights valid");
        Engine {
            nn,
            assessment: None,
            scores: None,
            plan: None,
        }
    }
    /// 載入 localStorage 保存的（可能已微調過的）權重
    pub fn load_weights(&mut self, json: &str) -> Result<(), JsError> {
        let nn = Mlp::from_json(json).map_err(to_js_value_err)?;
        self.nn = nn;
        Ok(())
    }

    /// 匯出當前權重（供 localStorage 持久化／匯出備份）
    pub fn export_weights(&self) -> String {
        self.nn.to_json()
    }

    /// 重置回基線權重；若已有體測，則以基線權重重算評分與課表
    pub fn reset_weights(&mut self) {
        self.nn = Mlp::from_json(BASELINE_WEIGHTS_JSON).expect("baseline weights valid");
        if let Some(a) = self.assessment.clone() {
            let scores = Scores::from_array(self.nn.infer(&a.features())).clamped();
            self.plan = Some(planner::build_plan(&a, &scores));
            self.scores = Some(scores);
        }
    }

    pub fn has_assessment(&self) -> bool {
        self.assessment.is_some()
    }

    /// 當前評分 JSON（無體測則回 null）
    pub fn current_scores(&self) -> Option<String> {
        self.scores
            .map(|s| serde_json::to_string(&s).expect("Scores serializes"))
    }

    /// 當前課表 JSON（無體測則回 null）
    pub fn current_plan(&self) -> Option<String> {
        self.plan
            .as_ref()
            .map(|p| serde_json::to_string(p).expect("Plan serializes"))
    }

    /// 體能測試 → 生成個人化 12 週課表（並保存狀態）
    pub fn assess(&mut self, assessment_json: &str) -> Result<String, JsError> {
        let a: Assessment = serde_json::from_str(assessment_json)
            .map_err(|e| to_js_value_err(format!("體測 JSON 不合法: {e}")))?;
        let a = a.sanitized();
        let scores = Scores::from_array(self.nn.infer(&a.features())).clamped();
        let plan = planner::build_plan(&a, &scores);
        let json = serde_json::to_string(&plan).expect("Plan serializes");
        self.assessment = Some(a);
        self.scores = Some(scores);
        self.plan = Some(plan);
        Ok(json)
    }

    /// 每週回報 → 線上微調 → 新課表＋權重＋變更說明
    pub fn recalibrate(&mut self, log_json: &str) -> Result<String, JsError> {
        let a = self
            .assessment
            .ok_or_else(|| JsError::new("尚未完成體能測試"))?;
        let prev = self.scores.ok_or_else(|| JsError::new("尚無能力評分"))?;
        let log: WeeklyLog = serde_json::from_str(log_json)
            .map_err(|e| to_js_value_err(format!("回報 JSON 不合法: {e}")))?;

        let r = finetune::recalibrate(&mut self.nn, &a, &log, &prev);
        let plan = planner::build_plan(&a, &r.new_scores);
        self.scores = Some(r.new_scores);
        self.plan = Some(plan.clone());

        let resp = RecalibrateResponse {
            plan,
            scores: r.new_scores,
            weights: self.nn.to_json(),
            changes: r.changes,
            stage_changed: r.stage_changed,
            force_deload: r.force_deload,
        };
        Ok(serde_json::to_string(&resp).expect("RecalibrateResponse serializes"))
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}
