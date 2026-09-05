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

export interface Prescription {
  exerciseId: string;
  nameZh: string;
  cuesZh: string[];
  sets: number;
  reps: string;
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
  stage: number;
  stageNameZh: string;
  isDeload: boolean;
  sessionsPerWeek: number;
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
