import { useRef } from 'preact/hooks';
import { app_info } from './wasm/gas2_wasm.js';
import type { PersistState } from './types';

export function SettingsSheet(props: {
  state: PersistState;
  onClose: () => void;
  onImport: (data: PersistState) => void;
  onResetWeights: () => void;
}) {
  const fileRef = useRef<HTMLInputElement>(null);

  function exportData() {
    const blob = new Blob([JSON.stringify(props.state, null, 2)], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = 'pth-backup.json';
    a.click();
    URL.revokeObjectURL(url);
  }

  async function importFile(file: File) {
    try {
      const data = JSON.parse(await file.text()) as PersistState;
      props.onImport({
        weights: typeof data.weights === 'string' ? data.weights : null,
        assessment: data.assessment ?? null,
        plan: data.plan ?? null,
        logs: Array.isArray(data.logs) ? data.logs : [],
      });
    } catch {
      alert('備份檔格式不正確');
    }
  }

  return (
    <div class="sheet-overlay" role="dialog" aria-modal="true">
      <div class="sheet">
        <h2>設定</h2>
        <p class="muted small">
          引擎：{app_info()}（手寫 MLP・瀏覽器本地推論，無任何 AI 服務）
        </p>

        <button class="btn big" onClick={exportData}>
          匯出資料（課表＋權重 JSON）
        </button>
        <button class="btn big" onClick={() => fileRef.current?.click()}>
          匯入資料
        </button>
        <input
          ref={fileRef}
          type="file"
          accept="application/json"
          hidden
          onChange={(e) => {
            const f = (e.target as HTMLInputElement).files?.[0];
            if (f) importFile(f);
          }}
        />
        <button
          class="btn big danger"
          onClick={() => {
            if (confirm('重置神經網絡權重回基線？你的體測與課表會保留。')) props.onResetWeights();
          }}
        >
          重置網絡權重
        </button>

        <div class="wizard-nav">
          <button class="btn primary big" onClick={props.onClose}>
            關閉
          </button>
        </div>
      </div>
    </div>
  );
}
