#!/usr/bin/env python3
"""crates/train 的 numpy 移植（僅供沙盒無 Rust 工具鏈時做可學習性驗證／產生權重）。

與 Rust 版對齊：SplitMix64、sample_user、expert_profile（8 維標籤）、12→24→12→8 MLP、
sigmoid 輸出 + 0.5·Σerr² 梯度。差異：以 batch-sum 梯度近似逐樣本 SGD（速度考量）。
正式權重仍以 CI 的 `cargo run -p gas2-train` 為準。
"""
import json
import sys
import time

import numpy as np

MASK = (1 << 64) - 1


class Rng:
    def __init__(self, seed):
        self.s = seed & MASK

    def next_u64(self):
        self.s = (self.s + 0x9E3779B97F4A7C15) & MASK
        z = self.s
        z = ((z ^ (z >> 30)) * 0xBF58476D1CE4E5B9) & MASK
        z = ((z ^ (z >> 27)) * 0x94D049BB133111EB) & MASK
        return z ^ (z >> 31)

    def next_f32(self):
        return np.float32(self.next_u64() >> 40) / np.float32(1 << 24)

    def range(self, a, b):
        a = np.float32(a)
        b = np.float32(b)
        return a + (b - a) * self.next_f32()


def rround(x):  # Rust f32::round：四捨五入、.5 遠離零（此處皆非負）
    return np.floor(np.float32(x) + np.float32(0.5))


def sat(x):
    x = np.float32(x)
    return x / (np.float32(1) + x)


def jitter(rng, u, lo, hi, spread):
    lo = np.float32(lo)
    hi = np.float32(hi)
    spread = np.float32(spread)
    base = lo + (hi - lo) * u
    return np.clip(rng.range(base - spread, base + spread), lo, hi)


def sample_user(rng):
    u = rng.next_f32()
    a = {}
    a["shoulder_mobility"] = int(rround(jitter(rng, u, 0, 5, 1.2)))
    a["wrist_mobility"] = int(rround(jitter(rng, u, 0, 5, 1.2)))
    a["plank_sec"] = int(rround(jitter(rng, u, 10, 150, 30)))
    a["hollow_sec"] = int(rround(jitter(rng, u, 5, 100, 25)))
    a["pushup_reps"] = int(rround(jitter(rng, u, 1, 45, 10)))
    a["pike_pushup_reps"] = int(rround(jitter(rng, u, 0, 18, 5)))
    a["wall_walk_reps"] = int(rround(jitter(rng, u, 0, 9, 3)))
    a["wall_hs_hold_sec"] = int(rround(jitter(rng, u, 0, 110, 30)))
    a["wall_hspu_reps"] = int(rround(jitter(rng, u, 0, 10, 3)))
    height = rng.range(155, 190)
    bw_ratio = rng.range(0.32, 0.62) + np.float32(0.06) * (np.float32(1) - u)
    a["bodyweight_kg"] = float(np.clip(bw_ratio * height, 40, 130))
    a["height_cm"] = float(height)
    a["days_per_week"] = int(rround(rng.range(2, 6)))
    a["experience"] = int(np.clip(rround(jitter(rng, u, 0, 3, 1.0)), 0, 3))
    return a


def sanitize(a):
    a = dict(a)
    a["shoulder_mobility"] = min(a["shoulder_mobility"], 5)
    a["wrist_mobility"] = min(a["wrist_mobility"], 5)
    a["plank_sec"] = min(a["plank_sec"], 180)
    a["hollow_sec"] = min(a["hollow_sec"], 120)
    a["pushup_reps"] = min(a["pushup_reps"], 50)
    a["pike_pushup_reps"] = min(a["pike_pushup_reps"], 20)
    a["wall_walk_reps"] = min(a["wall_walk_reps"], 10)
    a["wall_hs_hold_sec"] = min(a["wall_hs_hold_sec"], 120)
    a["wall_hspu_reps"] = min(a["wall_hspu_reps"], 12)
    a["bodyweight_kg"] = float(np.clip(a["bodyweight_kg"], 25, 250))
    a["height_cm"] = float(np.clip(a["height_cm"], 100, 230))
    a["days_per_week"] = int(np.clip(a["days_per_week"], 1, 7))
    a["experience"] = min(a["experience"], 3)
    return a


