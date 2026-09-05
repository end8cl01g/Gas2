// 與 Rust（gas2-core）schema 一一對應的 TypeScript 型別（JSON 欄位 camelCase）。
// 單一真相來源在 crates/core/src/model.rs；修改時兩邊同步。

export interface Assessment {
  shoulderMobility: number; // 0–5
  wristMobility: number; // 0–5
  plankSec: number; // 0–180
  hollowSec: number; // 0–120
  pushupReps: number; // 0–50
  pikePushupReps: number; // 0–20
  wallWalkReps: number; // 0–10
  wallHsHoldSec: number; // 0–120
  wallHspuReps: number; // 0–12
  bodyweightKg: number;
  heightCm: number;
  daysPerWeek: number; // 1–7
  experience: number; // 0–3
}

export type Focus = 'tooEasy' | 'ok' | 'tooHard';
export type PainArea = 'wrist' | 'shoulder' | 'lowerBack';

export interface WeeklyLog {
  weekIndex: number;
  sessionsCompleted: number;
  sessionsPlanned: number;
  focus: Focus;
  pain: PainArea[];
  notes?: string | null;
}

export interface Scores {
  basePush: number;
  coreControl: number;
  balanceSkill: number;
  overheadPress: number;
  compressionPower: number;
}

/** 神經網絡輸出的劑量參數（0–1）：決定組數、次數落點、休息、漸進斜率、減載深度、跨週升階投影 */
export interface Dosing {
  workCapacity: number;
  recovery: number;
  progressionRate: number;
}

export const DOSING_KEYS = ['workCapacity', 'recovery', 'progressionRate'] as const satisfies (keyof Dosing)[];

export const DOSING_NAMES_ZH: Record<keyof Dosing, string> = {
  workCapacity: '工作容量',
  recovery: '恢復力',
  progressionRate: '進步速率',
};

export const DOSING_HINTS_ZH: Record<keyof Dosing, string> = {
  workCapacity: '組數・次數落點',
  recovery: '組間休息・減載深度',
  progressionRate: '每週漸進・預計升階',
};

export const SCORE_KEYS = [
  'basePush',
  'coreControl',
  'balanceSkill',
  'overheadPress',
  'compressionPower',
] as const satisfies (keyof Scores)[];

export const SCORE_NAMES_ZH: Record<keyof Scores, string> = {
  basePush: '基礎推力',
  coreControl: '核心控制',
  balanceSkill: '倒立平衡',
  overheadPress: '上肢推撐',
  compressionPower: '壓撐爆發',
};

export type BlockKind = 'warmup' | 'main' | 'skill' | 'core' | 'accessory' | 'mobility';
export type RepUnit = 'reps' | 'seconds' | 'perSide' | 'attempts' | 'circles';
export type DeloadKind = 'scheduled' | 'forced';

export interface Prescription {
  exerciseId: string;
  nameZh: string;
  cuesZh: string[];
  sets: number;
  /** 顯示用（由 repsLo/repsHi/unit 組成） */
  reps: string;
  repsLo: number;
  repsHi: number;
  unit: RepUnit;
  restSec: number;
  regressionZh: string;
  progressionZh: string;
}

export interface Block {
  kind: BlockKind;
  items: Prescription[];
}

export interface Session {
  labelZh: string;
  blocks: Block[];
}

export interface PlanWeek {
  weekIndex: number;
  /** 本週階段（可能高於目前階段：依進步速率投影的預計升階） */
  stage: number;
  stageNameZh: string;
  isDeload: boolean;
  deloadKind?: DeloadKind | null;
  sessionsPerWeek: number;
  volumeScale: number;
  projectedScores: Scores;
  focusZh: string;
  sessions: Session[];
}

export interface PlanSummary {
  stageNameZh: string;
  goalZh: string;
  sessionsPerWeek: number;
  noteZh: string;
}

export interface Plan {
  totalWeeks: number;
  currentStage: number;
  scores: Scores;
  dosing: Dosing;
  /** 下一個要執行的週次（回報第 n 週後 = n+1） */
  nextWeek: number;
  summary: PlanSummary;
  weeks: PlanWeek[];
}

export interface ChangeNote {
  kind: string;
  messageZh: string;
}

export interface RecalibrateResponse {
  plan: Plan;
  scores: Scores;
  dosing: Dosing;
  weights: string;
  changes: ChangeNote[];
  stageChanged: boolean;
  forceDeload: boolean;
}

export interface PersistState {
  weights: string | null;
  assessment: Assessment | null;
  plan: Plan | null;
  logs: WeeklyLog[];
}
