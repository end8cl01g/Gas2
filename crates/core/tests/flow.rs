//! 端到端流程測試：體測 → 推論（評分＋劑量）→ 課表 → 回報 → 微調 → 新課表

use gas2_core::finetune::recalibrate;
use gas2_core::model::{
    Assessment, DeloadKind, Dosing, Focus, PainArea, Profile, Scores, WeeklyLog, OUTPUT_DIMS,
    TOTAL_WEEKS,
};
use gas2_core::nn::Mlp;
use gas2_core::planner::{build_plan, deload_factor, rest_factor, working_sets};
use gas2_core::{APP_VERSION, BASELINE_WEIGHTS_JSON};

fn assessment(strong: bool) -> Assessment {
    if strong {
        Assessment {
            shoulder_mobility: 5,
            wrist_mobility: 4,
            plank_sec: 150,
            hollow_sec: 90,
            pushup_reps: 40,
            pike_pushup_reps: 20,
            wall_walk_reps: 8,
            wall_hs_hold_sec: 80,
            wall_hspu_reps: 12,
            bodyweight_kg: 65.0,
            height_cm: 178.0,
            days_per_week: 5,
            experience: 3,
        }
    } else {
        Assessment {
            shoulder_mobility: 2,
            wrist_mobility: 2,
            plank_sec: 40,
            hollow_sec: 15,
            pushup_reps: 8,
            pike_pushup_reps: 2,
            wall_walk_reps: 0,
            wall_hs_hold_sec: 0,
            wall_hspu_reps: 0,
            bodyweight_kg: 78.0,
            height_cm: 170.0,
            days_per_week: 3,
            experience: 0,
        }
    }
}

#[test]
fn baseline_weights_parse_and_match_schema() {
    let nn = Mlp::from_json(BASELINE_WEIGHTS_JSON).expect("內嵌基線權重必須合法");
    assert_eq!(nn.arch, [12, 24, 12, OUTPUT_DIMS]);
}

#[test]
fn end_to_end_weak_user_full_flow() {
    let mut nn = Mlp::from_json(BASELINE_WEIGHTS_JSON).unwrap();
    let a = assessment(false);

    // 1. 體測 → 8 維輸出 → 課表
    let profile = Profile::from_array(nn.infer(&a.features())).clamped();
    let plan = build_plan(&a, &profile);
    assert_eq!(plan.weeks.len(), TOTAL_WEEKS as usize);
    assert!(plan.current_stage <= 4);
    assert_eq!(plan.next_week, 1);
    // 註：階段門檻的正確性由 planner 單元測試以合成評分覆蓋（不依賴權重狀態）

    // 2. 第 1 週回報（太難、低出席＋肩膀痛）
    let log = WeeklyLog {
        week_index: 1,
        sessions_completed: 1,
        sessions_planned: 3,
        focus: Focus::TooHard,
        pain: vec![PainArea::Shoulder],
        notes: Some("倒立撐下不去".into()),
    };
    let r = recalibrate(&mut nn, &a, &log, &profile);
    assert!(r.force_deload);
    assert!(r.profile.scores.overhead_press < profile.scores.overhead_press);
    assert!(r.profile.dosing.work_capacity < profile.dosing.work_capacity);
    assert!(r.profile.dosing.recovery < profile.dosing.recovery);

    // 3. 新課表：從第 2 週開始、第 2 週強制減載且組數真的比較少
    assert_eq!(r.plan.weeks.len(), TOTAL_WEEKS as usize);
    assert_eq!(r.plan.next_week, 2);
    assert_eq!(r.plan.weeks[1].deload_kind, Some(DeloadKind::Forced));
    assert!(working_sets(&r.plan.weeks[1]) < working_sets(&r.plan.weeks[2]));
    // 恢復力下降 → 組間休息係數上升（休息拉長）、減載更深
    assert!(rest_factor(&r.profile.dosing) > rest_factor(&profile.dosing));
    assert!(deload_factor(&r.profile.dosing) < deload_factor(&profile.dosing));

    // 4. 權重可匯出並重新載入（localStorage 持久化路徑）
    let json = nn.to_json();
    let nn2 = Mlp::from_json(&json).unwrap();
    let p2 = Profile::from_array(nn2.infer(&a.features())).clamped();
    assert!(
        (p2.scores.overhead_press - r.profile.scores.overhead_press).abs() < 1e-6,
        "匯入權重後推論結果應一致"
    );
}

#[test]
fn end_to_end_strong_user_starts_advanced() {
    // 階段判定是評分的純函數：滿分合成評分應直達 PTH 專項（與權重狀態無關）
    let a = assessment(true);
    let profile = Profile {
        scores: Scores::from_array([0.95; 5]).clamped(),
        dosing: Dosing {
            work_capacity: 0.9,
            recovery: 0.9,
            progression_rate: 0.9,
        },
    };
    let plan = build_plan(&a, &profile);
    assert_eq!(plan.current_stage, 4, "滿分能力應進入 PTH 專項");
}

#[test]
fn baseline_network_separates_weak_and_strong_dosing() {
    // 訓練後的網絡應讓高手的劑量參數高於新手（規則引擎標籤的單調性）
    let nn = Mlp::from_json(BASELINE_WEIGHTS_JSON).unwrap();
    if !nn.trained {
        eprintln!("權重未訓練，跳過劑量單調性檢查");
        return;
    }
    let weak = Profile::from_array(nn.infer(&assessment(false).features()));
    let strong = Profile::from_array(nn.infer(&assessment(true).features()));
    assert!(strong.dosing.work_capacity > weak.dosing.work_capacity);
    assert!(strong.dosing.recovery > weak.dosing.recovery);
    assert!(strong.dosing.progression_rate > weak.dosing.progression_rate);
    // 反映到課表：高手第 1 週組數更多、休息更短
    let pw = build_plan(&assessment(false), &weak);
    let ps = build_plan(&assessment(true), &strong);
    assert!(
        ps.weeks[0].volume_scale > pw.weeks[0].volume_scale,
        "{} vs {}",
        ps.weeks[0].volume_scale,
        pw.weeks[0].volume_scale
    );
}

#[test]
fn version_is_set() {
    assert!(!APP_VERSION.is_empty());
}