def features(a):
    a = sanitize(a)
    bw = (a["bodyweight_kg"] / a["height_cm"] - 0.30) / 0.35
    return np.array(
        [
            a["shoulder_mobility"] / 5.0,
            a["wrist_mobility"] / 5.0,
            a["plank_sec"] / 180.0,
            a["hollow_sec"] / 120.0,
            a["pushup_reps"] / 50.0,
            a["pike_pushup_reps"] / 20.0,
            a["wall_walk_reps"] / 10.0,
            a["wall_hs_hold_sec"] / 120.0,
            a["wall_hspu_reps"] / 12.0,
            float(np.clip(bw, 0, 1)),
            a["days_per_week"] / 7.0,
            a["experience"] / 3.0,
        ],
        dtype=np.float32,
    )


def expert_profile(a, rng):
    a = sanitize(a)
    bw_over = float(np.clip((a["bodyweight_kg"] / a["height_cm"] - 0.45) / 0.20, 0, 1))
    pen = 1.0 - 0.25 * bw_over
    g_push = sat(a["pushup_reps"] / 25.0)
    g_plank = sat(a["plank_sec"] / 90.0)
    g_hollow = sat(a["hollow_sec"] / 70.0)
    g_wsh = sat(a["wall_hs_hold_sec"] / 60.0)
    g_ww = sat(a["wall_walk_reps"] / 8.0)
    g_mob = sat(a["shoulder_mobility"] / 4.0)
    g_hspu = sat(a["wall_hspu_reps"] / 8.0)
    g_pike = sat(a["pike_pushup_reps"] / 12.0)
    exp = sat(a["experience"] / 2.0) * 0.08

    def noise():
        return float(rng.range(-0.03, 0.03))

    exp_n = a["experience"] / 3.0
    mob_avg = (a["shoulder_mobility"] + a["wrist_mobility"]) / 10.0
    work = 1.4 * (0.30 * g_plank + 0.25 * g_hollow + 0.25 * g_push + 0.20 * exp_n) * (1.0 - 0.10 * bw_over)
    day_pen = 0.10 if a["days_per_week"] >= 6 else 0.0
    recovery = 0.20 + 0.35 * exp_n + 0.30 * mob_avg + 0.15 * (1.0 - bw_over) - day_pen
    progression = 0.30 + 0.25 * exp_n + 0.20 * mob_avg + 0.25 * g_wsh - 0.20 * bw_over

    out = [
        (0.6 * g_push + 0.4 * g_plank) * (1.0 - 0.10 * bw_over) + exp + noise(),
        0.6 * g_hollow + 0.4 * g_plank + exp + noise(),
        (0.45 * g_wsh + 0.30 * g_ww + 0.25 * g_mob) * pen + exp + noise(),
        (0.6 * g_hspu + 0.4 * g_pike) * pen + exp + noise(),
        (0.45 * g_pike + 0.30 * g_wsh + 0.25 * g_push) * pen + exp + noise(),
        work + noise(),
        recovery + noise(),
        progression + noise(),
    ]
    return np.clip(np.array(out, dtype=np.float32), 0, 1)


def gen_dataset(n, seed):
    rng = Rng(seed)
    xs = np.zeros((n, 12), dtype=np.float32)
    ts = np.zeros((n, 8), dtype=np.float32)
    for i in range(n):
        a = sample_user(rng)
        xs[i] = features(a)
        ts[i] = expert_profile(a, rng)
    return xs, ts


def init_weights(rng, arch):
    def fill(rows, cols):
        k = 1.0 / np.sqrt(cols)
        w = np.zeros((rows, cols), dtype=np.float32)
        for r in range(rows):
            for c in range(cols):
                w[r, c] = rng.range(-k, k)
        return w

    i, h1, h2, o = arch
    return {
        "w1": fill(h1, i), "b1": np.zeros(h1, np.float32),
        "w2": fill(h2, h1), "b2": np.zeros(h2, np.float32),
        "w3": fill(o, h2), "b3": np.zeros(o, np.float32),
    }


