//! 資料模型：體測輸入、每週回報、課表輸出。
//! 所有 JSON 欄位採 camelCase，與前端 TypeScript 型別一一對應。

use serde::{Deserialize, Serialize};

/// 神經網絡輸入特徵數
pub const INPUT_FEATURES: usize = 12;
/// 神經網絡輸出能力評分數
pub const OUTPUT_SCORES: usize = 5;
/// 課表總週數
pub const TOTAL_WEEKS: u8 = 12;

pub const SCORE_KEYS: [&str; OUTPUT_SCORES] = [
    "basePush",
    "coreControl",
    "balanceSkill",
    "overheadPress",
    "compressionPower",
];

pub const SCORE_NAMES_ZH: [&str; OUTPUT_SCORES] = [
    "基礎推力",
    "核心控制",
    "倒立平衡",
    "上肢推撐",
    "壓撐爆發",
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
}

/// 五項能力評分（0–1），由神經網絡輸出
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
    pub fn from_array(a: [f32; OUTPUT_SCORES]) -> Self {
        Self {
            base_push: a[0],
            core_control: a[1],
            balance_skill: a[2],
            overhead_press: a[3],
            compression_power: a[4],
        }
    }

    pub fn to_array(&self) -> [f32; OUTPUT_SCORES] {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Prescription {
    pub exercise_id: String,
    pub name_zh: String,
    pub cues_zh: Vec<String>,
    pub sets: u8,
    pub reps: String,
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
    pub stage: u8,
    pub stage_name_zh: String,
    pub is_deload: bool,
    pub sessions_per_week: u8,
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
            plank_sec: 99_999,
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
    }
}
