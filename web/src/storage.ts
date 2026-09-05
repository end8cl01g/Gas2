import type { PersistState } from './types';

const KEY = 'pth.v1';

const EMPTY: PersistState = {
  weights: null,
  assessment: null,
  plan: null,
  logs: [],
};

export function loadState(): PersistState {
  try {
    const raw = localStorage.getItem(KEY);
    if (!raw) return { ...EMPTY };
    const parsed = JSON.parse(raw) as Partial<PersistState>;
    return {
      weights: parsed.weights ?? null,
      assessment: parsed.assessment ?? null,
      plan: parsed.plan ?? null,
      logs: Array.isArray(parsed.logs) ? parsed.logs : [],
    };
  } catch {
    return { ...EMPTY };
  }
}

export function saveState(s: PersistState): void {
  try {
    localStorage.setItem(KEY, JSON.stringify(s));
  } catch {
    // 隱私模式等場景下失敗可忽略（功能仍在，僅不持久化）
  }
}
