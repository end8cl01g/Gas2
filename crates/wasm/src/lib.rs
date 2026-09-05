//! WASM 綁定：瀏覽器端有狀態引擎（載入權重 → 體測 → 回報微調）。
//! JSON 字串跨越邊界（schema 見 gas2-core::model，camelCase）。

use gas2_core::model::{Assessment, Plan, Profile, RecalibrateResponse, WeeklyLog};
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

/// 瀏覽器端引擎：保存當前權重、體測結果、神經網絡輸出與課表。
#[wasm_bindgen]
pub struct Engine {
    nn: Mlp,
    assessment: Option<Assessment>,
    profile: Option<Profile>,
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
            profile: None,
            plan: None,
        }
    }
    /// 載入 localStorage 保存的（可能已微調過的）權重；
    /// 舊 schema（輸出維度不符）會回傳錯誤，呼叫端應重置回基線。
    pub fn load_weights(&mut self, json: &str) -> Result<(), JsError> {
        let nn = Mlp::from_json(json).map_err(to_js_value_err)?;
        self.nn = nn;
        Ok(())
    }

    /// 匯出當前權重（供 localStorage 持久化／匯出備份）
    pub fn export_weights(&self) -> String {
        self.nn.to_json()
    }

    /// 重置回基線權重；若已有體測，則以基線權重重算輸出與課表
    pub fn reset_weights(&mut self) {
        self.nn = Mlp::from_json(BASELINE_WEIGHTS_JSON).expect("baseline weights valid");
        if let Some(a) = self.assessment {
            let profile = Profile::from_array(self.nn.infer(&a.features())).clamped();
            self.plan = Some(planner::build_plan(&a, &profile));
            self.profile = Some(profile);
        }
    }

    pub fn has_assessment(&self) -> bool {
        self.assessment.is_some()
    }

    /// 當前能力評分 JSON（無體測則回 null）
    pub fn current_scores(&self) -> Option<String> {
        self.profile
            .map(|p| serde_json::to_string(&p.scores).expect("Scores serializes"))
    }

    /// 當前劑量參數 JSON（無體測則回 null）
    pub fn current_dosing(&self) -> Option<String> {
        self.profile
            .map(|p| serde_json::to_string(&p.dosing).expect("Dosing serializes"))
    }

    /// 當前神經網絡完整輸出 JSON（評分＋劑量；無體測則回 null）
    pub fn current_profile(&self) -> Option<String> {
        self.profile
            .map(|p| serde_json::to_string(&p).expect("Profile serializes"))
    }

    /// 當前課表 JSON（無體測則回 null）
    pub fn current_plan(&self) -> Option<String> {
        self.plan
            .as_ref()
            .map(|p| serde_json::to_string(p).expect("Plan serializes"))
    }

    /// 體能測試 → 神經網絡推論（評分＋劑量）→ 生成個人化 12 週課表（並保存狀態）
    pub fn assess(&mut self, assessment_json: &str) -> Result<String, JsError> {
        let a: Assessment = serde_json::from_str(assessment_json)
            .map_err(|e| to_js_value_err(format!("體測 JSON 不合法: {e}")))?;
        let a = a.sanitized();
        let profile = Profile::from_array(self.nn.infer(&a.features())).clamped();
        let plan = planner::build_plan(&a, &profile);
        let json = serde_json::to_string(&plan).expect("Plan serializes");
        self.assessment = Some(a);
        self.profile = Some(profile);
        self.plan = Some(plan);
        Ok(json)
    }

    /// 每週回報 → 線上微調（8 維輸出）→ 新課表（錨點＝下一週、必要時強制減載）＋權重＋變更說明
    pub fn recalibrate(&mut self, log_json: &str) -> Result<String, JsError> {
        let a = self
            .assessment
            .ok_or_else(|| JsError::new("尚未完成體能測試"))?;
        let prev = self
            .profile
            .ok_or_else(|| JsError::new("尚無神經網絡輸出"))?;
        let log: WeeklyLog = serde_json::from_str(log_json)
            .map_err(|e| to_js_value_err(format!("回報 JSON 不合法: {e}")))?;

        let r = finetune::recalibrate(&mut self.nn, &a, &log, &prev);
        self.profile = Some(r.profile);
        self.plan = Some(r.plan.clone());

        let resp = RecalibrateResponse {
            plan: r.plan,
            scores: r.profile.scores,
            dosing: r.profile.dosing,
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
