import { useEffect, useState } from 'preact/hooks';
import { DOSING_HINTS_ZH, DOSING_KEYS, DOSING_NAMES_ZH, SCORE_KEYS, SCORE_NAMES_ZH } from './types';
import type { ChangeNote, Plan } from './types';

const BLOCK_ZH: Record<string, string> = {
  warmup: '熱身',
  main: '主課表',
  skill: '技能',
  core: '核心',
  accessory: '輔助',
  mobility: '活動度',
};

export function PlanView(props: {
  plan: Plan;
  changes: ChangeNote[] | null;
  onDismissChanges: () => void;
  onOpenLog: () => void;
  onOpenSettings: () => void;
  onReassess: () => void;
  loggedWeeks: number[];
}) {
  const { plan } = props;
  // 預設顯示「下一個要執行的週次」；課表重算（回報後）時跟著跳到新的錨點週
  const anchorIdx = Math.min(Math.max((plan.nextWeek ?? 1) - 1, 0), plan.weeks.length - 1);
  const [weekIdx, setWeekIdx] = useState(anchorIdx);
  useEffect(() => {
    setWeekIdx(anchorIdx);
  }, [anchorIdx, plan]);
  const week = plan.weeks[Math.min(weekIdx, plan.weeks.length - 1)];
  const projectedUp = week.stage > plan.currentStage;

  return (
    <div class="container">
      <header class="topbar">
        <h1>PTH 倒立教練</h1>
        <button class="icon-btn" data-testid="open-settings" onClick={props.onOpenSettings} aria-label="設定">
          ⚙️
        </button>
      </header>

      {props.changes ? (
        <div class="card banner" data-testid="changes-banner">
          <h3>✅ 課表已更新</h3>
          <ul>
            {props.changes.map((c, i) => (
              <li key={i}>{c.messageZh}</li>
            ))}
          </ul>
          <button class="btn small" onClick={props.onDismissChanges}>
            知道了
          </button>
        </div>
      ) : null}

      <section class="card stage">
        <div class="stage-tag">
          階段 {plan.currentStage + 1}/5・{plan.summary.stageNameZh}
        </div>
        <p class="goal">{plan.summary.goalZh}</p>
        <p class="muted small">{plan.summary.noteZh}</p>
      </section>

      <section class="card">
        <h3>能力評分（神經網絡推論）</h3>
        {SCORE_KEYS.map((k) => (
          <div class="score" key={k}>
            <span class="score-name">{SCORE_NAMES_ZH[k]}</span>
            <div class="bar">
              <div class="bar-fill" style={`width:${Math.round(plan.scores[k] * 100)}%`} />
            </div>
            <span class="score-val">{Math.round(plan.scores[k] * 100)}</span>
          </div>
        ))}
        <h3 class="sub" data-testid="dosing-title">
          劑量參數（神經網絡推論）
        </h3>
        <p class="muted small">決定組數、次數落點、組間休息、每週漸進與減載深度</p>
        {DOSING_KEYS.map((k) => (
          <div class="score" key={k} data-testid={`dosing-${k}`}>
            <span class="score-name" title={DOSING_HINTS_ZH[k]}>
              {DOSING_NAMES_ZH[k]}
            </span>
            <div class="bar">
              <div class="bar-fill alt" style={`width:${Math.round(plan.dosing[k] * 100)}%`} />
            </div>
            <span class="score-val">{Math.round(plan.dosing[k] * 100)}</span>
          </div>
        ))}
      </section>

      <section>
        <div class="weeks" role="tablist">
          {plan.weeks.map((w, i) => (
            <button
              key={w.weekIndex}
              class={i === weekIdx ? 'chip on' : 'chip'}
              onClick={() => setWeekIdx(i)}
              title={
                w.deloadKind === 'forced'
                  ? '強制減載週'
                  : w.isDeload
                    ? '減載週'
                    : w.stage > plan.currentStage
                      ? '預計升階'
                      : undefined
              }
            >
              {w.weekIndex}
              {w.isDeload ? '↓' : w.stage > plan.currentStage ? '↑' : ''}
              {props.loggedWeeks.includes(w.weekIndex) ? '·' : ''}
            </button>
          ))}
        </div>

        <div class="card week-card">
          <h2 data-testid="week-title">
            第 {week.weekIndex} 週
            {week.deloadKind === 'forced' ? (
              <span class="badge danger" data-testid="deload-badge">
                強制減載
              </span>
            ) : week.isDeload ? (
              <span class="badge warn" data-testid="deload-badge">
                減載
              </span>
            ) : null}
            {projectedUp ? <span class="badge up">預計升階</span> : null}
          </h2>
          <p class="focus">{week.focusZh}</p>
          <p class="muted small">
            每週 {week.sessionsPerWeek} 次訓練・第 {week.stage + 1} 階段（{week.stageNameZh}）・訓練量 ×
            {week.volumeScale.toFixed(2)}
          </p>

          {week.sessions.map((s, si) => (
            <details class="session" key={si} open={si === 0}>
              <summary>{s.labelZh}</summary>
              {s.blocks.map((b, bi) => (
                <div class="block" key={bi}>
                  <h4>{BLOCK_ZH[b.kind] ?? b.kind}</h4>
                  {b.items.map((p) => (
                    <details class="exercise" key={p.exerciseId}>
                      <summary>
                        <strong>{p.nameZh}</strong>
                        <span class="dose">
                          {p.sets} × {p.reps}
                          <em>休 {p.restSec}s</em>
                        </span>
                      </summary>
                      <ul class="cues">
                        {p.cuesZh.map((c, ci) => (
                          <li key={ci}>{c}</li>
                        ))}
                      </ul>
                      <p class="alt">⬇ 退階：{p.regressionZh}</p>
                      <p class="alt">⬆ 進階：{p.progressionZh}</p>
                    </details>
                  ))}
                </div>
              ))}
            </details>
          ))}
        </div>
      </section>

      <div class="actionbar">
        <button class="btn primary big" data-testid="open-log" onClick={props.onOpenLog}>
          本週回報
        </button>
        <button class="btn" onClick={props.onReassess}>
          重新體測
        </button>
      </div>
    </div>
  );
}
