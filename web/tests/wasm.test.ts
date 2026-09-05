// Node 端整合測試：直接跑 wasm（nodejs target），驗證 Rust 引擎的完整閉環。
// 匯入採防禦式寫法：同時相容本地 stub（ESM）與 CI 的 wasm-bindgen nodejs 產物（CJS）。
import { beforeAll, describe, expect, it } from 'vitest';
import * as wasmPkg from '../src/wasm-node/gas2_wasm.js';
import type { Dosing, Plan, RecalibrateResponse, Scores } from '../src/types';

type EngineLike = {
  assess: (json: string) => string;
  recalibrate: (json: string) => string;
  export_weights: () => string;
  load_weights: (json: string) => void;
  reset_weights: () => void;
  current_scores: () => string | null;
  current_dosing: () => string | null;
};

/** 主課表／技能／核心／輔助 block 的總組數（排除熱身與活動度） */
function workingSets(w: Plan['weeks'][number]): number {
  return w.sessions
    .flatMap((s) => s.blocks)
    .filter((b) => b.kind !== 'warmup' && b.kind !== 'mobility')
    .flatMap((b) => b.items)
    .reduce((acc, p) => acc + p.sets, 0);
}

const pkg = wasmPkg as unknown as Record<string, unknown>;

function typeofKey(o: Record<string, unknown>, k: string): string {
  return `${k}:${typeof o[k]}`;
}

// 自癒式解析：相容 ESM / CJS-interop / namespace 展開等各種匯出形態，
// 都失敗時把實際匯出清單放進錯誤訊息（CI annotation 可診斷）。
function resolveInit(): () => unknown {
  const scopes: Record<string, unknown>[] = [pkg];
  if (pkg.default && typeof pkg.default === 'object') {
    scopes.push(pkg.default as Record<string, unknown>);
  }
  for (const scope of scopes) {
    for (const [k, v] of Object.entries(scope)) {
      if (typeof v === 'function' && /^init|^default$/.test(k)) return v as () => unknown;
    }
  }
  for (const scope of scopes) {
    for (const [k, v] of Object.entries(scope)) {
      if (typeof v === 'function' && /init/i.test(k)) return v as () => unknown;
    }
  }
  const manifest = scopes.map((s) => Object.keys(s).map((k) => typeofKey(s, k)).join(',')).join(' | ');
  throw new Error(`無法解析 wasm init 函式。exports = ${manifest}`);
}

function resolveEngine(): new () => EngineLike {
  if (typeof pkg.Engine === 'function') return pkg.Engine as new () => EngineLike;
  if (pkg.default && typeof pkg.default === 'object') {
    const d = pkg.default as Record<string, unknown>;
    if (typeof d.Engine === 'function') return d.Engine as new () => EngineLike;
  }
  throw new Error(`無法解析 Engine 類別。exports = ${Object.keys(pkg).join(',')}`);
}

