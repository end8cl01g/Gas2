//! 資料模型：體測輸入、每週回報、神經網絡輸出（能力評分＋劑量參數）、課表輸出。
//! 所有 JSON 欄位採 camelCase，與前端 TypeScript 型別一一對應。

use serde::{Deserialize, Serialize};

/// 神經網絡輸入特徵數
pub const INPUT_FEATURES: usize = 12;
/// 能力評分維度（基礎推力、核心控制、倒立平衡、上肢推撐、壓撐爆發）
pub const ABILITY_DIMS: usize = 5;
/// 劑量參數維度（工作容量、恢復力、進步速率）
pub const DOSING_DIMS: usize = 3;
/// 神經網絡輸出總維度：能力評分 + 劑量參數
pub const OUTPUT_DIMS: usize = ABILITY_DIMS + DOSING_DIMS;
/// 課表總週數
pub const TOTAL_WEEKS: u8 = 12;

pub const SCORE_KEYS: [&str; ABILITY_DIMS] = [
    "basePush",
    "coreControl",
    "balanceSkill",
    "overheadPress",
    "compressionPower",
];

pub const SCORE_NAMES_ZH: [&str; ABILITY_DIMS] =
    ["基礎推力", "核心控制", "倒立平衡", "上肢推撐", "壓撐爆發"];

pub const DOSING_KEYS: [&str; DOSING_DIMS] = ["workCapacity", "recovery", "progressionRate"];

pub const DOSING_NAMES_ZH: [&str; DOSING_DIMS] = ["工作容量", "恢復力", "進步速率"];

/// 神經網絡全部輸出鍵（順序 = 輸出層索引）
pub const OUTPUT_KEYS: [&str; OUTPUT_DIMS] = [
    "basePush",
    "coreControl",
    "balanceSkill",
    "overheadPress",
    "compressionPower",
    "workCapacity",
    "recovery",
    "progressionRate",
];

fn clampf(x: f32, lo: f32, hi: f32) -> f32 {
    x.clamp(lo, hi)
}

/// 體能測試結果（全部為自評/計時/計數，不使用攝影機）
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Assessment {
    /// 肩屈活動度自評 0–5（躺姿手臂貼地上舉的距離等級）
    pub shoulder_mobility: u8,
    /// 手腕活動度自評 0–5
    pub wrist_mobility: u8,
    /// 平板支撐最長秒數
    pub plank_sec: u16,
    /// 空心支撐（hollow hold）最長秒數
    pub hollow_sec: u16,
    /// 伏地挺身最大反覆次數
    pub pushup_reps: u8,
    /// 折刀伏地挺身（pike push-up）最大次數
    pub pike_pushup_reps: u8,
    /// 壁走（wall walk）最大次數
    pub wall_walk_reps: u8,
    /// 靠牆倒立支撐最長秒數
    pub wall_hs_hold_sec: u16,
    /// 靠牆倒立俯臥撐最大次數
    pub wall_hspu_reps: u8,
    pub bodyweight_kg: f32,
    pub height_cm: f32,
    /// 每週可訓練天數 1–7
    pub days_per_week: u8,
    /// 訓練年資自評 0–3（0=全新 … 3=三年以上）
    pub experience: u8,
}

impl Assessment {
    pub fn sanitized(&self) -> Self {
        Self {
            shoulder_mobility: self.shoulder_mobility.min(5),
            wrist_mobility: self.wrist_mobility.min(5),
            plank_sec: self.plank_sec.min(180),
            hollow_sec: self.hollow_sec.min(120),
            pushup_reps: self.pushup_reps.min(50),
            pike_pushup_reps: self.pike_pushup_reps.min(20),
            wall_walk_reps: self.wall_walk_reps.min(10),
            wall_hs_hold_sec: self.wall_hs_hold_sec.min(120),
            wall_hspu_reps: self.wall_hspu_reps.min(12),
            bodyweight_kg: clampf(self.bodyweight_kg, 25.0, 250.0),
            height_cm: clampf(self.height_cm, 100.0, 230.0),
            days_per_week: self.days_per_week.clamp(1, 7),
            experience: self.experience.min(3),
        }
    }

    /// 體重(kg) / 身高(cm)，典型範圍約 0.30–0.65
    pub fn bodyweight_ratio(&self) -> f32 {
        self.bodyweight_kg / self.height_cm
    }