def forward(p, X):
    h1 = np.maximum(X @ p["w1"].T + p["b1"], 0)
    h2 = np.maximum(h1 @ p["w2"].T + p["b2"], 0)
    y = 1.0 / (1.0 + np.exp(-(h2 @ p["w3"].T + p["b3"])))
    return h1, h2, y


def mse(p, X, T):
    return float(np.mean((forward(p, X)[2] - T) ** 2))


def train_step_batch(p, X, T, lr):
    """batch-sum 梯度（等價於小 lr 下逐樣本 SGD 的一階近似）"""
    h1, h2, y = forward(p, X)
    d_out = (y - T) * y * (1 - y)
    d_h2 = (d_out @ p["w3"]) * (h2 > 0)
    d_h1 = (d_h2 @ p["w2"]) * (h1 > 0)
    p["w3"] -= lr * d_out.T @ h2
    p["b3"] -= lr * d_out.sum(0)
    p["w2"] -= lr * d_h2.T @ h1
    p["b2"] -= lr * d_h2.sum(0)
    p["w1"] -= lr * d_h1.T @ X
    p["b1"] -= lr * d_h1.sum(0)


def main():
    out_path = sys.argv[1] if len(sys.argv) > 1 else "/tmp/baseline_numpy.json"
    t0 = time.time()
    tx, tt = gen_dataset(6000, 42)
    vx, vt = gen_dataset(2000, 20_260_905)
    print(f"dataset ready in {time.time()-t0:.1f}s; label mean={tt.mean(0).round(3)} std={tt.std(0).round(3)}")
    mean_mse = float(np.mean((tt - tt.mean(0)) ** 2))
    print(f"mean-baseline MSE={mean_mse:.5f}")

    rng = Rng(42)
    arch = [12, 24, 12, 8]
    p = init_weights(rng, arch)
    order = np.arange(len(tx))
    np_rng = np.random.default_rng(42)
    batch = 16
    epochs = 900
    for ep in range(epochs):
        lr = 0.015 if ep < 500 else (0.006 if ep < 800 else 0.002)
        np_rng.shuffle(order)
        for k in range(0, len(order), batch):
            idx = order[k:k + batch]
            train_step_batch(p, tx[idx], tt[idx], lr)
        if (ep + 1) % 50 == 0:
            print(f"epoch {ep+1:>3}: train={mse(p, tx, tt):.5f} val={mse(p, vx, vt):.5f}  ({time.time()-t0:.0f}s)")
    val = mse(p, vx, vt)
    print(f"final val MSE={val:.5f}")
    y = forward(p, vx)[2]
    keys = ["basePush", "coreControl", "balanceSkill", "overheadPress", "compressionPower",
            "workCapacity", "recovery", "progressionRate"]
    for d, k in enumerate(keys):
        se = float(np.mean((y[:, d] - vt[:, d]) ** 2))
        var = float(np.var(vt[:, d]))
        print(f"  {k:<17} mse={se:.5f} var={var:.5f} R2={1-se/var:.3f}")
    weights = {
        "arch": arch,
        "w1": p["w1"].astype(float).round(8).tolist(), "b1": p["b1"].astype(float).round(8).tolist(),
        "w2": p["w2"].astype(float).round(8).tolist(), "b2": p["b2"].astype(float).round(8).tolist(),
        "w3": p["w3"].astype(float).round(8).tolist(), "b3": p["b3"].astype(float).round(8).tolist(),
        "trained": bool(val <= 0.01),
    }
    with open(out_path, "w") as f:
        json.dump(weights, f, separators=(",", ":"))
    print(f"wrote {out_path} trained={weights['trained']}")
    sys.exit(0 if val <= 0.01 else 1)


if __name__ == "__main__":
    main()
