//! 線上微調（決策 §6-1b）：用每週回報產生偽標籤，只微調輸出層。
//!
//! 微調對象是神經網絡的全部 8 維輸出：
//! - 能力評分 5 維：太輕鬆上調、太難下調、疼痛部位相關項目下調
//! - 劑量參數 3 維：太輕鬆 → 工作容量／進步速率上調；太難／低出席 → 下調；疼痛 → 恢復力下調
//!
//! 三道安全閘：
//! 1. 只更新輸出層（隱層凍結）
//! 2. 每週每維變動上限：上調 +0.12、下調 −0.15
//! 3. 輸出本身夾限 [0.02, 0.98]，課表由規劃器硬約束生成

use crate::model::{
    Assessment, ChangeNote, Focus, PainArea, Plan, Profile, WeeklyLog, ABILITY_DIMS,
    INPUT_FEATURES, OUTPUT_DIMS,
};
use crate::nn::Mlp;
use crate::planner::{self, PlanOptions, STAGES};

/// 輸出層微調是固定隱層下的凸問題，可用較大學習率；
/// 早停條件：MSE 低於 eps（收斂）或達步數上限。
const FINETUNE_LR: f32 = 0.3;
const FINETUNE_STEPS: usize = 400;
const FINETUNE_EPS: f32 = 2e-5;
pub const MAX_UP: f32 = 0.12;
pub const MAX_DOWN: f32 = 0.15;
const SCORE_FLOOR: f32 = 0.02;
const SCORE_CEIL: f32 = 0.98;

// 輸出索引（與 model::OUTPUT_KEYS 對齊）
const I_BASE_PUSH: usize = 0;
const I_CORE: usize = 1;
const I_BALANCE: usize = 2;
const I_PRESS: usize = 3;
const I_COMPRESSION: usize = 4;
const I_WORK: usize = 5;
const I_RECOVERY: usize = 6;
const I_PROGRESSION: usize = 7;

pub struct Recalibration {
    pub profile: Profile,
    pub plan: Plan,
    pub stage: u8,
    pub stage_changed: bool,
    pub force_deload: bool,
    pub changes: Vec<ChangeNote>,
}

fn target_delta(log: &WeeklyLog) -> [f32; OUTPUT_DIMS] {
    let adherence = log.adherence();
    let mut delta = [0.0f32; OUTPUT_DIMS];
    match log.focus {
        Focus::TooEasy => {
            let a = if adherence >= 0.8 { 0.06 } else { 0.03 };
            delta[..ABILITY_DIMS].fill(a);
            delta[I_WORK] = 0.06;
            delta[I_PROGRESSION] = 0.05;
            delta[I_RECOVERY] = 0.03;
        }
        Focus::Ok => {
            delta[..ABILITY_DIMS].fill(0.01);
            delta[I_WORK] = 0.01;
            delta[I_PROGRESSION] = 0.01;
            // 剛好但出席率低 → 容量略降（實際可承受的訓練量比計畫少）
            if adherence < 0.6 {
                delta[I_WORK] = -0.03;
            }
        }
        Focus::TooHard => {
            let a = if adherence < 0.5 { -0.08 } else { -0.06 };
            delta[..ABILITY_DIMS].fill(a);
            delta[I_WORK] = -0.08;
            delta[I_PROGRESSION] = -0.08;
            delta[I_RECOVERY] = -0.04;
        }
    }
    for p in &log.pain {
        match p {
            PainArea::Wrist => {
                delta[I_PRESS] -= 0.05; // 上肢推撐
                delta[I_COMPRESSION] -= 0.05; // 壓撐爆發
            }
            PainArea::Shoulder => {
                delta[I_PRESS] -= 0.06;
                delta[I_BALANCE] -= 0.03; // 倒立平衡
                delta[I_BASE_PUSH] -= 0.03;
            }
            PainArea::LowerBack => {
                delta[I_CORE] -= 0.06; // 核心控制
            }
        }
        // 任何疼痛：恢復力下調（休息拉長、減載加深）、進步速率放慢
        delta[I_RECOVERY] -= 0.06;
        delta[I_PROGRESSION] -= 0.04;
    }
    delta
}