    /// 標準化特徵向量，每維皆夾限於 [0,1]
    pub fn features(&self) -> [f32; INPUT_FEATURES] {
        let a = self.sanitized();
        let bw = (a.bodyweight_ratio() - 0.30) / 0.35;
        [
            a.shoulder_mobility as f32 / 5.0,
            a.wrist_mobility as f32 / 5.0,
            a.plank_sec as f32 / 180.0,
            a.hollow_sec as f32 / 120.0,
            a.pushup_reps as f32 / 50.0,
            a.pike_pushup_reps as f32 / 20.0,
            a.wall_walk_reps as f32 / 10.0,
            a.wall_hs_hold_sec as f32 / 120.0,
            a.wall_hspu_reps as f32 / 12.0,
            clampf(bw, 0.0, 1.0),
            a.days_per_week as f32 / 7.0,
            a.experience as f32 / 3.0,
        ]
    }
}

/// 每週訓練主觀難度回報
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Focus {
    TooEasy,
    Ok,
    TooHard,
}

/// 疼痛部位回報
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PainArea {
    Wrist,
    Shoulder,
    LowerBack,
}

impl PainArea {
    pub fn zh(&self) -> &'static str {
        match self {
            PainArea::Wrist => "手腕",
            PainArea::Shoulder => "肩膀",
            PainArea::LowerBack => "下背",
        }
    }
}

/// 一週訓練回報（驅動線上微調）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeeklyLog {
    /// 週次 1–12
    pub week_index: u8,
    pub sessions_completed: u8,
    pub sessions_planned: u8,
    pub focus: Focus,
    #[serde(default)]
    pub pain: Vec<PainArea>,
    #[serde(default)]
    pub notes: Option<String>,
}

impl WeeklyLog {
    pub fn sanitized(&self) -> Self {
        Self {
            week_index: self.week_index.clamp(1, TOTAL_WEEKS),
            sessions_completed: self.sessions_completed.min(14),
            sessions_planned: self.sessions_planned.clamp(1, 14),
            focus: self.focus,
            pain: self.pain.clone(),
            notes: self.notes.clone(),
        }
    }

    /// 出席率 completed/planned，夾限 [0,1]
    pub fn adherence(&self) -> f32 {
        let l = self.sanitized();
        clampf(
            l.sessions_completed as f32 / l.sessions_planned.max(1) as f32,
            0.0,
            1.0,
        )
    }

    /// 是否觸發下一週強制減載（硬約束）：任何疼痛，或「太難」且出席率 < 50%
    pub fn force_deload(&self) -> bool {
        !self.pain.is_empty() || (self.focus == Focus::TooHard && self.adherence() < 0.5)
    }

    /// 下一個要執行的週次（1–12）
    pub fn next_week(&self) -> u8 {
        (self.sanitized().week_index + 1).min(TOTAL_WEEKS)
    }
}

/// 五項能力評分（0–1），神經網絡輸出前五維
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Scores {
    pub base_push: f32,
    pub core_control: f32,
    pub balance_skill: f32,
    pub overhead_press: f32,
    pub compression_power: f32,
}

impl Scores {
    pub fn from_array(a: [f32; ABILITY_DIMS]) -> Self {
        Self {
            base_push: a[0],
            core_control: a[1],
            balance_skill: a[2],
            overhead_press: a[3],
            compression_power: a[4],
        }
    }

    pub fn to_array(&self) -> [f32; ABILITY_DIMS] {
        [
            self.base_push,
            self.core_control,
            self.balance_skill,
            self.overhead_press,
            self.compression_power,
        ]
    }

    pub fn clamped(&self) -> Self {
        Self::from_array(self.to_array().map(|v| clampf(v, 0.0, 1.0)))
    }
}

/// 三項劑量參數（0–1），神經網絡輸出後三維。
/// 規劃器據此決定組數、次數落點、組間休息、漸進斜率、減載深度與跨週升階投影。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Dosing {
    /// 工作容量 → 組數係數與次數落點
    pub work_capacity: f32,
    /// 恢復力 → 組間休息長短與減載深度
    pub recovery: f32,
    /// 進步速率 → 每負荷週漸進斜率與跨週升階投影
    pub progression_rate: f32,
}

impl Dosing {
    pub fn from_array(a: [f32; DOSING_DIMS]) -> Self {
        Self {
            work_capacity: a[0],
            recovery: a[1],
            progression_rate: a[2],
        }
    }

    pub fn to_array(&self) -> [f32; DOSING_DIMS] {
        [self.work_capacity, self.recovery, self.progression_rate]
    }

    pub fn clamped(&self) -> Self {
        Self::from_array(self.to_array().map(|v| clampf(v, 0.0, 1.0)))
    }
}

/// 神經網絡完整輸出：五項能力評分 + 三項劑量參數
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub scores: Scores,
    pub dosing: Dosing,
}

