//! 端到端流程測試：體測 → 推論 → 課表 → 回報 → 微調 → 新課表

use gas2_core::finetune::recalibrate;
use gas2_core::model::{Assessment, Focus, PainArea, Scores, WeeklyLog, TOTAL_WEEKS};
use gas2_core::nn::Mlp;
use gas2_core::planner::build_plan;
use gas2_core::{BASELINE_WEIGHTS_JSON, APP_VERSION};

fn assessment(strong: bool) -> Assessment {
    if strong {
        Assessment {
            shoulder_mobility: 5,
            wrist_mobility: 4,
            plank_sec: 150,
            hollow_sec: 90,
            pushup_reps: 40,
            pike_pushup_reps: 15,
            wall_walk_reps: 8,
            wall_hs_hold_sec: 80,
            wall_hspu_reps: 8,
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
fn baseline_weights_parse_and_marked() {
    let nn = Mlp::from_json(BASELINE_WEIGHTS_JSON).expect("內嵌基線權重必須合法");
    assert_eq!(nn.arch, [12, 16, 8, 5]);
}

#[test]
fn end_to_end_weak_user_full_flow() {
    let mut nn = Mlp::from_json(BASELINE_WEIGHTS_JSON).unwrap();
    let a = assessment(false);

    // 1. 體測 → 課表
    let scores = Scores::from_array(nn.infer(&a.features())).clamped();
    let plan = build_plan(&a, &scores);
    assert_eq!(plan.weeks.len(), TOTAL_WEEKS as usize);
    assert_eq!(plan.current_stage, 0, "新手應從基礎力量開始");

    // 2. 第 1 週回報（太難、低出席＋肩膀痛）
    let log = WeeklyLog {
        week_index: 1,
        sessions_completed: 1,
        sessions_planned: 3,
        focus: Focus::TooHard,
        pain: vec![PainArea::Shoulder],
        notes: Some("倒立撐下不去".into()),
    };
    let prev = plan.scores;
    let r = recalibrate(&mut nn, &a, &log, &prev);
    assert!(r.force_deload);
    assert!(r.new_scores.overhead_press < prev.overhead_press);

    // 3. 新課表生成且仍完整
    let plan2 = build_plan(&a, &r.new_scores);
    assert_eq!(plan2.weeks.len(), TOTAL_WEEKS as usize);

    // 4. 權重可匯出並重新載入（localStorage 持久化路徑）
    let json = nn.to_json();
    let nn2 = Mlp::from_json(&json).unwrap();
    let scores2 = Scores::from_array(nn2.infer(&a.features())).clamped();
    assert!(
        (scores2.overhead_press - r.new_scores.overhead_press).abs() < 1e-6,
        "匯入權重後推論結果應一致"
    );
}

#[test]
fn end_to_end_strong_user_starts_advanced() {
    let nn = Mlp::from_json(BASELINE_WEIGHTS_JSON).unwrap();
    let a = assessment(true);
    let scores = Scores::from_array(nn.infer(&a.features())).clamped();
    let plan = build_plan(&a, &scores);
    assert_eq!(plan.current_stage, 4, "高水準使用者應直接進入 PTH 專項");
}

#[test]
fn version_is_set() {
    assert!(!APP_VERSION.is_empty());
}
