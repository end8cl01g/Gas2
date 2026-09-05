// Node 端整合測試：直接跑 wasm（nodejs target），驗證 Rust 引擎的完整閉環。
// 匯入採防禦式寫法：同時相容本地 stub（ESM）與 CI 的 wasm-bindgen nodejs 產物（CJS）。
import { beforeAll, describe, expect, it } from 'vitest';
import * as wasmPkg from '../src/wasm-node/gas2_wasm.js';
import type { Plan, RecalibrateResponse, Scores } from '../src/types';

type EngineLike = {
  assess: (json: string) => string;
  recalibrate: (json: string) => string;
  export_weights: () => string;
  load_weights: (json: string) => void;
  reset_weights: () => void;
  current_scores: () => string | null;
};

const pkg = wasmPkg as unknown as Record<string, unknown>;
const EngineCtor = pkg.Engine as unknown as new () => EngineLike;
const initFn = (pkg.default ?? pkg.initSync) as (() => Promise<unknown>) | (() => unknown);

beforeAll(async () => {
  await initFn();
});

const SAMPLE = {
  shoulderMobility: 3,
  wristMobility: 2,
  plankSec: 60,
  hollowSec: 30,
  pushupReps: 15,
  pikePushupReps: 5,
  wallWalkReps: 2,
  wallHsHoldSec: 15,
  wallHspuReps: 0,
  bodyweightKg: 68,
  heightCm: 172,
  daysPerWeek: 3,
  experience: 1,
};

describe('wasm engine', () => {
  it('assess 產生 12 週課表，欄位齊全', () => {
    const e = new EngineCtor();
    const plan = JSON.parse(e.assess(JSON.stringify(SAMPLE))) as Plan;
    expect(plan.totalWeeks).toBe(12);
    expect(plan.weeks).toHaveLength(12);
    expect(plan.currentStage).toBeGreaterThanOrEqual(0);
    expect(plan.currentStage).toBeLessThanOrEqual(4);
    for (const w of plan.weeks) {
      expect(w.sessionsPerWeek).toBeGreaterThanOrEqual(2);
      expect(w.sessionsPerWeek).toBeLessThanOrEqual(5);
      expect(w.sessions).toHaveLength(w.sessionsPerWeek);
      expect(w.isDeload).toBe(w.weekIndex % 4 === 0);
      for (const s of w.sessions) {
        expect(s.blocks.length).toBeGreaterThan(0);
        for (const b of s.blocks) {
          for (const p of b.items) {
            expect(p.sets).toBeGreaterThanOrEqual(1);
            expect(p.cuesZh.length).toBeGreaterThan(0);
          }
        }
      }
    }
    const deloads = plan.weeks.filter((w) => w.isDeload).map((w) => w.weekIndex);
    expect(deloads).toEqual([4, 8, 12]);
  });

  it('recalibrate（太輕鬆）→ 評分上調且附帶說明', () => {
    const e = new EngineCtor();
    e.assess(JSON.stringify(SAMPLE));
    const before = JSON.parse(e.current_scores()!) as Scores;
    const resp = JSON.parse(
      e.recalibrate(
        JSON.stringify({
          weekIndex: 1,
          sessionsCompleted: 3,
          sessionsPlanned: 3,
          focus: 'tooEasy',
          pain: [],
          notes: null,
        })
      )
    ) as RecalibrateResponse;
    expect(resp.changes.length).toBeGreaterThan(0);
    expect(resp.changes.some((c) => c.messageZh.includes('太輕鬆'))).toBe(true);
    expect(resp.scores.basePush).toBeGreaterThan(before.basePush);
    // 安全夾限：單週上調 ≤ 0.12
    for (const k of Object.keys(resp.scores) as (keyof Scores)[]) {
      expect(resp.scores[k] - before[k]).toBeLessThanOrEqual(0.12 + 1e-4);
    }
    // 新課表完整
    expect(resp.plan.weeks).toHaveLength(12);
    // 權重可重新載入
    const e2 = new EngineCtor();
    expect(() => e2.load_weights(resp.weights)).not.toThrow();
  });

  it('疼痛回報 → 強制減載＋相關評分下調', () => {
    const e = new EngineCtor();
    e.assess(JSON.stringify(SAMPLE));
    const before = JSON.parse(e.current_scores()!) as Scores;
    const resp = JSON.parse(
      e.recalibrate(
        JSON.stringify({
          weekIndex: 1,
          sessionsCompleted: 2,
          sessionsPlanned: 3,
          focus: 'ok',
          pain: ['shoulder'],
          notes: null,
        })
      )
    ) as RecalibrateResponse;
    expect(resp.forceDeload).toBe(true);
    expect(resp.scores.overheadPress).toBeLessThan(before.overheadPress);
    expect(resp.changes.some((c) => c.kind === 'pain')).toBe(true);
  });

  it('權重匯出/載入 roundtrip 保持推論一致', () => {
    const e = new EngineCtor();
    e.assess(JSON.stringify(SAMPLE));
    const weights = e.export_weights();
    const e2 = new EngineCtor();
    e2.load_weights(weights);
    e2.assess(JSON.stringify(SAMPLE)); // 同權重＋同體測 → 同評分
    expect(e2.current_scores()).toBe(e.current_scores());
  });

  it('重置權重回到基線', () => {
    const e = new EngineCtor();
    e.assess(JSON.stringify(SAMPLE));
    e.recalibrate(
      JSON.stringify({ weekIndex: 1, sessionsCompleted: 3, sessionsPlanned: 3, focus: 'tooEasy', pain: [], notes: null })
    );
    e.reset_weights();
    const e0 = new EngineCtor();
    e0.assess(JSON.stringify(SAMPLE));
    expect(e.current_scores()).toBe(e0.current_scores());
  });
});