/// 以一週回報微調網絡，重建課表（錨點＝下一週、必要時強制減載），並產出人話說明。
pub fn recalibrate(
    nn: &mut Mlp,
    assessment: &Assessment,
    log: &WeeklyLog,
    prev: &Profile,
) -> Recalibration {
    let log = log.sanitized();
    let adherence = log.adherence();
    let delta = target_delta(&log);
    let prev_arr = prev.to_array();

    // 1. 產生偽標籤（微調目標）
    let mut target = [0.0f32; OUTPUT_DIMS];
    for i in 0..OUTPUT_DIMS {
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
    let mut out = [0.0f32; OUTPUT_DIMS];
    let mut clamped_any = false;
    for i in 0..OUTPUT_DIMS {
        let lo = (prev_arr[i] - MAX_DOWN).max(SCORE_FLOOR);
        let hi = (prev_arr[i] + MAX_UP).min(SCORE_CEIL);
        let v = raw[i].clamp(lo, hi);
        if (v - raw[i]).abs() > 1e-6 {
            clamped_any = true;
        }
        out[i] = v;
    }
    let profile = Profile::from_array(out).clamped();

    // 4. 階段與減載判定（硬約束）
    let old_stage = planner::stage_for(&prev.scores);
    let stage = planner::stage_for(&profile.scores);
    let stage_changed = stage != old_stage;
    let force_deload = log.force_deload();

    // 5. 以新輸出重建課表：從下一週開始，必要時強制減載
    let plan = planner::build_plan_with(
        assessment,
        &profile,
        PlanOptions {
            next_week: log.next_week(),
            force_deload,
        },
    );

    // 6. 說明「改了什麼、為什麼」
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

    let pd = &prev.dosing;
    let nd = &profile.dosing;
    let arrow = |a: f32, b: f32| -> Option<String> {
        let d = b - a;
        if d.abs() < 0.005 {
            None
        } else {
            Some(format!(
                "{:.0}→{:.0}",
                (a * 100.0).round(),
                (b * 100.0).round()
            ))
        }
    };
    let mut parts = Vec::new();
    if let Some(s) = arrow(pd.work_capacity, nd.work_capacity) {
        parts.push(format!("工作容量 {s}（組數／次數落點）"));
    }
    if let Some(s) = arrow(pd.recovery, nd.recovery) {
        parts.push(format!("恢復力 {s}（組間休息／減載深度）"));
    }
    if let Some(s) = arrow(pd.progression_rate, nd.progression_rate) {
        parts.push(format!("進步速率 {s}（每週漸進斜率／預計升階）"));
    }
    if !parts.is_empty() {
        changes.push(ChangeNote {
            kind: "dosing".into(),
            message_zh: format!("劑量參數調整：{}", parts.join("；")),
        });
    }

    for p in &log.pain {
        changes.push(ChangeNote {
            kind: "pain".into(),
            message_zh: format!(
                "⚠️ {}不適 → 相關項目評分與恢復力下調，組間休息拉長並移除高負荷支撐類動作（退階替代）",
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
        let pct = ((1.0 - planner::deload_factor(nd)) * 100.0).round();
        changes.push(ChangeNote {
            kind: "deload".into(),
            message_zh: format!(
                "第 {} 週強制減載：組數相對上週下調約 {pct:.0}%（深度依恢復力決定），優先恢復",
                plan.next_week
            ),
        });
    }
    if clamped_any {
        changes.push(ChangeNote {
            kind: "safety".into(),
            message_zh: format!(
                "已套用安全夾限：單週每項輸出變動上限 +{MAX_UP:.2} / −{MAX_DOWN:.2}"
            ),
        });
    }

    Recalibration {
        profile,
        plan,
        stage,
        stage_changed,
        force_deload,
        changes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Assessment, DeloadKind};
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

    fn baseline() -> (Mlp, Profile) {
        let nn = Mlp::from_json(crate::BASELINE_WEIGHTS_JSON).unwrap();
        let prev = Profile::from_array(nn.infer(&assessment().features())).clamped();
        (nn, prev)
    }

    #[test]
    fn too_easy_raises_all_outputs_within_cap() {
        let (mut nn, prev) = baseline();
        let r = recalibrate(
            &mut nn,
            &assessment(),
            &log(Focus::TooEasy, vec![], 3, 3),
            &prev,
        );
        let p = prev.to_array();
        let n = r.profile.to_array();
        for i in 0..OUTPUT_DIMS {
            assert!(n[i] > p[i], "太輕鬆應上調第 {i} 維");
            assert!(
                n[i] - p[i] <= MAX_UP + 1e-4,
                "上調不得超過上限: {} -> {}",
                p[i],
                n[i]
            );
        }
        assert!(!r.force_deload);
        assert!(r.changes.iter().any(|c| c.kind == "dosing"));
        assert_eq!(r.plan.next_week, 2);
        assert!(!r.plan.weeks[1].is_deload);
    }

    #[test]
    fn pain_forces_deload_and_lowers_scores_and_recovery() {
        let (mut nn, prev) = baseline();
        let r = recalibrate(
            &mut nn,
            &assessment(),
            &log(Focus::Ok, vec![PainArea::Shoulder], 3, 3),
            &prev,
        );
        assert!(r.force_deload);
        assert!(r.profile.scores.overhead_press < prev.scores.overhead_press);
        assert!(r.profile.dosing.recovery < prev.dosing.recovery);
        assert!(r
            .changes
            .iter()
            .any(|c| c.kind == "pain" && c.message_zh.contains("肩膀")));
        // 強制減載真的套用在下一週課表上
        let w2 = &r.plan.weeks[1];
        assert!(w2.is_deload);
        assert_eq!(w2.deload_kind, Some(DeloadKind::Forced));
        assert!(planner::working_sets(w2) < planner::working_sets(&r.plan.weeks[2]));
        assert!(r
            .changes
            .iter()
            .any(|c| c.kind == "deload" && c.message_zh.contains("第 2 週")));
    }

    #[test]
    fn too_hard_lowers_all_outputs() {
        let (mut nn, prev) = baseline();
        let r = recalibrate(
            &mut nn,
            &assessment(),
            &log(Focus::TooHard, vec![], 1, 4),
            &prev,
        );
        let p = prev.to_array();
        let n = r.profile.to_array();
        for i in 0..OUTPUT_DIMS {
            assert!(n[i] < p[i], "太難應下調第 {i} 維");
            assert!(p[i] - n[i] <= MAX_DOWN + 1e-4);
        }
        assert!(r.force_deload, "低出席＋太難應強制減載");
    }

    #[test]
    fn too_hard_with_good_adherence_does_not_force_deload() {
        let (mut nn, prev) = baseline();
        let r = recalibrate(
            &mut nn,
            &assessment(),
            &log(Focus::TooHard, vec![], 3, 3),
            &prev,
        );
        assert!(!r.force_deload);
        assert!(!r.plan.weeks[1].is_deload);
        assert!(r.profile.dosing.work_capacity < prev.dosing.work_capacity);
    }

    #[test]
    fn last_week_log_anchors_on_week_12() {
        let (mut nn, prev) = baseline();
        let mut l = log(Focus::Ok, vec![PainArea::Wrist], 3, 3);
        l.week_index = 12;
        let r = recalibrate(&mut nn, &assessment(), &l, &prev);
        assert_eq!(r.plan.next_week, 12);
        assert_eq!(r.plan.weeks[11].deload_kind, Some(DeloadKind::Forced));
    }

    #[test]
    fn weights_stay_valid_after_finetune() {
        let (mut nn, prev) = baseline();
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

    #[test]
    fn repeated_easy_weeks_compound_but_stay_bounded() {
        let (mut nn, mut prev) = baseline();
        let first = prev;
        for w in 1..=6u8 {
            let mut l = log(Focus::TooEasy, vec![], 3, 3);
            l.week_index = w;
            let r = recalibrate(&mut nn, &assessment(), &l, &prev);
            prev = r.profile;
        }
        assert!(prev.dosing.work_capacity > first.dosing.work_capacity);
        for v in prev.to_array() {
            assert!((0.0..=1.0).contains(&v));
        }
    }
}
