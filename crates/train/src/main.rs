//! 離線訓練器：規則引擎產生合成資料 → 手寫 SGD 訓練 MLP → 權重寫入 core。
//!
//! 用法：
//! - `cargo run -p gas2-train`                          訓練並寫入 crates/core/weights/baseline.json
//! - `cargo run -p gas2-train -- --out <path>`          指定輸出
//! - `cargo run -p gas2-train -- --check`               驗證已提交權重的驗證集 MSE（CI 回歸防護）
//!
//! 「不用 AI」說明：標籤由專家規則系統（可審計的飽和曲線）生成，
//! 訓練是手寫梯度下降，無任何 ML 框架。

use gas2_core::model::{Assessment, INPUT_FEATURES, OUTPUT_SCORES};
use gas2_core::nn::Mlp;

/// 固定種子線性同餘亂數（可重現，無外部依賴）
struct Lcg(u64);

impl Lcg {
    fn next_f32(&mut self) -> f32 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((self.0 >> 33) as f32) / (u32::MAX as f32)
    }
    fn range(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.next_f32()
    }
    fn shuffle<T>(&mut self, xs: &mut [T]) {
        for i in (1..xs.len()).rev() {
            let j = (self.next_f32() * (i + 1) as f32) as usize % (i + 1);
            xs.swap(i, j);
        }
    }
}

fn sat(x: f32) -> f32 {
    x / (1.0 + x)
}

/// 專家規則系統：由體測項目推導「真實」能力評分（含微噪聲）。
/// 這是訓練資料的標籤來源，規則完全可審計。
fn expert_scores(a: &Assessment, rng: &mut Lcg) -> [f32; OUTPUT_SCORES] {
    let a = a.sanitized();
    let bw_over = ((a.bodyweight_ratio() - 0.45) / 0.20).clamp(0.0, 1.0);
    let pen = 1.0 - 0.25 * bw_over; // 體重相對身高偏高 → 倒立類吃虧
    let g_push = sat(a.pushup_reps as f32 / 25.0);
    let g_plank = sat(a.plank_sec as f32 / 90.0);
    let g_hollow = sat(a.hollow_sec as f32 / 70.0);
    let g_wsh = sat(a.wall_hs_hold_sec as f32 / 60.0);
    let g_ww = sat(a.wall_walk_reps as f32 / 8.0);
    let g_mob = sat(a.shoulder_mobility as f32 / 4.0);
    let g_hspu = sat(a.wall_hspu_reps as f32 / 8.0);
    let g_pike = sat(a.pike_pushup_reps as f32 / 12.0);
    let exp = sat(a.experience as f32 / 2.0) * 0.08;
    let noise = |rng: &mut Lcg| rng.range(-0.03, 0.03);
    [
        (0.6 * g_push + 0.4 * g_plank)
            .mul_add(1.0 - 0.10 * bw_over, exp + noise(rng))
            .clamp(0.0, 1.0),
        (0.6 * g_hollow + 0.4 * g_plank + exp + noise(rng)).clamp(0.0, 1.0),
        ((0.45 * g_wsh + 0.30 * g_ww + 0.25 * g_mob) * pen + exp + noise(rng)).clamp(0.0, 1.0),
        ((0.6 * g_hspu + 0.4 * g_pike) * pen + exp + noise(rng)).clamp(0.0, 1.0),
        ((0.45 * g_pike + 0.30 * g_wsh + 0.25 * g_push) * pen + exp + noise(rng)).clamp(0.0, 1.0),
    ]
}

/// 依「潛在運動能力 u」採樣合成使用者（欄位間有相關性）
fn jitter(rng: &mut Lcg, u: f32, lo: f32, hi: f32, spread: f32) -> f32 {
    let base = lo + (hi - lo) * u;
    rng.range(base - spread, base + spread).clamp(lo, hi)
}

fn sample_user(rng: &mut Lcg) -> Assessment {
    let u = rng.next_f32(); // 0=新手 … 1=高手
    let shoulder_mobility = jitter(rng, u, 0.0, 5.0, 1.2).round() as u8;
    let wrist_mobility = jitter(rng, u, 0.0, 5.0, 1.2).round() as u8;
    let plank_sec = jitter(rng, u, 10.0, 150.0, 30.0).round() as u16;
    let hollow_sec = jitter(rng, u, 5.0, 100.0, 25.0).round() as u16;
    let pushup_reps = jitter(rng, u, 1.0, 45.0, 10.0).round() as u8;
    let pike_pushup_reps = jitter(rng, u, 0.0, 18.0, 5.0).round() as u8;
    let wall_walk_reps = jitter(rng, u, 0.0, 9.0, 3.0).round() as u8;
    let wall_hs_hold_sec = jitter(rng, u, 0.0, 110.0, 30.0).round() as u16;
    let wall_hspu_reps = jitter(rng, u, 0.0, 10.0, 3.0).round() as u8;
    let height_cm = rng.range(155.0, 190.0);
    let bw_ratio = rng.range(0.32, 0.62) + 0.06 * (1.0 - u);
    let bodyweight_kg = (bw_ratio * height_cm).clamp(40.0, 130.0);
    let days_per_week = rng.range(2.0, 6.0).round() as u8;
    let experience = jitter(rng, u, 0.0, 3.0, 1.0).round().clamp(0.0, 3.0) as u8;
    Assessment {
        shoulder_mobility,
        wrist_mobility,
        plank_sec,
        hollow_sec,
        pushup_reps,
        pike_pushup_reps,
        wall_walk_reps,
        wall_hs_hold_sec,
        wall_hspu_reps,
        bodyweight_kg,
        height_cm,
        days_per_week,
        experience,
    }
}

