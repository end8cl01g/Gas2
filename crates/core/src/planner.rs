//! 課表規劃器：把能力評分（神經網絡輸出）轉成 12 週週期化課表。
//!
//! 硬約束（安全網，不受線上微調影響）：
//! - 階段只按評分門檻推進，一次最多一階
//! - 每週訓練量漸進有上限，第 4/8 週強制減載
//! - 每週訓練次數夾限 2–5

use crate::exercises::{self, Exercise};
use crate::model::{
    Assessment, Block, BlockKind, Plan, PlanSummary, PlanWeek, Prescription, Scores, Session,
    TOTAL_WEEKS,
};

pub struct StageDef {
    pub index: u8,
    pub name_zh: &'static str,
    pub goal_zh: &'static str,
}

pub const STAGES: [StageDef; 5] = [
    StageDef {
        index: 0,
        name_zh: "基礎力量",
        goal_zh: "建立推撐與核心基礎",
    },
    StageDef {
        index: 1,
        name_zh: "壓撐與支撐",
        goal_zh: "發展肩角壓撐與倒立支撐耐受",
    },
    StageDef {
        index: 2,
        name_zh: "倒立技能",
        goal_zh: "取得穩定倒立平衡",
    },
    StageDef {
        index: 3,
        name_zh: "倒立力量",
        goal_zh: "倒立姿勢下的垂直推力",
    },
    StageDef {
        index: 4,
        name_zh: "PTH 專項",
        goal_zh: "組合壓撐與倒立，完成 Press to Handstand",
    },
];

/// 階段推進門檻（規劃器硬約束）
pub fn stage_exit(scores: &Scores, stage: u8) -> bool {
    match stage {
        0 => scores.base_push >= 0.45 && scores.core_control >= 0.40,
        1 => scores.overhead_press >= 0.40 && scores.balance_skill >= 0.30,
        2 => scores.balance_skill >= 0.55 && scores.core_control >= 0.50,
        3 => scores.overhead_press >= 0.60 && scores.compression_power >= 0.45,
        _ => false,
    }
}

/// 由評分決定所處階段（最多推進到 4）
pub fn stage_for(scores: &Scores) -> u8 {
    let mut s: u8 = 0;
    while s < 4 && stage_exit(scores, s) {
        s += 1;
    }
    s
}

// ── 動作選擇（依評分決定適當變化式）────────────────────────────
fn pick_push(s: &Scores) -> &'static str {
    if s.base_push < 0.35 {
        "incline_pushup"
    } else {
        "pushup"
    }
}

fn pick_press(s: &Scores) -> &'static str {
    if s.overhead_press < 0.20 {
        "pike_pushup"
    } else if s.overhead_press < 0.45 {
        "elevated_pike_pushup"
    } else if s.overhead_press < 0.65 {
        "wall_hspu_partial"
    } else {
        "wall_hspu"
    }
}

fn pick_core(s: &Scores) -> &'static str {
    if s.core_control < 0.35 {
        "dead_bug"
    } else {
        "hollow_hold"
    }
}

fn pick_pth(s: &Scores) -> &'static str {
    if s.compression_power < 0.30 {
        "pth_neg_elevated"
    } else if s.compression_power < 0.55 {
        "tuck_pth"
    } else if s.compression_power < 0.80 {
        "pth_neg"
    } else {
        "pth_full"
    }
}

fn skill_drills(stage: u8, s: &Scores) -> Vec<&'static str> {
    match stage {
        0 => vec!["wall_plank", "wall_walk"],
        1 => vec!["wall_walk", "chest_to_wall"],
        2 => vec![
            "chest_to_wall",
            if s.balance_skill < 0.42 {
                "wall_handstand_hold"
            } else {
                "kickup_practice"
            },
        ],
        3 => vec!["wall_handstand_hold", "kickup_practice"],
        _ => vec!["chest_to_wall", pick_pth(s)],
    }
}

fn main_pair(stage: u8, s: &Scores) -> Vec<&'static str> {
    match stage {
        0 => vec![pick_push(s), "pike_pushup"],
        1 => vec![pick_press(s), "seated_pike_compress"],
        2 => vec![pick_press(s), pick_core(s)],
        3 => vec![pick_press(s), pick_core(s)],
        _ => vec![pick_pth(s), pick_press(s)],
    }
}

