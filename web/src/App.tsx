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
      await getEngine();
      const st = loadState();
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
    if (data.weights) {
      engine.load_weights(data.weights);
    }
    setState(data);
    saveState(data);
    setSettingsOpen(false);
    setChanges(null);
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
  return (
    <div>
      <PlanView
        plan={state.plan as Plan}
        changes={changes}
        onDismissChanges={() => setChanges(null)}
        onOpenLog={() => setLogOpen(true)}
        onOpenSettings={() => setSettingsOpen(true)}
        onReassess={() => setScreen('wizard')}
        loggedWeeks={state.logs.map((l) => l.weekIndex)}
      />
      {logOpen && lastLog ? (
        <LogSheet
          weekIndex={Math.min(lastLog.weekIndex + 1, (state.plan as Plan).totalWeeks)}
          defaultPlanned={(state.plan as Plan).summary.sessionsPerWeek}
          onSubmit={handleLog}
          onClose={() => setLogOpen(false)}
        />
      ) : null}
      {logOpen && !lastLog ? (
        <LogSheet
          weekIndex={1}
          defaultPlanned={(state.plan as Plan).summary.sessionsPerWeek}
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
