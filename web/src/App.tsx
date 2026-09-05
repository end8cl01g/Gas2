import { useEffect, useState } from 'preact/hooks';
import type {
  Assessment,
  ChangeNote,
  PersistState,
  Plan,
  RecalibrateResponse,
  WeeklyLog,
} from './types';
import { getEngine } from './engine';
import { loadState, saveState } from './storage';
import { Wizard } from './Wizard';
import { PlanView } from './PlanView';
import { LogSheet } from './LogSheet';
import { SettingsSheet } from './SettingsSheet';

type Screen = 'loading' | 'welcome' | 'wizard' | 'plan';

export function App() {
  const [state, setState] = useState<PersistState>(() => loadState());
  const [screen, setScreen] = useState<Screen>('loading');
  const [changes, setChanges] = useState<ChangeNote[] | null>(null);
  const [logOpen, setLogOpen] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);

  useEffect(() => {
    (async () => {
      const engine = await getEngine();
      let st = loadState();
      // 舊版 localStorage 課表（無劑量參數／錨點週）→ 以目前引擎重算一次
      if (st.assessment && (!st.plan || !st.plan.dosing || !st.plan.nextWeek)) {
        const plan = JSON.parse(engine.assess(JSON.stringify(st.assessment))) as Plan;
        st = { ...st, plan, weights: engine.export_weights() };
        saveState(st);
        setChanges([{ kind: 'migrate', messageZh: '課表已依新版神經網絡（含劑量參數）重算' }]);
      }
      setState(st);
      setScreen(st.assessment && st.plan ? 'plan' : 'welcome');
    })();
  }, []);

  async function handleAssessment(a: Assessment) {
    const engine = await getEngine();
    const plan = JSON.parse(engine.assess(JSON.stringify(a))) as Plan;
    const next: PersistState = {
      weights: engine.export_weights(),
      assessment: a,
      plan,
      logs: state.logs,
    };
    setState(next);
    saveState(next);
    setChanges(null);
    setScreen('plan');
  }

  async function handleLog(log: WeeklyLog) {
    const engine = await getEngine();
    const resp = JSON.parse(engine.recalibrate(JSON.stringify(log))) as RecalibrateResponse;
    const next: PersistState = {
      weights: resp.weights,
      assessment: state.assessment,
      plan: resp.plan,
      logs: [...state.logs.filter((l) => l.weekIndex !== log.weekIndex), log],
    };
    setState(next);
    saveState(next);
    setLogOpen(false);
    setChanges(resp.changes);
  }

  async function handleImport(data: PersistState) {
    const engine = await getEngine();
    let notes: ChangeNote[] | null = null;
    if (data.weights) {
      try {
        engine.load_weights(data.weights);
      } catch {
        // 舊版權重（輸出維度不同）：重置回基線，並提示
        engine.reset_weights();
        data = { ...data, weights: engine.export_weights() };
        notes = [{ kind: 'reset', messageZh: '備份內的權重為舊版格式，已改用目前基線權重' }];
      }
    }
    // 舊版課表缺少劑量參數／錨點週 → 以目前引擎重算，避免畫面欄位缺漏
    if (data.assessment && (!data.plan || !data.plan.dosing || !data.plan.nextWeek)) {
      const plan = JSON.parse(engine.assess(JSON.stringify(data.assessment))) as Plan;
      data = { ...data, plan, weights: engine.export_weights() };
      notes = [...(notes ?? []), { kind: 'migrate', messageZh: '課表已依新版神經網絡（含劑量參數）重算' }];
    }
    setState(data);
    saveState(data);
    setSettingsOpen(false);
    setChanges(notes);
    setScreen(data.assessment && data.plan ? 'plan' : 'welcome');
  }

  async function handleResetWeights() {
    const engine = await getEngine();
    engine.reset_weights();
    const next: PersistState = { ...state, weights: engine.export_weights() };
    setState(next);
    saveState(next);
    setSettingsOpen(false);
    setChanges([{ kind: 'reset', messageZh: '已重置回基線權重（神經網絡回到出廠狀態）' }]);
  }

  if (screen === 'loading') {
    return <div class="loading">載入神經網絡引擎…</div>;
  }

  if (screen === 'welcome') {
    return (
      <div class="container">
        <header class="hero">
          <img src="./icons/icon-192.png" alt="" width="96" height="96" />
          <h1>PTH 倒立教練</h1>
          <p class="tagline">
            Rust 神經網絡 × 規劃器
            <br />
            為你算出專屬的 Press to Handstand 路徑
          </p>
        </header>
        <button class="btn primary big" data-testid="start-assessment" onClick={() => setScreen('wizard')}>
          開始體能測試
        </button>
        <p class="muted center">約 5 分鐘・不需攝影機・全程瀏覽器本地運算</p>
        <ul class="feature-list">
          <li>🧠 手寫神經網絡依你的體能評分（非 AI 服務）</li>
          <li>📅 12 週週期化課表，含減載與退階方案</li>
          <li>🔁 每週回報訓練結果，路徑自動微調（越用越準）</li>
          <li>📴 離線可用，資料只存在你的手機</li>
        </ul>
      </div>
    );
  }

  if (screen === 'wizard') {
    return <Wizard initial={state.assessment} onSubmit={handleAssessment} />;
  }

  const lastLog = state.logs.length > 0 ? state.logs[state.logs.length - 1] : null;
  const plan = state.plan as Plan;
  // 回報週次以課表錨點為準（引擎在回報後把 nextWeek 推進到 n+1）；舊版課表退回用回報紀錄推算
  const nextLogWeek =
    plan.nextWeek ?? (lastLog ? Math.min(lastLog.weekIndex + 1, plan.totalWeeks) : 1);
  return (
    <div>
      <PlanView
        plan={plan}
        changes={changes}
        onDismissChanges={() => setChanges(null)}
        onOpenLog={() => setLogOpen(true)}
        onOpenSettings={() => setSettingsOpen(true)}
        onReassess={() => setScreen('wizard')}
        loggedWeeks={state.logs.map((l) => l.weekIndex)}
      />
      {logOpen ? (
        <LogSheet
          weekIndex={nextLogWeek}
          defaultPlanned={plan.summary.sessionsPerWeek}
          onSubmit={handleLog}
          onClose={() => setLogOpen(false)}
        />
      ) : null}
      {settingsOpen ? (
        <SettingsSheet
          state={state}
          onClose={() => setSettingsOpen(false)}
          onImport={handleImport}
          onResetWeights={handleResetWeights}
        />
      ) : null}
    </div>
  );
}