fn gen_dataset(n: usize, seed: u64) -> (Vec<[f32; INPUT_FEATURES]>, Vec<[f32; OUTPUT_SCORES]>) {
    let mut rng = Lcg(seed);
    let mut xs = Vec::with_capacity(n);
    let mut ts = Vec::with_capacity(n);
    for _ in 0..n {
        let a = sample_user(&mut rng);
        xs.push(a.features());
        ts.push(expert_scores(&a, &mut rng));
    }
    (xs, ts)
}

fn fill_gaussian(rng: &mut Lcg, rows: &mut Vec<Vec<f32>>, fan_in: usize) {
    let k = 1.0 / (fan_in as f32).sqrt();
    for row in rows.iter_mut() {
        for v in row.iter_mut() {
            *v = rng.range(-k, k);
        }
    }
}

fn init_weights(nn: &mut Mlp, rng: &mut Lcg) {
    fill_gaussian(rng, &mut nn.w1, nn.arch[0]);
    fill_gaussian(rng, &mut nn.w2, nn.arch[1]);
    fill_gaussian(rng, &mut nn.w3, nn.arch[2]);
    for v in nn
        .b1
        .iter_mut()
        .chain(nn.b2.iter_mut())
        .chain(nn.b3.iter_mut())
    {
        *v = 0.0;
    }
}

const VAL_SEED: u64 = 20_260_905;
const TRAIN_SEED: u64 = 42;
const MSE_THRESHOLD: f32 = 0.01;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let check = args.iter().any(|a| a == "--check");
    let out_path = args
        .iter()
        .position(|a| a == "--out")
        .and_then(|i| args.get(i + 1))
        .cloned()
        .unwrap_or_else(|| "crates/core/weights/baseline.json".to_string());

    if check {
        let (vx, vt) = gen_dataset(2000, VAL_SEED);
        let bytes =
            std::fs::read(&out_path).unwrap_or_else(|e| panic!("讀取 {out_path} 失敗: {e}"));
        let nn = Mlp::from_json(std::str::from_utf8(&bytes).expect("weights utf8"))
            .expect("權重結構不合法");
        if !nn.trained {
            println!("[check] 權重尚未訓練（trained=false），跳過 MSE 檢查");
            return;
        }
        let mse = nn.mse_on(&vx, &vt);
        println!("[check] 驗證集 MSE = {mse:.5}（門檻 {MSE_THRESHOLD}）");
        if mse > MSE_THRESHOLD {
            eprintln!("[check] 失敗：權重退化，請重新執行訓練");
            std::process::exit(1);
        }
        println!("[check] OK");
        return;
    }

    // ── 訓練流程 ──────────────────────────────────────────────
    let (tx, tt) = gen_dataset(6000, TRAIN_SEED);
    let (vx, vt) = gen_dataset(2000, VAL_SEED);
    println!(
        "資料集：train={} val={}（規則引擎標籤＋噪聲）",
        tx.len(),
        vx.len()
    );

    let mut rng = Lcg(TRAIN_SEED);
    let mut nn = Mlp::new(INPUT_FEATURES, 16, 8, OUTPUT_SCORES);
    init_weights(&mut nn, &mut rng);

    let epochs = 400usize;
    let batch = 64usize;
    let mut idx: Vec<usize> = (0..tx.len()).collect();
    let mut i = 0usize;
    for epoch in 0..epochs {
        let lr = if epoch < 200 { 0.08 } else { 0.02 };
        rng.shuffle(&mut idx);
        let mut k = 0;
        while k < idx.len() {
            let end = (k + batch).min(idx.len());
            for &j in &idx[k..end] {
                nn.train_step(&tx[j], &tt[j], lr);
            }
            k = end;
            i += 1;
        }
        if (epoch + 1) % 50 == 0 {
            println!(
                "epoch {:>3}: train_mse={:.5} val_mse={:.5}",
                epoch + 1,
                nn.mse_on(&tx, &tt),
                nn.mse_on(&vx, &vt)
            );
        }
    }
    let _ = i;

    let val = nn.mse_on(&vx, &vt);
    println!("最終驗證集 MSE = {val:.5}");
    if val > MSE_THRESHOLD {
        eprintln!("警告：MSE 高於門檻 {MSE_THRESHOLD}，請檢查訓練流程");
        std::process::exit(1);
    }

    nn.trained = true;
    std::fs::write(&out_path, nn.to_json()).unwrap_or_else(|e| panic!("寫入 {out_path} 失敗: {e}"));
    println!("權重已寫入 {out_path}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn training_reduces_validation_error_dramatically() {
        let (tx, tt) = gen_dataset(800, 7);
        let (vx, vt) = gen_dataset(300, 8);
        let mut rng = Lcg(9);
        let mut nn = Mlp::new(INPUT_FEATURES, 16, 8, OUTPUT_SCORES);
        init_weights(&mut nn, &mut rng);
        let before = nn.mse_on(&vx, &vt);
        for _ in 0..40 {
            for j in 0..tx.len() {
                nn.train_step(&tx[j], &tt[j], 0.05);
            }
        }
        let after = nn.mse_on(&vx, &vt);
        assert!(
            after < before * 0.5,
            "40 epochs 應大幅降低誤差: {before} -> {after}"
        );
        assert!(after < 0.02, "應接近標籤函數: {after}");
    }

    #[test]
    fn expert_scores_in_range() {
        let mut rng = Lcg(1);
        for _ in 0..200 {
            let a = sample_user(&mut rng);
            for v in expert_scores(&a, &mut rng) {
                assert!((0.0..=1.0).contains(&v));
            }
        }
    }
}
