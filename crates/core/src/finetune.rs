//! 線上微調（決策 §6-1b）：用每週回報產生偽標籤，只微調輸出層。
//!
//! 三道安全閘：
//! 1. 只更新輸出層（隱層凍結）
//! 2. 每週每項評分變動上限：上調 +0.12、下調 −0.15
//! 3. 評分本身夾限 [0.02, 0.98]，課表由規劃器硬約束生成

use crate::model::{
    Assessment, ChangeNote, Focus, PainArea, Scores, WeeklyLog, INPUT_FEATURES, OUTPUT_SCORES,
};
use crate::nn::Mlp;
use crate::planner::{self, STAGES};

/// 輸出層微調是固定隱層下的凸問題，可用較大學習率；
/// 早停條件：MSE 低於 eps（收斂）或達步數上限。
const FINETUNE_LR: f32 = 0.3;
const FINETUNE_STEPS: usize = 400;
const FINETUNE_EPS: f32 = 2e-5;
const MAX_UP: f32 = 0.12;
const MAX_DOWN: f32 = 0.15;
const SCORE_FLOOR: f32 = 0.02;
const SCORE_CEIL: f32 = 0.98;

pub struct Recalibration {
    pub new_scores: Scores,
    pub stage: u8,
    pub stage_changed: bool,
    pub force_deload: bool,
    pub changes: Vec<ChangeNote>,
}

fn target_delta(log: &WeeklyLog) -> [f32; OUTPUT_SCORES] {
    let adherence = log.adherence();
    let base = match log.focus {
        Focus::TooEasy => {
            if adherence >= 0.8 {
                0.06
            } else {
                0.03
            }
        }
        Focus::Ok => 0.01,
        Focus::TooHard => {
            if adherence < 0.5 {
                -0.08
            } else {
                -0.06
            }
        }
    };
    let mut delta = [base; OUTPUT_SCORES];
    for p in &log.pain {
        match p {
            PainArea::Wrist => {
                delta[3] -= 0.05; // 上肢推撐
                delta[4] -= 0.05; // 壓撐爆發
            }
            PainArea::Shoulder => {
                delta[3] -= 0.06;
                delta[2] -= 0.03; // 倒立平衡
                delta[0] -= 0.03;
            }
            PainArea::LowerBack => {
                delta[1] -= 0.06; // 核心控制
            }
        }
    }
    delta
}

