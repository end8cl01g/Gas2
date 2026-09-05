//! 手寫多層感知器（MLP）：12 → 24 → 12 → 5，ReLU 隱層、Sigmoid 輸出。
//! 含前向推論、完整反向傳播（離線訓練用）、僅輸出層微調（線上用）。

use serde::{Deserialize, Serialize};

use crate::model::{INPUT_FEATURES, OUTPUT_SCORES};

/// 權重結構：三層全連接。`trained` 標記是否已完成離線訓練。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mlp {
    pub arch: [usize; 4],
    pub w1: Vec<Vec<f32>>,
    pub b1: Vec<f32>,
    pub w2: Vec<Vec<f32>>,
    pub b2: Vec<f32>,
    pub w3: Vec<Vec<f32>>,
    pub b3: Vec<f32>,
    #[serde(default)]
    pub trained: bool,
}

fn relu(v: Vec<f32>) -> Vec<f32> {
    v.into_iter()
        .map(|x| if x > 0.0 { x } else { 0.0 })
        .collect()
}

fn sigmoid(x: f32) -> f32 {
    1.0 / (1.0 + (-x).exp())
}

fn matvec(w: &[Vec<f32>], b: &[f32], x: &[f32]) -> Vec<f32> {
    w.iter()
        .zip(b.iter())
        .map(|(row, &bi)| {
            row.iter()
                .zip(x.iter())
                .map(|(&wi, &xi)| wi * xi)
                .sum::<f32>()
                + bi
        })
        .collect()
}

impl Mlp {
    /// 全零初始化（供訓練端自行填入隨機權重）
    pub fn new(input: usize, h1: usize, h2: usize, output: usize) -> Self {
        Self {
            arch: [input, h1, h2, output],
            w1: vec![vec![0.0; input]; h1],
            b1: vec![0.0; h1],
            w2: vec![vec![0.0; h1]; h2],
            b2: vec![0.0; h2],
            w3: vec![vec![0.0; h2]; output],
            b3: vec![0.0; output],
            trained: false,
        }
    }

    pub fn from_json(json: &str) -> Result<Self, String> {
        let m: Mlp = serde_json::from_str(json).map_err(|e| format!("權重 JSON 解析失敗: {e}"))?;
        m.validate()?;
        Ok(m)
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("Mlp serializes")
    }

    /// 結構一致性檢查
    pub fn validate(&self) -> Result<(), String> {
        let [input, h1, h2, output] = self.arch;
        let check = |cond: bool, msg: &str| if cond { Ok(()) } else { Err(msg.to_string()) };
        check(!self.w1.is_empty(), "w1 是空的")?;
        check(self.w1.len() == h1, "w1 列數不符 h1")?;
        check(
            self.w1.iter().all(|r| r.len() == input),
            "w1 行數不符 input",
        )?;
        check(self.b1.len() == h1, "b1 長度不符")?;
        check(self.w2.len() == h2, "w2 列數不符 h2")?;
        check(self.w2.iter().all(|r| r.len() == h1), "w2 行數不符 h1")?;
        check(self.b2.len() == h2, "b2 長度不符")?;
        check(self.w3.len() == output, "w3 列數不符 output")?;
        check(self.w3.iter().all(|r| r.len() == h2), "w3 行數不符 h2")?;
        check(self.b3.len() == output, "b3 長度不符")?;
        check(
            self.w1.iter().flatten().all(|v| v.is_finite()),
            "w1 有非有限值",
        )?;
        check(
            self.w2.iter().flatten().all(|v| v.is_finite()),
            "w2 有非有限值",
        )?;
        check(
            self.w3.iter().flatten().all(|v| v.is_finite()),
            "w3 有非有限值",
        )?;
        Ok(())
    }

    /// 前向推論：回傳 OUTPUT_SCORES 個 0–1 能力評分
    pub fn forward(&self, x: &[f32]) -> Vec<f32> {
        let h1 = relu(matvec(&self.w1, &self.b1, x));
        let h2 = relu(matvec(&self.w2, &self.b2, &h1));
        matvec(&self.w3, &self.b3, &h2)
            .into_iter()
            .map(sigmoid)
            .collect()
    }

    /// 以標準化評分輸入推論
    pub fn infer(&self, x: &[f32; INPUT_FEATURES]) -> [f32; OUTPUT_SCORES] {
        let mut out = [0.0f32; OUTPUT_SCORES];
        let v = self.forward(x);
        out.copy_from_slice(&v);
        out
    }

