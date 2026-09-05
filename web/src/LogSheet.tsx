import { useState } from 'preact/hooks';
import type { Focus, PainArea, WeeklyLog } from './types';

const FOCUS_OPTS: { value: Focus; label: string }[] = [
  { value: 'tooEasy', label: '太輕鬆 😮‍💨' },
  { value: 'ok', label: '剛好 👍' },
  { value: 'tooHard', label: '太難 😵' },
];

const PAIN_OPTS: { value: PainArea; label: string }[] = [
  { value: 'wrist', label: '手腕' },
  { value: 'shoulder', label: '肩膀' },
  { value: 'lowerBack', label: '下背' },
];

export function LogSheet(props: {
  weekIndex: number;
  defaultPlanned: number;
  onSubmit: (log: WeeklyLog) => void;
  onClose: () => void;
}) {
  const [planned, setPlanned] = useState(props.defaultPlanned);
  const [completed, setCompleted] = useState(props.defaultPlanned);
  const [focus, setFocus] = useState<Focus>('ok');
  const [pain, setPain] = useState<PainArea[]>([]);
  const [notes, setNotes] = useState('');

  function togglePain(p: PainArea) {
    setPain((prev) => (prev.includes(p) ? prev.filter((x) => x !== p) : [...prev, p]));
  }

  function submit() {
    props.onSubmit({
      weekIndex: props.weekIndex,
      sessionsCompleted: Math.min(completed, planned),
      sessionsPlanned: planned,
      focus,
      pain,
      notes: notes || null,
    });
  }

  return (
    <div class="sheet-overlay" role="dialog" aria-modal="true">
      <div class="sheet">
        <h2>第 {props.weekIndex} 週訓練回報</h2>
        <p class="muted small">
          回報會驅動神經網絡微調（單週評分變動上限 +0.12 / −0.15，安全優先）
        </p>

        <div class="steppers">
          <label class="field">
            <span class="field-label">計畫訓練次數</span>
            <div class="stepper">
              <button onClick={() => setPlanned(Math.max(1, planned - 1))}>−</button>
              <span>{planned}</span>
              <button onClick={() => setPlanned(Math.min(7, planned + 1))}>＋</button>
            </div>
          </label>
          <label class="field">
            <span class="field-label">實際完成次數</span>
            <div class="stepper">
              <button onClick={() => setCompleted(Math.max(0, completed - 1))}>−</button>
              <span>{completed}</span>
              <button onClick={() => setCompleted(Math.min(planned, completed + 1))}>＋</button>
            </div>
          </label>
        </div>

        <div class="field">
          <span class="field-label">本週整體強度感受</span>
          <div class="options">
            {FOCUS_OPTS.map((o) => (
              <button
                key={o.value}
                class={focus === o.value ? 'opt on' : 'opt'}
                onClick={() => setFocus(o.value)}
              >
                {o.label}
              </button>
            ))}
          </div>
        </div>

        <div class="field">
          <span class="field-label">疼痛部位（可多選）</span>
          <div class="options">
            {PAIN_OPTS.map((o) => (
              <button
                key={o.value}
                class={pain.includes(o.value) ? 'opt danger on' : 'opt danger'}
                onClick={() => togglePain(o.value)}
              >
                {o.label}
              </button>
            ))}
          </div>
        </div>

        <label class="field">
          <span class="field-label">備註（選填）</span>
          <textarea
            rows={2}
            value={notes}
            placeholder="例：倒立撐下不去、手腕撐不久…"
            onInput={(e) => setNotes((e.target as HTMLTextAreaElement).value)}
          />
        </label>

        <div class="wizard-nav">
          <button class="btn" onClick={props.onClose}>
            取消
          </button>
          <button class="btn primary big" data-testid="log-submit" onClick={submit}>
            送出回報
          </button>
        </div>
      </div>
    </div>
  );
}
