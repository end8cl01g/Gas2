import { useState } from 'preact/hooks';
import type { Assessment } from './types';

interface FieldDef {
  key: keyof Assessment;
  label: string;
  hint?: string;
  unit?: string;
  min: number;
  max: number;
  def: number;
}

const STEPS: { title: string; fields: FieldDef[] }[] = [
  {
    title: '關於你',
    fields: [
      { key: 'heightCm', label: '身高', unit: 'cm', min: 100, max: 230, def: 172 },
      { key: 'bodyweightKg', label: '體重', unit: 'kg', min: 30, max: 200, def: 70 },
      { key: 'daysPerWeek', label: '每週可訓練天數', unit: '天/週', min: 1, max: 7, def: 3 },
      {
        key: 'experience',
        label: '訓練年資',
        min: 0,
        max: 3,
        def: 0,
        hint: '0=全新　1=一年內　2=1–3 年　3=三年以上',
      },
    ],
  },
  {
    title: '活動度',
    fields: [
      {
        key: 'shoulderMobility',
        label: '肩屈活動度',
        min: 0,
        max: 5,
        def: 2,
        hint: '背貼牆、雙臂沿牆上舉：能完全貼牆過頭=5，僅能舉到水平=0',
      },
      {
        key: 'wristMobility',
        label: '手腕活動度',
        min: 0,
        max: 5,
        def: 2,
        hint: '四足跪姿掌壓繞環：能深壓且無痛=5，略壓即痛=0',
      },
    ],
  },
  {
    title: '支撐與推力',
    fields: [
      { key: 'plankSec', label: '平板支撐最長時間', unit: '秒', min: 0, max: 180, def: 45 },
      {
        key: 'hollowSec',
        label: '空心支撐最長時間',
        unit: '秒',
        min: 0,
        max: 120,
        def: 20,
        hint: '仰躺下背貼地、肩與腳離地的船式支撐',
      },
      { key: 'pushupReps', label: '伏地挺身最大次數', unit: '下', min: 0, max: 50, def: 10 },
    ],
  },
  {
    title: '倒立專項',
    fields: [
      {
        key: 'pikePushupReps',
        label: '折刀伏地挺身次數',
        unit: '下',
        min: 0,
        max: 20,
        def: 3,
        hint: '髖推高的倒 V 姿勢，頭頂朝地下放',
      },
      { key: 'wallWalkReps', label: '壁走次數', unit: '次', min: 0, max: 10, def: 1 },
      { key: 'wallHsHoldSec', label: '靠牆倒立支撐', unit: '秒', min: 0, max: 120, def: 10 },
      { key: 'wallHspuReps', label: '靠牆倒立俯臥撐', unit: '下', min: 0, max: 12, def: 0 },
    ],
  },
];

function defaultValues(initial: Assessment | null): Record<string, number> {
  const vals: Record<string, number> = {};
  for (const step of STEPS) {
    for (const f of step.fields) {
      vals[f.key as string] = initial ? (initial[f.key] as number) : f.def;
    }
  }
  return vals;
}

export function Wizard(props: { initial: Assessment | null; onSubmit: (a: Assessment) => void }) {
  const [values, setValues] = useState<Record<string, number>>(() => defaultValues(props.initial));
  const [step, setStep] = useState(0);

  const current = STEPS[step];
  const isLast = step === STEPS.length - 1;

  function set(key: string, v: number) {
    setValues((prev) => ({ ...prev, [key]: v }));
  }

  function submit() {
    const a: Record<string, number> = {};
    for (const s of STEPS) {
      for (const f of s.fields) {
        a[f.key as string] = Math.min(f.max, Math.max(f.min, values[f.key as string] ?? f.def));
      }
    }
    props.onSubmit(a as unknown as Assessment);
  }

  return (
    <div class="container">
      <header class="wizard-head">
        <div class="progress">
          {STEPS.map((_, i) => (
            <span key={i} class={i <= step ? 'dot on' : 'dot'} />
          ))}
        </div>
        <h2>
          體能測試 {step + 1}/{STEPS.length}・{current.title}
        </h2>
      </header>

      <div class="card form">
        {current.fields.map((f) => (
          <label class="field" key={f.key as string}>
            <span class="field-label">
              {f.label}
              {f.unit ? <em class="unit">{f.unit}</em> : null}
            </span>
            <input
              type="number"
              inputMode="numeric"
              min={f.min}
              max={f.max}
              step={1}
              value={values[f.key as string]}
              onInput={(e) => set(f.key as string, Number((e.target as HTMLInputElement).value) || 0)}
              data-testid={`field-${f.key as string}`}
            />
            {f.hint ? <span class="hint">{f.hint}</span> : null}
          </label>
        ))}
      </div>

      <div class="wizard-nav">
        {step > 0 ? (
          <button class="btn" data-testid="wizard-back" onClick={() => setStep(step - 1)}>
            上一步
          </button>
        ) : null}
        {isLast ? (
          <button class="btn primary big" data-testid="wizard-submit" onClick={submit}>
            生成我的課表
          </button>
        ) : (
          <button class="btn primary big" data-testid="wizard-next" onClick={() => setStep(step + 1)}>
            下一步
          </button>
        )}
      </div>
    </div>
  );
}