impl Profile {
    pub fn from_array(a: [f32; OUTPUT_DIMS]) -> Self {
        let mut s = [0.0f32; ABILITY_DIMS];
        s.copy_from_slice(&a[..ABILITY_DIMS]);
        let mut d = [0.0f32; DOSING_DIMS];
        d.copy_from_slice(&a[ABILITY_DIMS..]);
        Self {
            scores: Scores::from_array(s),
            dosing: Dosing::from_array(d),
        }
    }

    pub fn to_array(&self) -> [f32; OUTPUT_DIMS] {
        let mut out = [0.0f32; OUTPUT_DIMS];
        out[..ABILITY_DIMS].copy_from_slice(&self.scores.to_array());
        out[ABILITY_DIMS..].copy_from_slice(&self.dosing.to_array());
        out
    }

    pub fn clamped(&self) -> Self {
        Self {
            scores: self.scores.clamped(),
            dosing: self.dosing.clamped(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BlockKind {
    Warmup,
    Main,
    Skill,
    Core,
    Accessory,
    Mobility,
}

impl BlockKind {
    pub fn zh(&self) -> &'static str {
        match self {
            BlockKind::Warmup => "熱身",
            BlockKind::Main => "主課表",
            BlockKind::Skill => "技能",
            BlockKind::Core => "核心",
            BlockKind::Accessory => "輔助",
            BlockKind::Mobility => "活動度",
        }
    }
}

/// 次數單位
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RepUnit {
    Reps,
    Seconds,
    PerSide,
    Attempts,
    Circles,
}

impl RepUnit {
    /// 顯示字串：例「8–10 次」「30–40 秒」「每側 8 次」
    pub fn format(&self, lo: u16, hi: u16) -> String {
        let n = if lo == hi {
            format!("{lo}")
        } else {
            format!("{lo}–{hi}")
        };
        match self {
            RepUnit::Reps => format!("{n} 次"),
            RepUnit::Seconds => format!("{n} 秒"),
            RepUnit::PerSide => format!("每側 {n} 次"),
            RepUnit::Attempts => format!("{n} 次嘗試"),
            RepUnit::Circles => format!("{n} 圈"),
        }
    }
}

/// 減載種類：排程（第 4/8/12 週）或回報觸發（疼痛／過難）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DeloadKind {
    Scheduled,
    Forced,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Prescription {
    pub exercise_id: String,
    pub name_zh: String,
    pub cues_zh: Vec<String>,
    pub sets: u8,
    /// 顯示用劑量字串（由 reps_lo/reps_hi/unit 組成）
    pub reps: String,
    pub reps_lo: u16,
    pub reps_hi: u16,
    pub unit: RepUnit,
    pub rest_sec: u16,
    pub regression_zh: String,
    pub progression_zh: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Block {
    pub kind: BlockKind,
    pub items: Vec<Prescription>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub label_zh: String,
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanWeek {
    pub week_index: u8,
    /// 本週階段（可能高於目前階段：依進步速率投影的預計升階）
    pub stage: u8,
    pub stage_name_zh: String,
    pub is_deload: bool,
    #[serde(default)]
    pub deload_kind: Option<DeloadKind>,
    pub sessions_per_week: u8,
    /// 本週訓練量係數（組數縮放，含減載）
    pub volume_scale: f32,
    /// 本週投影能力評分（錨點週之前 = 目前評分）
    pub projected_scores: Scores,
    pub focus_zh: String,
    pub sessions: Vec<Session>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanSummary {
    pub stage_name_zh: String,
    pub goal_zh: String,
    pub sessions_per_week: u8,
    pub note_zh: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    pub total_weeks: u8,
    pub current_stage: u8,
    pub scores: Scores,
    pub dosing: Dosing,
    /// 下一個要執行的週次（回報第 n 週後 = n+1）
    pub next_week: u8,
    pub summary: PlanSummary,
    pub weeks: Vec<PlanWeek>,
}

/// 微調後向使用者說明「改了什麼、為什麼」
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeNote {
    pub kind: String,
    pub message_zh: String,
}

/// `recalibrate` 的完整回應：新課表＋更新後權重＋變更說明
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecalibrateResponse {
    pub plan: Plan,
    pub scores: Scores,
    pub dosing: Dosing,
    pub weights: String,
    pub changes: Vec<ChangeNote>,
    pub stage_changed: bool,
    pub force_deload: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn features_are_normalized() {
        let a = Assessment {
            shoulder_mobility: 5,
            wrist_mobility: 5,
            plank_sec: 180,
            hollow_sec: 120,
            pushup_reps: 50,
            pike_pushup_reps: 20,
            wall_walk_reps: 10,
            wall_hs_hold_sec: 120,
            wall_hspu_reps: 12,
            bodyweight_kg: 70.0,
            height_cm: 175.0,
            days_per_week: 7,
            experience: 3,
        };
        for v in a.features() {
            assert!((0.0..=1.0).contains(&v), "feature out of range: {v}");
        }
    }

    #[test]
    fn sanitized_clamps_crazy_input() {
        let a = Assessment {
            shoulder_mobility: 200,
            wrist_mobility: 3,
            plank_sec: 60_000,
            hollow_sec: 0,
            pushup_reps: 255,
            pike_pushup_reps: 0,
            wall_walk_reps: 0,
            wall_hs_hold_sec: 0,
            wall_hspu_reps: 0,
            bodyweight_kg: 9999.0,
            height_cm: 0.0,
            days_per_week: 42,
            experience: 9,
        };
        let s = a.sanitized();
        assert_eq!(s.shoulder_mobility, 5);
        assert_eq!(s.plank_sec, 180);
        assert_eq!(s.pushup_reps, 50);
        assert_eq!(s.days_per_week, 7);
        assert_eq!(s.experience, 3);
        assert!(s.bodyweight_kg <= 250.0);
        assert!(s.height_cm >= 100.0);
    }

    #[test]
    fn assessment_json_roundtrip_camel_case() {
        let a = Assessment {
            shoulder_mobility: 3,
            wrist_mobility: 2,
            plank_sec: 60,
            hollow_sec: 30,
            pushup_reps: 15,
            pike_pushup_reps: 5,
            wall_walk_reps: 3,
            wall_hs_hold_sec: 20,
            wall_hspu_reps: 1,
            bodyweight_kg: 68.0,
            height_cm: 172.0,
            days_per_week: 3,
            experience: 1,
        };
        let json = serde_json::to_string(&a).unwrap();
        assert!(json.contains("\"shoulderMobility\""));
        let back: Assessment = serde_json::from_str(&json).unwrap();
        assert_eq!(a, back);
    }

    #[test]
    fn weekly_log_adherence() {
        let log = WeeklyLog {
            week_index: 2,
            sessions_completed: 1,
            sessions_planned: 4,
            focus: Focus::Ok,
            pain: vec![],
            notes: None,
        };
        assert!((log.adherence() - 0.25).abs() < 1e-6);
        assert!(!log.force_deload(), "剛好＋低出席不觸發強制減載");
        assert_eq!(log.next_week(), 3);
    }

    #[test]
    fn force_deload_rule() {
        let mut log = WeeklyLog {
            week_index: 12,
            sessions_completed: 1,
            sessions_planned: 4,
            focus: Focus::TooHard,
            pain: vec![],
            notes: None,
        };
        assert!(log.force_deload(), "太難＋出席 < 50% 觸發");
        assert_eq!(log.next_week(), TOTAL_WEEKS, "最後一週不超出範圍");
        log.sessions_completed = 4;
        assert!(!log.force_deload(), "太難但出席率高不觸發");
        log.pain = vec![PainArea::Wrist];
        assert!(log.force_deload(), "任何疼痛觸發");
    }

    #[test]
    fn profile_array_roundtrip_and_clamp() {
        let arr = [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 1.4];
        let p = Profile::from_array(arr);
        assert!((p.scores.compression_power - 0.5).abs() < 1e-6);
        assert!((p.dosing.work_capacity - 0.6).abs() < 1e-6);
        assert!((p.dosing.progression_rate - 1.4).abs() < 1e-6);
        assert_eq!(p.to_array(), arr);
        let c = p.clamped();
        assert!((c.dosing.progression_rate - 1.0).abs() < 1e-6);
        let json = serde_json::to_string(&c).unwrap();
        assert!(json.contains("\"workCapacity\""));
        assert!(json.contains("\"progressionRate\""));
        let back: Profile = serde_json::from_str(&json).unwrap();
        assert_eq!(back, c);
    }

    #[test]
    fn rep_unit_format() {
        assert_eq!(RepUnit::Reps.format(8, 10), "8–10 次");
        assert_eq!(RepUnit::Reps.format(4, 4), "4 次");
        assert_eq!(RepUnit::Seconds.format(30, 40), "30–40 秒");
        assert_eq!(RepUnit::PerSide.format(8, 9), "每側 8–9 次");
        assert_eq!(RepUnit::Attempts.format(10, 10), "10 次嘗試");
        assert_eq!(RepUnit::Circles.format(10, 10), "10 圈");
    }

    #[test]
    fn output_keys_align_with_struct_order() {
        assert_eq!(&OUTPUT_KEYS[..ABILITY_DIMS], &SCORE_KEYS[..]);
        assert_eq!(&OUTPUT_KEYS[ABILITY_DIMS..], &DOSING_KEYS[..]);
        assert_eq!(SCORE_NAMES_ZH.len(), ABILITY_DIMS);
        assert_eq!(DOSING_NAMES_ZH.len(), DOSING_DIMS);
    }
}