const initFn = resolveInit();
const EngineCtor = resolveEngine();

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
  it('assess 產生 12 週課表，欄位齊全（含劑量參數與結構化次數）', () => {
    const e = new EngineCtor();
    const plan = JSON.parse(e.assess(JSON.stringify(SAMPLE))) as Plan;
    expect(plan.totalWeeks).toBe(12);
    expect(plan.weeks).toHaveLength(12);
    expect(plan.nextWeek).toBe(1);
    expect(plan.currentStage).toBeGreaterThanOrEqual(0);
    expect(plan.currentStage).toBeLessThanOrEqual(4);
    for (const k of ['workCapacity', 'recovery', 'progressionRate'] as (keyof Dosing)[]) {
      expect(plan.dosing[k]).toBeGreaterThanOrEqual(0);
      expect(plan.dosing[k]).toBeLessThanOrEqual(1);
    }
    expect(JSON.parse(e.current_dosing()!)).toEqual(plan.dosing);
    for (const w of plan.weeks) {
      expect(w.sessionsPerWeek).toBeGreaterThanOrEqual(2);
      expect(w.sessionsPerWeek).toBeLessThanOrEqual(5);
      expect(w.sessions).toHaveLength(w.sessionsPerWeek);
      expect(w.isDeload).toBe(w.weekIndex % 4 === 0);
      expect(w.deloadKind ?? null).toBe(w.isDeload ? 'scheduled' : null);
      expect(w.volumeScale).toBeGreaterThan(0);
      expect(w.volumeScale).toBeLessThanOrEqual(1.5 + 1e-6);
      expect(w.stage).toBeGreaterThanOrEqual(plan.currentStage);
      for (const s of w.sessions) {
        expect(s.blocks.length).toBeGreaterThan(0);
        for (const b of s.blocks) {
          for (const p of b.items) {
            expect(p.sets).toBeGreaterThanOrEqual(1);
            expect(p.sets).toBeLessThanOrEqual(6);
            expect(p.repsLo).toBeGreaterThanOrEqual(1);
            expect(p.repsHi).toBeGreaterThanOrEqual(p.repsLo);
            expect(p.reps.length).toBeGreaterThan(0);
            expect(p.restSec).toBeLessThanOrEqual(240);
            expect(p.cuesZh.length).toBeGreaterThan(0);
          }
        }
      }
    }
    const deloads = plan.weeks.filter((w) => w.isDeload).map((w) => w.weekIndex);
    expect(deloads).toEqual([4, 8, 12]);
    // 漸進：第 3 週訓練量高於第 1 週；減載週低於前一週
    expect(plan.weeks[2].volumeScale).toBeGreaterThan(plan.weeks[0].volumeScale);
    expect(workingSets(plan.weeks[3])).toBeLessThan(workingSets(plan.weeks[2]));
  });

  it('劑量參數驅動課表：高手比新手組數更多、休息更短', () => {
    const weak = { ...SAMPLE, plankSec: 30, hollowSec: 10, pushupReps: 5, experience: 0 };
    const strong = {
      ...SAMPLE,
      shoulderMobility: 5,
      wristMobility: 4,
      plankSec: 150,
      hollowSec: 90,
      pushupReps: 40,
      pikePushupReps: 20,
      wallWalkReps: 8,
      wallHsHoldSec: 80,
      wallHspuReps: 12,
      experience: 3,
    };
    const pw = JSON.parse(new EngineCtor().assess(JSON.stringify(weak))) as Plan;
    const ps = JSON.parse(new EngineCtor().assess(JSON.stringify(strong))) as Plan;
    expect(ps.dosing.workCapacity).toBeGreaterThan(pw.dosing.workCapacity);
    expect(ps.dosing.recovery).toBeGreaterThan(pw.dosing.recovery);
    expect(ps.weeks[0].volumeScale).toBeGreaterThan(pw.weeks[0].volumeScale);
    const restOf = (p: Plan) => p.weeks[0].sessions[0].blocks[1].items[0].restSec;
    expect(restOf(ps)).toBeLessThan(restOf(pw));
  });

  it('recalibrate（太輕鬆）→ 評分與劑量上調且附帶說明', () => {
    const e = new EngineCtor();
    e.assess(JSON.stringify(SAMPLE));
    const before = JSON.parse(e.current_scores()!) as Scores;
    const beforeDosing = JSON.parse(e.current_dosing()!) as Dosing;
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
    expect(resp.changes.some((c) => c.kind === 'dosing')).toBe(true);
    expect(resp.scores.basePush).toBeGreaterThan(before.basePush);
    expect(resp.dosing.workCapacity).toBeGreaterThan(beforeDosing.workCapacity);
    // 安全夾限：單週上調 ≤ 0.12
    for (const k of Object.keys(resp.scores) as (keyof Scores)[]) {
      expect(resp.scores[k] - before[k]).toBeLessThanOrEqual(0.12 + 1e-4);
    }
    // 新課表完整，錨點移到第 2 週
    expect(resp.plan.weeks).toHaveLength(12);
    expect(resp.plan.nextWeek).toBe(2);
    expect(resp.plan.weeks[1].isDeload).toBe(false);
    // 權重可重新載入
    const e2 = new EngineCtor();
    expect(() => e2.load_weights(resp.weights)).not.toThrow();
  });

  it('疼痛回報 → 強制減載真的套用在下一週＋相關評分／恢復力下調', () => {
    const e = new EngineCtor();
    e.assess(JSON.stringify(SAMPLE));
    const before = JSON.parse(e.current_scores()!) as Scores;
    const beforeDosing = JSON.parse(e.current_dosing()!) as Dosing;
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
    expect(resp.dosing.recovery).toBeLessThan(beforeDosing.recovery);
    expect(resp.changes.some((c) => c.kind === 'pain')).toBe(true);
    expect(resp.changes.some((c) => c.kind === 'deload')).toBe(true);
    // 修復：先前 forceDeload 只出現在說明文字，課表本身沒有減載
    const w2 = resp.plan.weeks[1];
    expect(resp.plan.nextWeek).toBe(2);
    expect(w2.isDeload).toBe(true);
    expect(w2.deloadKind).toBe('forced');
    expect(workingSets(w2)).toBeLessThan(workingSets(resp.plan.weeks[2]));
    // 排程減載仍在
    expect(resp.plan.weeks[3].deloadKind).toBe('scheduled');
  });

  it('舊版 5 維輸出權重被拒絕載入（前端據此重置回基線）', () => {
    const e = new EngineCtor();
    const legacy = JSON.parse(e.export_weights()) as { arch: number[]; w3: number[][]; b3: number[] };
    legacy.arch = [legacy.arch[0], legacy.arch[1], legacy.arch[2], 5];
    legacy.w3 = legacy.w3.slice(0, 5);
    legacy.b3 = legacy.b3.slice(0, 5);
    expect(() => e.load_weights(JSON.stringify(legacy))).toThrow();
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
