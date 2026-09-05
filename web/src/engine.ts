// WASM 引擎單例：載入 →（可選）恢復已微調權重
import init, { Engine } from './wasm/gas2_wasm.js';
import { loadState } from './storage';

let enginePromise: Promise<Engine> | null = null;

export function getEngine(): Promise<Engine> {
  if (!enginePromise) {
    enginePromise = (async () => {
      await init();
      const engine = new Engine();
      const { weights } = loadState();
      if (weights) {
        try {
          engine.load_weights(weights);
        } catch {
          // 保存的權重不相容時，重置回基線
          engine.reset_weights();
        }
      }
      return engine;
    })();
  }
  return enginePromise;
}

export type { Engine };
