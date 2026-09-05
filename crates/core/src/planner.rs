//! 課表規劃器：把神經網絡輸出（五項能力評分＋三項劑量參數）轉成 12 週週期化課表。
//!
//! 神經網絡決定「多少、多快」：
//! - 能力評分 → 階段、動作變化式
//! - 工作容量 → 組數係數、次數落在區間的哪一段
//! - 恢復力   → 組間休息長短、減載深度
//! - 進步速率 → 每負荷週漸進斜率、跨週能力投影（後段週次預先呈現升階）
//!
//! 硬約束（安全網，不受線上微調影響）：
//! - 階段只按評分門檻推進；投影每 4 週 block 最多升一階
//! - 組數 ≤ 基準+2 且 ≤ 6；次數不超出動作庫區間；休息 40–240 秒
//! - 每週漸進斜率上限 +10%、累積係數上限 ×1.5
//! - 第 4/8/12 週強制排程減載；回報疼痛／過難的下一週強制減載
//! - 每週訓練次數夾限 2–5

use crate::exercises::{self, Exercise};
use crate::model::{
    Assessment, Block, BlockKind, DeloadKind, Dosing, Plan, PlanSummary, PlanWeek, Prescription,
    Profile, Scores, Session, TOTAL_WEEKS,
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

// ── 硬約束常數 ────────────────────────────────────────────────
/// 每負荷週漸進斜率範圍（由進步速率決定）
pub const PROGRESSION_MIN: f32 = 0.03;
pub const PROGRESSION_MAX: f32 = 0.10;
/// 累積訓練量係數上限
pub const VOLUME_CAP: f32 = 1.5;
/// 減載係數範圍（由恢復力決定：恢復差 → 減更多）
pub const DELOAD_MIN: f32 = 0.5;
pub const DELOAD_MAX: f32 = 0.7;
/// 組間休息絕對範圍
pub const REST_MIN: u16 = 40;
pub const REST_MAX: u16 = 240;
/// 跨週投影：每負荷週每項能力最大成長（進步速率滿分時）
pub const PROJECTION_MAX_PER_WEEK: f32 = 0.02;

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

// ── 劑量參數 → 規劃係數（純函數，可單元測試）────────────────

/// 每負荷週漸進斜率：進步速率 0→+3%、1→+10%
pub fn progression_step(d: &Dosing) -> f32 {
    PROGRESSION_MIN + (PROGRESSION_MAX - PROGRESSION_MIN) * d.progression_rate.clamp(0.0, 1.0)
}

/// 起始訓練量係數：工作容量 0→×0.8、1→×1.2（基準 3 組的動作 → 2／3／4 組）
pub fn base_volume(d: &Dosing) -> f32 {
    0.8 + 0.4 * d.work_capacity.clamp(0.0, 1.0)
}

/// 減載係數：恢復力 0→×0.5、1→×0.7
pub fn deload_factor(d: &Dosing) -> f32 {
    DELOAD_MIN + (DELOAD_MAX - DELOAD_MIN) * d.recovery.clamp(0.0, 1.0)
}

/// 組間休息係數：恢復力 0→×1.3、1→×0.8
pub fn rest_factor(d: &Dosing) -> f32 {
    1.3 - 0.5 * d.recovery.clamp(0.0, 1.0)
}

/// 次數落點（0–1）：工作容量決定在動作區間內選取子窗的位置，
/// 每個負荷週（同一 block 內）再往上推進一點；減載週回到區間下段。
pub fn rep_position(d: &Dosing, load_weeks: u8, deload: bool) -> f32 {
    if deload {
        return 0.0;
    }
    let base = 0.15 + 0.55 * d.work_capacity.clamp(0.0, 1.0);
    let step = 0.10 + 0.15 * d.progression_rate.clamp(0.0, 1.0);
    (base + step * load_weeks as f32).clamp(0.0, 1.0)
}

/// 每週訓練量係數：起始係數 × (1 + 斜率 × 累計負荷週)，上限 VOLUME_CAP；
/// 減載週：呼叫端傳入「上一個負荷週」的累計數，再乘以減載係數（＝相對上週下調）
pub fn volume_scale(d: &Dosing, load_weeks: u8, deload: bool) -> f32 {
    let v = (base_volume(d) * (1.0 + progression_step(d) * load_weeks as f32)).min(VOLUME_CAP);
    if deload {
        v * deload_factor(d)
    } else {
        v
    }
}

// ── 跨週能力投影 ──────────────────────────────────────────────

/// 由目前評分投影未來第 `load_weeks` 個負荷週後的評分（每週最多 +PROJECTION_MAX_PER_WEEK×進步速率，
/// 越接近滿分成長越慢），並以「每 4 週 block 最多升一階」硬約束截斷。
pub fn project_scores(scores: &Scores, d: &Dosing, load_weeks: u8, block_index: u8) -> Scores {
    let rate = d.progression_rate.clamp(0.0, 1.0) * PROJECTION_MAX_PER_WEEK;
    let base_stage = stage_for(scores);
    let mut arr = scores.to_array();
    for v in arr.iter_mut() {
        let mut x = *v;
        for _ in 0..load_weeks {
            x += rate * (1.0 - x);
        }
        *v = x.clamp(0.0, 1.0);
    }
    let projected = Scores::from_array(arr);
    let max_stage = (base_stage + block_index).min(4);
    if stage_for(&projected) <= max_stage {
        return projected;
    }
    // 超過允許階段：把觸發下一階的門檻項目回壓到「剛好未達門檻」
    let mut capped = projected;
    let mut guard = 0;
    while stage_for(&capped) > max_stage && guard < 8 {
        capped = hold_below_exit(&capped, max_stage);
        guard += 1;
    }
    capped
}

/// 將評分中觸發 `stage` 出口條件的項目降到門檻以下（保留其他項目的成長）
fn hold_below_exit(scores: &Scores, stage: u8) -> Scores {
    const EPS: f32 = 0.005;
    let mut s = *scores;
    match stage {
        0 => s.base_push = s.base_push.min(0.45 - EPS),
        1 => s.overhead_press = s.overhead_press.min(0.40 - EPS),
        2 => s.balance_skill = s.balance_skill.min(0.55 - EPS),
        3 => s.overhead_press = s.overhead_press.min(0.60 - EPS),
        _ => {}
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

// ── 劑量計算 ──────────────────────────────────────────────────

/// 本週劑量情境（由 Profile + 週次推導，所有 block 共用）
#[derive(Debug, Clone, Copy)]
struct WeekDose {
    volume: f32,
    /// 次數落點；None = 使用動作完整區間（熱身／活動度）
    rep_pos: Option<f32>,
    rest_factor: f32,
}

fn scale_sets(base: u8, volume: f32) -> u8 {
    let hi = (base as f32 + 2.0).min(6.0);
    (base as f32 * volume).round().clamp(1.0, hi) as u8
}

/// 在 [lo, hi] 區間內依落點選取子窗：窗寬約為區間的一半，落點 0 → 貼齊下緣，1 → 貼齊上緣
fn rep_window(lo: u16, hi: u16, pos: f32) -> (u16, u16) {
    if hi <= lo {
        return (lo, lo);
    }
    let span = (hi - lo) as f32;
    let width = (span * 0.5).round().clamp(1.0, span);
    let start = lo as f32 + (span - width) * pos.clamp(0.0, 1.0);
    let a = (start.round() as u16).clamp(lo, hi);
    let b = ((start + width).round() as u16).clamp(a, hi);
    (a, b)
}

fn scale_rest(base: u16, factor: f32) -> u16 {
    let r = (base as f32 * factor / 5.0).round() * 5.0;
    (r as u16).clamp(REST_MIN.min(base), REST_MAX)
}

fn rx(e: &'static Exercise, dose: WeekDose, volume_mult: f32) -> Prescription {
    let (lo, hi) = match dose.rep_pos {
        Some(pos) => rep_window(e.rep_lo, e.rep_hi, pos),
        None => (e.rep_lo, e.rep_hi),
    };
    let rest_sec = scale_rest(e.base_rest_sec, dose.rest_factor);
    Prescription {
        exercise_id: e.id.to_string(),
        name_zh: e.name_zh.to_string(),
        cues_zh: e.cues_zh.iter().map(|s| s.to_string()).collect(),
        sets: scale_sets(e.base_sets, dose.volume * volume_mult),
        reps: e.unit.format(lo, hi),
        reps_lo: lo,
        reps_hi: hi,
        unit: e.unit,
        rest_sec,
        regression_zh: e.regression_zh.to_string(),
        progression_zh: e.progression_zh.to_string(),
    }
}

/// 熱身／活動度：固定基準劑量（不隨訓練量縮放，休息不變）
fn fixed_block(kind: BlockKind, ids: &[&str]) -> Block {
    let dose = WeekDose {
        volume: 1.0,
        rep_pos: None,
        rest_factor: 1.0,
    };
    block_with(kind, ids, dose, 1.0)
}

fn block_with(kind: BlockKind, ids: &[&str], dose: WeekDose, volume_mult: f32) -> Block {
    let items = ids
        .iter()
        .filter_map(|id| exercises::get(id))
        .map(|e| rx(e, dose, volume_mult))
        .collect();
    Block { kind, items }
}

fn strength_session(idx: usize, stage: u8, s: &Scores, dose: WeekDose) -> Session {
    let warmup = fixed_block(BlockKind::Warmup, &["wrist_prep", "shoulder_circles"]);
    let main = block_with(BlockKind::Main, &main_pair(stage, s), dose, 1.0);
    let mut core_ids: Vec<&str> = vec![pick_core(s)];
    if stage < 2 {
        core_ids.push("plank_shoulder_taps");
    }
    let core = block_with(BlockKind::Core, &core_ids, dose, 1.0);
    let mobility = fixed_block(BlockKind::Mobility, &["wall_down_dog"]);
    Session {
        label_zh: format!("訓練 {}・力量重點", idx + 1),
        blocks: vec![warmup, main, core, mobility],
    }
}

fn skill_session(idx: usize, stage: u8, s: &Scores, dose: WeekDose) -> Session {
    let warmup = fixed_block(BlockKind::Warmup, &["wrist_prep", "shoulder_circles"]);
    let skill = block_with(BlockKind::Skill, &skill_drills(stage, s), dose, 1.0);
    let maintain = block_with(BlockKind::Accessory, &[pick_push(s)], dose, 0.7);
    let mobility = fixed_block(
        BlockKind::Mobility,
        &["wrist_stretch", "shoulder_flex_stretch"],
    );
    Session {
        label_zh: format!("訓練 {}・技能重點", idx + 1),
        blocks: vec![warmup, skill, maintain, mobility],
    }
}

fn focus_zh(week: u8, deload: Option<DeloadKind>, vs: f32, stage_up: bool, step: f32) -> String {
    match deload {
        Some(DeloadKind::Forced) => {
            format!("強制減載週：依上週回報（疼痛／過難）組數下調至 ×{vs:.2}，優先恢復與動作品質")
        }
        Some(DeloadKind::Scheduled) if week == TOTAL_WEEKS => {
            format!("重測週（減載 ×{vs:.2}）：完成本週後重新體測，更新個人化路徑")
        }
        Some(DeloadKind::Scheduled) => {
            format!("減載週：組數下調至 ×{vs:.2}（依恢復力決定深度），以恢復與動作品質優先")
        }
        None if stage_up => {
            format!("預計升階週：依進步速率投影，動作變化式提前升級；訓練量係數 ×{vs:.2}")
        }
        None => format!(
            "漸進超載：訓練量係數 ×{vs:.2}（每負荷週 +{:.0}%，由進步速率決定）",
            step * 100.0
        ),
    }
}

/// 規劃參數
#[derive(Debug, Clone, Copy, Default)]
pub struct PlanOptions {
    /// 下一個要執行的週次（1–12）；回報第 n 週後為 n+1
    pub next_week: u8,
    /// 是否在 next_week 強制減載（疼痛／過難）
    pub force_deload: bool,
}

/// 生成完整 12 週課表（初次體測：從第 1 週開始，無強制減載）
pub fn build_plan(a: &Assessment, profile: &Profile) -> Plan {
    build_plan_with(a, profile, PlanOptions::default())
}

/// 生成完整 12 週課表，含回報後的錨點週與強制減載
pub fn build_plan_with(a: &Assessment, profile: &Profile, opts: PlanOptions) -> Plan {
    let profile = profile.clamped();
    let scores = profile.scores;
    let dosing = profile.dosing;
    let stage = stage_for(&scores);
    let sessions_per_week = a.sanitized().days_per_week.clamp(2, 5);
    let anchor = opts.next_week.clamp(1, TOTAL_WEEKS);
    let step = progression_step(&dosing);
    let rest_f = rest_factor(&dosing);

    // 錨點週之前的週次已成過去：顯示為目前評分、負荷週計數 0
    let mut load_weeks: u8 = 0;
    let weeks = (1u8..=TOTAL_WEEKS)
        .map(|w| {
            let scheduled = w % 4 == 0;
            let forced = opts.force_deload && w == anchor;
            let deload_kind = if forced {
                Some(DeloadKind::Forced)
            } else if scheduled {
                Some(DeloadKind::Scheduled)
            } else {
                None
            };
            let is_deload = deload_kind.is_some();
            let past = w < anchor;

            let lw = if past { 0 } else { load_weeks };
            let block_index = if past { 0 } else { (w - anchor) / 4 };
            let projected = if past {
                scores
            } else {
                project_scores(&scores, &dosing, lw, block_index)
            };
            let week_stage = stage_for(&projected);
            // 減載週相對「上一個負荷週」下調；每個 4 週 block 含 3 個負荷週，次數落點在 block 內推進
            let vs = if is_deload {
                volume_scale(&dosing, lw.saturating_sub(1), true)
            } else {
                volume_scale(&dosing, lw, false)
            };
            let dose = WeekDose {
                volume: vs,
                rep_pos: Some(rep_position(&dosing, lw % 3, is_deload)),
                rest_factor: rest_f,
            };
            if !past && !is_deload {
                load_weeks += 1;
            }

            let sessions = (0..sessions_per_week)
                .map(|i| {
                    if i % 2 == 0 {
                        strength_session(i as usize, week_stage, &projected, dose)
                    } else {
                        skill_session(i as usize, week_stage, &projected, dose)
                    }
                })
                .collect();
            PlanWeek {
                week_index: w,
                stage: week_stage,
                stage_name_zh: STAGES[week_stage as usize].name_zh.to_string(),
                is_deload,
                deload_kind,
                sessions_per_week,
                volume_scale: vs,
                projected_scores: projected,
                focus_zh: focus_zh(w, deload_kind, vs, week_stage > stage, step),
                sessions,
            }
        })
        .collect();

    Plan {
        total_weeks: TOTAL_WEEKS,
        current_stage: stage,
        scores,
        dosing,
        next_week: anchor,
        summary: PlanSummary {
            stage_name_zh: STAGES[stage as usize].name_zh.to_string(),
            goal_zh: STAGES[stage as usize].goal_zh.to_string(),
            sessions_per_week,
            note_zh: format!(
                "路徑共 {} 週；每 4 週一個減載，第 {} 週重新體測。組數、次數、休息與漸進速度由神經網絡的劑量參數（工作容量 {:.0}／恢復力 {:.0}／進步速率 {:.0}）決定，每週回報後重算。",
                TOTAL_WEEKS,
                TOTAL_WEEKS,
                dosing.work_capacity * 100.0,
                dosing.recovery * 100.0,
                dosing.progression_rate * 100.0
            ),
        },
        weeks,
    }
}

/// 所有主課表／技能／核心／輔助 block 的總組數（測試與診斷用）
pub fn working_sets(week: &PlanWeek) -> u32 {
    week.sessions
        .iter()
        .flat_map(|s| s.blocks.iter())
        .filter(|b| !matches!(b.kind, BlockKind::Warmup | BlockKind::Mobility))
        .flat_map(|b| b.items.iter())
        .map(|p| p.sets as u32)
        .sum()
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

    fn dosing(w: f32, r: f32, p: f32) -> Dosing {
        Dosing {
            work_capacity: w,
            recovery: r,
            progression_rate: p,
        }
    }

    fn profile(s: [f32; 5], d: Dosing) -> Profile {
        Profile {
            scores: scores(s),
            dosing: d,
        }
    }

    const MID: Dosing = Dosing {
        work_capacity: 0.5,
        recovery: 0.5,
        progression_rate: 0.5,
    };

    #[test]
    fn weak_athlete_stays_in_stage0() {
        let p = profile([0.1; 5], dosing(0.5, 0.5, 0.0));
        assert_eq!(stage_for(&p.scores), 0);
        let plan = build_plan(&assessment(3), &p);
        assert_eq!(plan.current_stage, 0);
        assert_eq!(plan.weeks.len(), TOTAL_WEEKS as usize);
        assert!(plan.weeks.iter().all(|w| w.stage == 0));
    }

    #[test]
    fn strong_athlete_lands_stage4() {
        let s = scores([0.9, 0.9, 0.9, 0.9, 0.9]);
        assert_eq!(stage_for(&s), 4);
    }

    #[test]
    fn deload_weeks_are_4_8_12() {
        let plan = build_plan(&assessment(4), &profile([0.3; 5], MID));
        for w in &plan.weeks {
            assert_eq!(w.is_deload, w.week_index % 4 == 0);
            if w.is_deload {
                assert_eq!(w.deload_kind, Some(DeloadKind::Scheduled));
            }
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
            let plan = build_plan(&assessment(days), &profile([0.3; 5], MID));
            let n = plan.weeks[0].sessions.len() as u8;
            assert!((2..=5).contains(&n), "days={days} → sessions={n}");
        }
    }

    #[test]
    fn deload_volume_below_normal_week() {
        let plan = build_plan(&assessment(3), &profile([0.3; 5], MID));
        let normal = working_sets(&plan.weeks[2]);
        let deload = working_sets(&plan.weeks[3]);
        assert!(
            deload < normal,
            "減載週總組數 ({deload}) 應低於正常週 ({normal})"
        );
    }

    #[test]
    fn plan_serializes_and_deserializes() {
        let plan = build_plan(&assessment(3), &profile([0.5; 5], MID));
        let json = serde_json::to_string(&plan).unwrap();
        assert!(json.contains("\"weekIndex\""));
        assert!(json.contains("\"projectedScores\""));
        assert!(json.contains("\"repsLo\""));
        assert!(json.contains("\"deloadKind\":\"scheduled\""));
        let back: Plan = serde_json::from_str(&json).unwrap();
        assert_eq!(back.total_weeks, TOTAL_WEEKS);
    }

    // ── 劑量參數驅動 ──

    #[test]
    fn work_capacity_drives_sets_and_reps() {
        let low = build_plan(&assessment(3), &profile([0.3; 5], dosing(0.0, 0.5, 0.5)));
        let high = build_plan(&assessment(3), &profile([0.3; 5], dosing(1.0, 0.5, 0.5)));
        assert!(
            working_sets(&high.weeks[0]) > working_sets(&low.weeks[0]),
            "高工作容量第 1 週組數應更多: {} vs {}",
            working_sets(&high.weeks[0]),
            working_sets(&low.weeks[0])
        );
        // 次數落點：同一動作高容量者的下限更高
        let main_low = &low.weeks[0].sessions[0].blocks[1].items[0];
        let main_high = &high.weeks[0].sessions[0].blocks[1].items[0];
        assert_eq!(main_low.exercise_id, main_high.exercise_id);
        assert!(main_high.reps_lo >= main_low.reps_lo);
        assert!(main_high.reps_hi > main_low.reps_hi || main_high.reps_lo > main_low.reps_lo);
    }

    #[test]
    fn recovery_drives_rest_and_deload_depth() {
        let poor = build_plan(&assessment(3), &profile([0.3; 5], dosing(0.5, 0.0, 0.5)));
        let good = build_plan(&assessment(3), &profile([0.3; 5], dosing(0.5, 1.0, 0.5)));
        let rest = |p: &Plan| p.weeks[0].sessions[0].blocks[1].items[0].rest_sec;
        assert!(rest(&poor) > rest(&good), "恢復差者休息應更長");
        // 減載深度：恢復差者減得更深（比例更低）
        let ratio = |p: &Plan| p.weeks[3].volume_scale / p.weeks[2].volume_scale;
        assert!(ratio(&poor) < ratio(&good));
        assert!((ratio(&poor) - DELOAD_MIN).abs() < 1e-3);
        assert!((ratio(&good) - DELOAD_MAX).abs() < 1e-3);
    }

    #[test]
    fn progression_rate_drives_weekly_slope() {
        let slow = build_plan(&assessment(3), &profile([0.3; 5], dosing(0.5, 0.5, 0.0)));
        let fast = build_plan(&assessment(3), &profile([0.3; 5], dosing(0.5, 0.5, 1.0)));
        let slope = |p: &Plan| p.weeks[2].volume_scale - p.weeks[0].volume_scale;
        assert!(slope(&fast) > slope(&slow));
        assert!((progression_step(&dosing(0.5, 0.5, 0.0)) - PROGRESSION_MIN).abs() < 1e-6);
        assert!((progression_step(&dosing(0.5, 0.5, 1.0)) - PROGRESSION_MAX).abs() < 1e-6);
    }

    #[test]
    fn hard_caps_hold_for_extreme_dosing() {
        let plan = build_plan(&assessment(5), &profile([0.6; 5], dosing(1.0, 1.0, 1.0)));
        for w in &plan.weeks {
            assert!(w.volume_scale <= VOLUME_CAP + 1e-6);
            for s in &w.sessions {
                for b in &s.blocks {
                    for p in &b.items {
                        let e = exercises::get(&p.exercise_id).unwrap();
                        assert!(
                            (1..=6).contains(&p.sets),
                            "{} sets={}",
                            p.exercise_id,
                            p.sets
                        );
                        assert!(p.sets <= e.base_sets + 2);
                        assert!(p.reps_lo >= e.rep_lo && p.reps_hi <= e.rep_hi);
                        assert!(p.reps_lo <= p.reps_hi);
                        assert!(p.rest_sec <= REST_MAX);
                        assert!(p.rest_sec >= REST_MIN.min(e.base_rest_sec));
                    }
                }
            }
        }
    }

    // ── 跨週投影 ──

    #[test]
    fn projection_can_raise_stage_but_only_one_per_block() {
        // 就在階段 0 出口邊緣、進步速率滿分 → 後段週次應預先升階
        let p = profile([0.44, 0.39, 0.30, 0.10, 0.10], dosing(0.5, 0.5, 1.0));
        let plan = build_plan(&assessment(3), &p);
        assert_eq!(plan.current_stage, 0);
        assert_eq!(plan.weeks[0].stage, 0, "第 1 週維持目前階段");
        let stages: Vec<u8> = plan.weeks.iter().map(|w| w.stage).collect();
        assert!(
            stages.iter().any(|&s| s == 1),
            "應出現預計升階週: {stages:?}"
        );
        for (i, w) in plan.weeks.iter().enumerate() {
            let block = (i as u8) / 4;
            assert!(
                w.stage <= block,
                "第 {} 週階段 {} 超過 block 上限",
                w.week_index,
                w.stage
            );
        }
        // 非遞減
        assert!(stages.windows(2).all(|p| p[0] <= p[1]), "{stages:?}");
    }

    #[test]
    fn zero_progression_never_projects_stage_up() {
        let p = profile([0.44, 0.39, 0.30, 0.10, 0.10], dosing(0.5, 0.5, 0.0));
        let plan = build_plan(&assessment(3), &p);
        assert!(plan.weeks.iter().all(|w| w.stage == 0));
    }

    #[test]
    fn projection_never_exceeds_one_stage_per_block_even_for_strong_scores() {
        // 階段 2，且階段 2/3 出口門檻近在咫尺：不設限的話兩週內就會投影到階段 4
        let s = scores([0.6, 0.6, 0.54, 0.59, 0.44]);
        assert_eq!(stage_for(&s), 2);
        let d = dosing(0.5, 0.5, 1.0);
        let unconstrained = project_scores(&s, &d, 6, 4);
        assert_eq!(stage_for(&unconstrained), 4, "無上限時應投影到階段 4");
        for block in 0..3u8 {
            let proj = project_scores(&s, &d, 3 * (block + 1), block);
            assert_eq!(stage_for(&proj), (2 + block).min(4), "block {block}");
        }
    }

    // ── 錨點週與強制減載（修復：回報後的減載必須真正套用到課表）──

    #[test]
    fn forced_deload_applies_to_next_week() {
        let p = profile([0.3; 5], MID);
        let plan = build_plan_with(
            &assessment(3),
            &p,
            PlanOptions {
                next_week: 2,
                force_deload: true,
            },
        );
        assert_eq!(plan.next_week, 2);
        let w2 = &plan.weeks[1];
        assert!(w2.is_deload);
        assert_eq!(w2.deload_kind, Some(DeloadKind::Forced));
        assert!(w2.focus_zh.contains("強制減載"));
        let w3 = &plan.weeks[2];
        assert!(!w3.is_deload);
        assert!(
            working_sets(w2) < working_sets(w3),
            "強制減載週組數應低於下一負荷週"
        );
        // 第 4 週仍是排程減載
        assert_eq!(plan.weeks[3].deload_kind, Some(DeloadKind::Scheduled));
    }

    #[test]
    fn forced_deload_on_scheduled_week_is_marked_forced() {
        let plan = build_plan_with(
            &assessment(3),
            &profile([0.3; 5], MID),
            PlanOptions {
                next_week: 4,
                force_deload: true,
            },
        );
        assert_eq!(plan.weeks[3].deload_kind, Some(DeloadKind::Forced));
    }

    #[test]
    fn anchor_week_resets_progression() {
        let p = profile([0.3; 5], dosing(0.5, 0.5, 1.0));
        let fresh = build_plan(&assessment(3), &p);
        let resumed = build_plan_with(
            &assessment(3),
            &p,
            PlanOptions {
                next_week: 6,
                force_deload: false,
            },
        );
        // 錨點週從起始係數重新累計
        assert!((resumed.weeks[5].volume_scale - fresh.weeks[0].volume_scale).abs() < 1e-6);
        // 過去週次不投影、不加量
        for w in &resumed.weeks[..5] {
            assert_eq!(w.projected_scores, p.scores.clamped());
        }
    }

    #[test]
    fn rep_window_is_within_bounds_and_monotonic() {
        for (lo, hi) in [(8u16, 12u16), (3, 5), (20, 45), (10, 10), (2, 4)] {
            let mut prev = (lo, lo);
            for i in 0..=10 {
                let pos = i as f32 / 10.0;
                let (a, b) = rep_window(lo, hi, pos);
                assert!((lo..=hi).contains(&a), "{lo}-{hi} @ {pos}: {a}-{b}");
                assert!((a..=hi).contains(&b), "{lo}-{hi} @ {pos}: {a}-{b}");
                assert!(a >= prev.0 && b >= prev.1, "非單調 {lo}-{hi} @ {pos}");
                prev = (a, b);
            }
            assert_eq!(rep_window(lo, hi, 0.0).0, lo);
            assert_eq!(rep_window(lo, hi, 1.0).1, hi);
        }
    }
}