// ── 組課表 ────────────────────────────────────────────────────
fn rx(e: &'static Exercise, sets: u8) -> Prescription {
    Prescription {
        exercise_id: e.id.to_string(),
        name_zh: e.name_zh.to_string(),
        cues_zh: e.cues_zh.iter().map(|s| s.to_string()).collect(),
        sets,
        reps: e.base_reps.to_string(),
        rest_sec: e.rest_sec,
        regression_zh: e.regression_zh.to_string(),
        progression_zh: e.progression_zh.to_string(),
    }
}

fn scale_sets(base: u8, volume: f32, deload: bool) -> u8 {
    let scaled = (base as f32 * volume).round().max(1.0);
    let capped = scaled.min(base as f32 + 2.0).min(6.0);
    if deload {
        (capped * 0.6).round().max(1.0) as u8
    } else {
        capped as u8
    }
}

fn block_of(kind: BlockKind, ids: &[&str], volume: f32, deload: bool) -> Block {
    let items = ids
        .iter()
        .filter_map(|id| exercises::get(id))
        .map(|e| rx(e, scale_sets(e.base_sets, volume, deload)))
        .collect();
    Block { kind, items }
}

fn strength_session(idx: usize, stage: u8, s: &Scores, volume: f32, deload: bool) -> Session {
    let warmup = block_of(
        BlockKind::Warmup,
        &["wrist_prep", "shoulder_circles"],
        1.0,
        false,
    );
    let main = block_of(BlockKind::Main, &main_pair(stage, s), volume, deload);
    let mut core_ids: Vec<&str> = vec![pick_core(s)];
    if stage < 2 {
        core_ids.push("plank_shoulder_taps");
    }
    let core = block_of(BlockKind::Core, &core_ids, volume, deload);
    let mobility = block_of(BlockKind::Mobility, &["wall_down_dog"], 1.0, false);
    Session {
        label_zh: format!("訓練 {}・力量重點", idx + 1),
        blocks: vec![warmup, main, core, mobility],
    }
}

fn skill_session(idx: usize, stage: u8, s: &Scores, volume: f32, deload: bool) -> Session {
    let warmup = block_of(
        BlockKind::Warmup,
        &["wrist_prep", "shoulder_circles"],
        1.0,
        false,
    );
    let skill = block_of(BlockKind::Skill, &skill_drills(stage, s), volume, deload);
    let maintain = block_of(BlockKind::Accessory, &[pick_push(s)], volume * 0.7, deload);
    let mobility = block_of(
        BlockKind::Mobility,
        &["wrist_stretch", "shoulder_flex_stretch"],
        1.0,
        false,
    );
    Session {
        label_zh: format!("訓練 {}・技能重點", idx + 1),
        blocks: vec![warmup, skill, maintain, mobility],
    }
}

fn volume_scale(week: u8, deload: bool) -> f32 {
    if deload {
        0.6
    } else {
        (1.0 + 0.08 * (week - 1).min(5) as f32).min(1.4)
    }
}

fn focus_zh(week: u8, deload: bool, vs: f32) -> String {
    if week == TOTAL_WEEKS {
        "重測週：完成本週後重新體測，更新個人化路徑".to_string()
    } else if deload {
        "減載週：組數下調，以恢復與動作品質優先".to_string()
    } else {
        format!("漸進超載：主課表訓練量係數 ×{vs:.2}")
    }
}