/// 以一週回報微調網絡，並產出人話說明。
pub fn recalibrate(
    nn: &mut Mlp,
    assessment: &crate::model::Assessment,
    log: &WeeklyLog,
    prev: &Scores,
) -> Recalibration {
    let log = log.sanitized();
    let adherence = log.adherence();
    let delta = target_delta(&log);
    let prev_arr = prev.to_array();

    // 1. 產生偽標籤（微調目標）
    let mut target = [0.0f32; OUTPUT_SCORES];
    for i in 0..OUTPUT_SCORES {
        target[i] = (prev_arr[i] + delta[i]).clamp(SCORE_FLOOR, SCORE_CEIL);
    }

    // 2. 只微調輸出層（凸問題＋早停）
    let x: [f32; INPUT_FEATURES] = assessment.sanitized().features();
    for _ in 0..FINETUNE_STEPS {
        let mse = nn.train_step_output_layer(&x, &target, FINETUNE_LR);
        if mse < FINETUNE_EPS {
            break;
        }
    }

    // 3. 重新推論並套用安全夾限
    let raw = nn.infer(&x);
    let mut out = [0.0f32; OUTPUT_SCORES];
    let mut clamped_any = false;
    for i in 0..OUTPUT_SCORES {
        let lo = (prev_arr[i] - MAX_DOWN).max(SCORE_FLOOR);
        let hi = (prev_arr[i] + MAX_UP).min(SCORE_CEIL);
        let v = raw[i].clamp(lo, hi);
        if (v - raw[i]).abs() > 1e-6 {
            clamped_any = true;
        }
        out[i] = v;
    }
    let new_scores = Scores::from_array(out).clamped();

    // 4. 階段與減載判定（硬約束）
    let old_stage = planner::stage_for(prev);
    let stage = planner::stage_for(&new_scores);
    let stage_changed = stage != old_stage;
    let force_deload = !log.pain.is_empty() || (log.focus == Focus::TooHard && adherence < 0.5);

    // 5. 說明「改了什麼、為什麼」
    let mut changes = Vec::new();
    changes.push(ChangeNote {
        kind: "focus".into(),
        message_zh: match log.focus {
            Focus::TooEasy => format!(
                "本週回報太輕鬆（出席率 {:.0}%）→ 五項能力評分上調，下週訓練量與動作難度隨之提高",
                adherence * 100.0
            ),
            Focus::Ok => format!(
                "本週節奏剛好（出席率 {:.0}%）→ 維持路徑，評分微幅上調",
                adherence * 100.0
            ),
            Focus::TooHard => format!(
                "本週太難（出席率 {:.0}%）→ 下調強度、退階動作變化式",
                adherence * 100.0
            ),
        },
    });
    for p in &log.pain {
        changes.push(ChangeNote {
            kind: "pain".into(),
            message_zh: format!(
                "⚠️ {}不適 → 相關項目評分下調，並移除高負荷支撐類動作（退階替代）",
                p.zh()
            ),
        });
    }
    if stage_changed {
        changes.push(ChangeNote {
            kind: "stage".into(),
            message_zh: format!(
                "階段調整：{} → {}（{}）",
                STAGES[old_stage as usize].name_zh,
                STAGES[stage as usize].name_zh,
                STAGES[stage as usize].goal_zh
            ),
        });
    }
    if force_deload {
        changes.push(ChangeNote {
            kind: "deload".into(),
            message_zh: "下週強制減載：組數下調 40%，優先恢復".into(),
        });
    }
    if clamped_any {
        changes.push(ChangeNote {
            kind: "safety".into(),
            message_zh: format!(
                "已套用安全夾限：單週每項評分變動上限 +{MAX_UP:.2} / −{MAX_DOWN:.2}"
            ),
        });
    }

    Recalibration {
        new_scores,
        stage,
        stage_changed,
        force_deload,
        changes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Assessment;
    use crate::nn::Mlp;

    fn assessment() -> Assessment {
        Assessment {
            shoulder_mobility: 3,
            wrist_mobility: 2,
            plank_sec: 60,
            hollow_sec: 30,
            pushup_reps: 15,
            pike_pushup_reps: 5,
            wall_walk_reps: 2,
            wall_hs_hold_sec: 15,
            wall_hspu_reps: 0,
            bodyweight_kg: 68.0,
            height_cm: 172.0,
            days_per_week: 3,
            experience: 1,
        }
    }

    fn log(focus: Focus, pain: Vec<PainArea>, done: u8, planned: u8) -> WeeklyLog {
        WeeklyLog {
            week_index: 1,
            sessions_completed: done,
            sessions_planned: planned,
            focus,
            pain,
            notes: None,
        }
    }

    #[test]
    fn too_easy_raises_scores_within_cap() {
        let mut nn = Mlp::from_json(crate::BASELINE_WEIGHTS_JSON).unwrap();
        let prev = Scores::from_array(nn.infer(&assessment().features())).clamped();
        let r = recalibrate(
            &mut nn,
            &assessment(),
            &log(Focus::TooEasy, vec![], 3, 3),
            &prev,
        );
        let p = prev.to_array();
        let n = r.new_scores.to_array();
        for i in 0..OUTPUT_SCORES {
            assert!(n[i] > p[i], "太輕鬆應上調第 {i} 項");
            assert!(
                n[i] - p[i] <= MAX_UP + 1e-4,
                "上調不得超過上限: {} -> {}",
                p[i],
                n[i]
            );
        }
        assert!(!r.force_deload);
        assert!(!r.changes.is_empty());
    }

    #[test]
    fn pain_forces_deload_and_lowers_scores() {
        let mut nn = Mlp::from_json(crate::BASELINE_WEIGHTS_JSON).unwrap();
        let prev = Scores::from_array(nn.infer(&assessment().features())).clamped();
        let r = recalibrate(
            &mut nn,
            &assessment(),
            &log(Focus::Ok, vec![PainArea::Shoulder], 3, 3),
            &prev,
        );
        assert!(r.force_deload);
        assert!(r.new_scores.overhead_press < prev.overhead_press);
        assert!(r
            .changes
            .iter()
            .any(|c| c.kind == "pain" && c.message_zh.contains("肩膀")));
    }

    #[test]
    fn too_hard_lowers_scores() {
        let mut nn = Mlp::from_json(crate::BASELINE_WEIGHTS_JSON).unwrap();
        let prev = Scores::from_array(nn.infer(&assessment().features())).clamped();
        let r = recalibrate(
            &mut nn,
            &assessment(),
            &log(Focus::TooHard, vec![], 1, 4),
            &prev,
        );
        let p = prev.to_array();
        let n = r.new_scores.to_array();
        for i in 0..OUTPUT_SCORES {
            assert!(n[i] < p[i], "太難應下調第 {i} 項");
            assert!(p[i] - n[i] <= MAX_DOWN + 1e-4);
        }
        assert!(r.force_deload, "低出席＋太難應強制減載");
    }

    #[test]
    fn weights_stay_valid_after_finetune() {
        let mut nn = Mlp::from_json(crate::BASELINE_WEIGHTS_JSON).unwrap();
        let prev = Scores::from_array(nn.infer(&assessment().features())).clamped();
        recalibrate(
            &mut nn,
            &assessment(),
            &log(Focus::TooEasy, vec![], 3, 3),
            &prev,
        );
        let json = nn.to_json();
        let back = Mlp::from_json(&json).expect("微調後權重應仍為合法 JSON/結構");
        assert_eq!(back.arch, nn.arch);
    }
}