    /// 單一樣本 SGD（全網路），回傳該樣本 MSE。
    /// 數值核心採索引迴圈寫法（多陣列交錯存取），豁免 needless_range_loop。
    #[allow(clippy::needless_range_loop)]
    pub fn train_step(&mut self, x: &[f32], target: &[f32], lr: f32) -> f32 {
        let h1 = relu(matvec(&self.w1, &self.b1, x));
        let h2 = relu(matvec(&self.w2, &self.b2, &h1));
        let y: Vec<f32> = matvec(&self.w3, &self.b3, &h2)
            .into_iter()
            .map(sigmoid)
            .collect();

        let n = y.len() as f32;
        let mut mse = 0.0;
        let mut d_out = vec![0.0; y.len()];
        for i in 0..y.len() {
            let err = y[i] - target[i];
            mse += err * err;
            d_out[i] = err * y[i] * (1.0 - y[i]); // sigmoid + MSE
        }
        mse /= n;

        // 輸出層梯度，同時累積 d_h2
        let mut d_h2 = vec![0.0; h2.len()];
        for i in 0..y.len() {
            for (j, dh) in d_h2.iter_mut().enumerate() {
                *dh += self.w3[i][j] * d_out[i];
            }
            for (j, w) in self.w3[i].iter_mut().enumerate() {
                *w -= lr * d_out[i] * h2[j];
            }
            self.b3[i] -= lr * d_out[i];
        }
        for (j, dh) in d_h2.iter_mut().enumerate() {
            if h2[j] <= 0.0 {
                *dh = 0.0;
            }
        }

        let mut d_h1 = vec![0.0; h1.len()];
        for j in 0..h2.len() {
            for (k, dh) in d_h1.iter_mut().enumerate() {
                *dh += self.w2[j][k] * d_h2[j];
            }
            for (k, w) in self.w2[j].iter_mut().enumerate() {
                *w -= lr * d_h2[j] * h1[k];
            }
            self.b2[j] -= lr * d_h2[j];
        }
        for (k, dh) in d_h1.iter_mut().enumerate() {
            if h1[k] <= 0.0 {
                *dh = 0.0;
            }
        }

        for k in 0..h1.len() {
            for (xi, w) in self.w1[k].iter_mut().enumerate() {
                *w -= lr * d_h1[k] * x[xi];
            }
            self.b1[k] -= lr * d_h1[k];
        }
        mse
    }

    /// 線上微調：只更新輸出層（最穩定、參數最少）。
    /// 隱層凍結，因此單次回報不會破壞整體特徵映射。
    pub fn train_step_output_layer(&mut self, x: &[f32], target: &[f32], lr: f32) -> f32 {
        let h1 = relu(matvec(&self.w1, &self.b1, x));
        let h2 = relu(matvec(&self.w2, &self.b2, &h1));
        let y: Vec<f32> = matvec(&self.w3, &self.b3, &h2)
            .into_iter()
            .map(sigmoid)
            .collect();

        let mut mse = 0.0;
        for i in 0..y.len() {
            let err = y[i] - target[i];
            mse += err * err;
            let d = err * y[i] * (1.0 - y[i]);
            for (j, w) in self.w3[i].iter_mut().enumerate() {
                *w -= lr * d * h2[j];
            }
            self.b3[i] -= lr * d;
        }
        mse / y.len() as f32
    }

    /// 資料集 MSE（離線訓練/驗證用）
    pub fn mse_on(&self, xs: &[[f32; INPUT_FEATURES]], ts: &[[f32; OUTPUT_SCORES]]) -> f32 {
        if xs.is_empty() {
            return 0.0;
        }
        let mut total = 0.0;
        for (x, t) in xs.iter().zip(ts.iter()) {
            let y = self.infer(x);
            for i in 0..OUTPUT_SCORES {
                let e = y[i] - t[i];
                total += e * e;
            }
        }
        total / (xs.len() as f32 * OUTPUT_SCORES as f32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forward_output_in_unit_range() {
        let nn = Mlp::new(INPUT_FEATURES, 16, 8, OUTPUT_SCORES);
        let x = [0.5f32; INPUT_FEATURES];
        for v in nn.infer(&x) {
            assert!((0.0..=1.0).contains(&v));
        }
    }

    #[test]
    fn full_training_reduces_error_on_single_sample() {
        let mut nn = Mlp::new(INPUT_FEATURES, 16, 8, OUTPUT_SCORES);
        // 以常數函數為玩具目標
        let x = [0.5f32; INPUT_FEATURES];
        let t = [0.9f32; OUTPUT_SCORES];
        let before = nn.mse_on(&[x], &[t]);
        for _ in 0..500 {
            nn.train_step(&x, &t, 0.1);
        }
        let after = nn.mse_on(&[x], &[t]);
        assert!(after < before, "訓練應降低誤差: {before} -> {after}");
        assert!(after < 0.01, "應收斂到小誤差: {after}");
    }

    #[test]
    fn output_layer_training_moves_output_toward_target() {
        let mut nn = Mlp::new(INPUT_FEATURES, 16, 8, OUTPUT_SCORES);
        // 給隱層一點非零結構
        for (i, row) in nn.w1.iter_mut().enumerate() {
            for v in row.iter_mut() {
                *v = ((i % 3) as f32 - 1.0) * 0.1;
            }
        }
        let x = [0.4f32; INPUT_FEATURES];
        let y0 = nn.infer(&x);
        let t = [0.95f32; OUTPUT_SCORES];
        for _ in 0..300 {
            nn.train_step_output_layer(&x, &t, 0.08);
        }
        let y1 = nn.infer(&x);
        for i in 0..OUTPUT_SCORES {
            assert!(y1[i] > y0[i], "第 {i} 維應上移: {} -> {}", y0[i], y1[i]);
        }
    }

    #[test]
    fn json_roundtrip_and_validation() {
        let mut nn = Mlp::new(INPUT_FEATURES, 16, 8, OUTPUT_SCORES);
        nn.b3[0] = 0.3;
        let json = nn.to_json();
        let back = Mlp::from_json(&json).unwrap();
        assert_eq!(back.arch, nn.arch);
        assert!((back.b3[0] - 0.3).abs() < 1e-9);

        let bad = r#"{"arch":[12,16,8,5],"w1":[],"b1":[],"w2":[],"b2":[],"w3":[],"b3":[]}"#;
        assert!(Mlp::from_json(bad).is_err());
    }
}