/// 生成完整 12 週課表
pub fn build_plan(a: &Assessment, scores: &Scores) -> Plan {
    let scores = scores.clamped();
    let stage = stage_for(&scores);
    let sessions_per_week = a.sanitized().days_per_week.clamp(2, 5);

    let weeks = (1u8..=TOTAL_WEEKS)
        .map(|w| {
            let is_deload = w % 4 == 0;
            let vs = volume_scale(w, is_deload);
            let sessions = (0..sessions_per_week)
                .map(|i| {
                    if i % 2 == 0 {
                        strength_session(i as usize, stage, &scores, vs, is_deload)
                    } else {
                        skill_session(i as usize, stage, &scores, vs, is_deload)
                    }
                })
                .collect();
            PlanWeek {
                week_index: w,
                stage,
                stage_name_zh: STAGES[stage as usize].name_zh.to_string(),
                is_deload,
                sessions_per_week,
                focus_zh: focus_zh(w, is_deload, vs),
                sessions,
            }
        })
        .collect();

    Plan {
        total_weeks: TOTAL_WEEKS,
        current_stage: stage,
        scores,
        summary: PlanSummary {
            stage_name_zh: STAGES[stage as usize].name_zh.to_string(),
            goal_zh: STAGES[stage as usize].goal_zh.to_string(),
            sessions_per_week,
            note_zh: format!(
                "路徑共 {} 週；每 4 週一個減載，第 {} 週重新體測以更新路徑。每次訓練後以週回報驅動微調。",
                TOTAL_WEEKS, TOTAL_WEEKS
            ),
        },
        weeks,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Assessment;

    fn assessment(days: u8) -> Assessment {
        Assessment {
            shoulder_mobility: 2,
            wrist_mobility: 2,
            plank_sec: 45,
            hollow_sec: 20,
            pushup_reps: 10,
            pike_pushup_reps: 3,
            wall_walk_reps: 1,
            wall_hs_hold_sec: 5,
            wall_hspu_reps: 0,
            bodyweight_kg: 70.0,
            height_cm: 175.0,
            days_per_week: days,
            experience: 0,
        }
    }

    fn scores(vals: [f32; 5]) -> Scores {
        Scores::from_array(vals).clamped()
    }

    #[test]
    fn weak_athlete_stays_in_stage0() {
        let s = scores([0.1, 0.1, 0.1, 0.1, 0.1]);
        assert_eq!(stage_for(&s), 0);
        let plan = build_plan(&assessment(3), &s);
        assert_eq!(plan.current_stage, 0);
        assert_eq!(plan.weeks.len(), TOTAL_WEEKS as usize);
    }

    #[test]
    fn strong_athlete_lands_stage4() {
        let s = scores([0.9, 0.9, 0.9, 0.9, 0.9]);
        assert_eq!(stage_for(&s), 4);
    }

    #[test]
    fn deload_weeks_are_4_8_12() {
        let plan = build_plan(&assessment(4), &scores([0.3; 5]));
        for w in &plan.weeks {
            assert_eq!(w.is_deload, w.week_index % 4 == 0);
        }
        let deloads: Vec<u8> = plan
            .weeks
            .iter()
            .filter(|w| w.is_deload)
            .map(|w| w.week_index)
            .collect();
        assert_eq!(deloads, vec![4, 8, 12]);
    }

    #[test]
    fn sessions_clamped_between_2_and_5() {
        for days in [1u8, 2, 3, 5, 7] {
            let plan = build_plan(&assessment(days), &scores([0.3; 5]));
            let n = plan.weeks[0].sessions.len() as u8;
            assert!((2..=5).contains(&n), "days={days} → sessions={n}");
        }
    }

    #[test]
    fn deload_volume_not_above_normal_week() {
        let plan = build_plan(&assessment(3), &scores([0.3; 5]));
        let normal_sets: u32 = plan.weeks[2]
            .sessions
            .iter()
            .flat_map(|s| s.blocks.iter())
            .flat_map(|b| b.items.iter())
            .map(|p| p.sets as u32)
            .sum();
        let deload_sets: u32 = plan.weeks[3]
            .sessions
            .iter()
            .flat_map(|s| s.blocks.iter())
            .flat_map(|b| b.items.iter())
            .map(|p| p.sets as u32)
            .sum();
        assert!(
            deload_sets < normal_sets,
            "減載週總組數 ({deload_sets}) 應低於正常週 ({normal_sets})"
        );
    }

    #[test]
    fn plan_serializes_and_deserializes() {
        let plan = build_plan(&assessment(3), &scores([0.5; 5]));
        let json = serde_json::to_string(&plan).unwrap();
        assert!(json.contains("\"weekIndex\""));
        let back: Plan = serde_json::from_str(&json).unwrap();
        assert_eq!(back.total_weeks, TOTAL_WEEKS);
    }
}
