//! PTH（Press to Handstand）動作庫：文字要點＋退階/進階說明。
//! 決策 §6-7：v1 不做圖片，文字承載教學價值。

#[derive(Debug, Clone, Copy)]
pub struct Exercise {
    pub id: &'static str,
    pub name_zh: &'static str,
    pub cues_zh: &'static [&'static str],
    pub regression_zh: &'static str,
    pub progression_zh: &'static str,
    pub base_sets: u8,
    /// 例如 "8-10" 或 "30-45秒"
    pub base_reps: &'static str,
    pub rest_sec: u16,
}

pub const EXERCISES: &[Exercise] = &[
    // ── 熱身／活動度 ──────────────────────────────────────────────
    Exercise {
        id: "wrist_prep",
        name_zh: "手腕熱身（掌壓繞環）",
        cues_zh: &[
            "四足跪姿，手掌貼地，重心緩慢前後左右繞行",
            "指背朝向不同方向各繞 5–8 圈",
            "過程中保持手肘伸直、肩膀推離地面",
        ],
        regression_zh: "減少繞行圈數、降低重心偏移幅度",
        progression_zh: "伸直雙腿改成平板姿勢增加負荷",
        base_sets: 2,
        base_reps: "30秒",
        rest_sec: 30,
    },
    Exercise {
        id: "shoulder_circles",
        name_zh: "肩繞環／肩胛啟動",
        cues_zh: &[
            "站姿雙臂伸直畫大圓，順逆時針各 10 圈",
            "感受肩胛骨先動、手臂跟著動",
        ],
        regression_zh: "縮小繞圈半徑",
        progression_zh: "手持輕量水瓶增加阻力",
        base_sets: 2,
        base_reps: "10圈",
        rest_sec: 30,
    },
    Exercise {
        id: "wall_down_dog",
        name_zh: "牆壁下犬式",
        cues_zh: &[
            "背對牆壁，手扶牆面走低成下犬角度",
            "胸口向地板方向壓，延展腋窩與背闊",
            "保持骨盆後傾、肋骨內收",
        ],
        regression_zh: "手扶高度較高的位置",
        progression_zh: "手的位置越來越低，接近地面下犬",
        base_sets: 2,
        base_reps: "20-30秒",
        rest_sec: 30,
    },
    Exercise {
        id: "wrist_stretch",
        name_zh: "手腕伸展放鬆",
        cues_zh: &["跪姿手背貼地，輕壓 20 秒", "再以掌心貼地前後搖晃伸展"],
        regression_zh: "減少按壓深度",
        progression_zh: "增加按壓時間至 30–40 秒",
        base_sets: 2,
        base_reps: "20秒",
        rest_sec: 20,
    },
    Exercise {
        id: "shoulder_flex_stretch",
        name_zh: "肩屈活動度伸展",
        cues_zh: &[
            "背貼牆、骨盆後傾，雙臂沿牆向上滑",
            "手臂盡量貼牆、肋骨不外翻",
            "在最高點停留呼吸 3 次",
        ],
        regression_zh: "雙臂張開與肩同寬再上滑",
        progression_zh: "單臂進行並靠近牆角加深",
        base_sets: 2,
        base_reps: "8次",
        rest_sec: 30,
    },
    // ── 階段 0：基礎力量 ─────────────────────────────────────────
    Exercise {
        id: "incline_pushup",
        name_zh: "上斜伏地挺身",
        cues_zh: &[
            "手撐於穩定高處（桌緣/床緣），身體成直線",
            "下放至胸口接近支撐面再推回",
            "全程臀肌與腹肌收緊，不塌腰",
        ],
        regression_zh: "提高支撐面高度",
        progression_zh: "降低支撐面高度直至地面伏地挺身",
        base_sets: 3,
        base_reps: "8-12",
        rest_sec: 90,
    },
    Exercise {
        id: "pushup",
        name_zh: "伏地挺身",
        cues_zh: &[
            "雙手略寬於肩，手肘與身體約 45 度",
            "身體成一直線，核心與臀肌收緊",
            "胸口觸地前 2–3 公分即推起，肩胛不翼狀突出",
        ],
        regression_zh: "改為上斜伏地挺身或跪姿伏地挺身",
        progression_zh: "放慢離心 3 秒，或改鑽石伏地挺身",
        base_sets: 3,
        base_reps: "8-12",
        rest_sec: 90,
    },
    Exercise {
        id: "plank",
        name_zh: "平板支撐",
        cues_zh: &[
            "手肘在肩正下方，骨盆後傾",
            "臀肌與腹肌同時收緊，避免塌腰或撅臀",
            "自然呼吸不憋氣",
        ],
        regression_zh: "膝蓋著地的跪姿平板",
        progression_zh: "單腳離地或延長支撐時間",
        base_sets: 3,
        base_reps: "30-45秒",
        rest_sec: 60,
    },
    Exercise {
        id: "dead_bug",
        name_zh: "死蟲式",
        cues_zh: &[
            "仰躺，對側手腳同時伸直放下",
            "下背全程貼地，肋骨下沉",
            "動作慢，配合呼吸",
        ],
        regression_zh: "只移動腳、手不動",
        progression_zh: "改為空心體位準備（hollow 搖擺）",
        base_sets: 3,
        base_reps: "每側8次",
        rest_sec: 60,
    },
    Exercise {
        id: "hollow_hold",
        name_zh: "空心支撐（Hollow Hold）",
        cues_zh: &[
            "下背壓緊地面，肩與腳離地",
            "骨盆後傾是重點，腰椎不離地",
            "可用手姿勢調整難度（高舉較難）",
        ],
        regression_zh: "改為屈膝空心支撐（tuck hollow）",
        progression_zh: "延長時間或雙臂過頭、手腕／腳踝加重",
        base_sets: 3,
        base_reps: "20-30秒",
        rest_sec: 60,
    },
    Exercise {
        id: "plank_shoulder_taps",
        name_zh: "平板拍肩",
        cues_zh: &[
            "平板姿勢下交替以手拍對側肩膀",
            "骨盆保持水平，不左右搖晃",
            "支撐手把地面推穩",
        ],
        regression_zh: "改跪姿平板拍肩",
        progression_zh: "放慢節奏並在頂點停頓 1 秒",
        base_sets: 3,
        base_reps: "每側8-10次",
        rest_sec: 60,
    },
    Exercise {
        id: "pike_pushup",
        name_zh: "折刀伏地挺身（Pike Push-up）",
        cues_zh: &[
            "髖部推高成倒 V，頭頂朝地面下放",
            "手肘約 45 度內收，不外開",
            "推回時肩膀主動推離地面",
        ],
        regression_zh: "縮小下放深度或墊高雙手",
        progression_zh: "雙腳墊高（階梯/椅子）增加垂直負荷",
        base_sets: 3,
        base_reps: "6-10",
        rest_sec: 90,
    },
    // ── 階段 1：壓撐與支撐 ───────────────────────────────────────
    Exercise {
        id: "elevated_pike_pushup",
        name_zh: "抬高折刀伏地挺身",
        cues_zh: &[
            "雙腳墊高至桌面高度，身體接近垂直",
            "頭頂朝手之間落地，保持前傾重心",
            "核心收緊避免塌腰",
        ],
        regression_zh: "降低墊高高度",
        progression_zh: "朝靠牆倒立俯臥撐（半程）過渡",
        base_sets: 4,
        base_reps: "5-8",
        rest_sec: 90,
    },
    Exercise {
        id: "wall_plank",
        name_zh: "面牆折刀支撐",
        cues_zh: &[
            "腳踩牆面、髖高於肩的折刀支撐",
            "肩膀主動推離地面（protracted）",
            "眼睛看向雙手之間的地板",
        ],
        regression_zh: "腳踩位置放低",
        progression_zh: "腳沿牆上移，加大肩角",
        base_sets: 3,
        base_reps: "20-30秒",
        rest_sec: 60,
    },
    Exercise {
        id: "wall_walk",
        name_zh: "壁走（Wall Walk）",
        cues_zh: &[
            "背對牆，腳沿牆上走、手同步走近牆",
            "目標：胸口貼牆、手臂伸直倒立支撐",
            "下降一樣慢，不要滑落",
        ],
        regression_zh: "限制上走範圍（45–60 度）",
        progression_zh: "鼻子貼牆停留 5 秒再下降",
        base_sets: 3,
        base_reps: "3-5",
        rest_sec: 90,
    },
    Exercise {
        id: "seated_pike_compress",
        name_zh: "坐姿壓撐（Pike Compression）",
        cues_zh: &[
            "坐姿直腿前伸，手掌撐地於髖側",
            "主動把骨盆壓離地面（壓撐感）",
            "肩膀下沉、手肘微彎可控",
        ],
        regression_zh: "屈膝降低力矩",
        progression_zh: "改為支撐 L 座（L-sit 前身）抬離更久",
        base_sets: 4,
        base_reps: "10-15秒",
        rest_sec: 60,
    },
    // ── 階段 2：倒立技能 ─────────────────────────────────────────
    Exercise {
        id: "chest_to_wall",
        name_zh: "胸口貼牆倒立",
        cues_zh: &[
            "面牆壁走至上位成倒立，胸口輕貼牆",
            "手肘伸直、肩膀推高（聳肩頂位）",
            "骨盆後傾、腳跟併攏，用指尖抓平衡",
        ],
        regression_zh: "只走到一半角度即回",
        progression_zh: "僅以腳尖點牆，拉長離牆時間",
        base_sets: 4,
        base_reps: "20-40秒",
        rest_sec: 60,
    },
    Exercise {
        id: "wall_handstand_hold",
        name_zh: "靠牆倒立支撐",
        cues_zh: &[
            "背對牆上牆成倒立",
            "身體成一直線：肋骨內收、骨盆對位",
            "重心放在手掌根部，指尖微調",
        ],
        regression_zh: "上牆角度減小或改面牆版本",
        progression_zh: "單腳離牆找重心、縮短觸牆時間",
        base_sets: 4,
        base_reps: "20-45秒",
        rest_sec: 60,
    },
    Exercise {
        id: "kickup_practice",
        name_zh: "踢上倒立練習（自由倒立）",
        cues_zh: &[
            "從站立前折踢上，控制力道不過衝",
            "失衡時安全側跨一步落地",
            "每次只追求「停住 1–2 秒」的品質",
        ],
        regression_zh: "面對牆以指尖輔助修正重心",
        progression_zh: "連續多次 3–5 秒的穩定停駐",
        base_sets: 5,
        base_reps: "10次嘗試",
        rest_sec: 60,
    },
    // ── 階段 3：倒立力量 ─────────────────────────────────────────
    Exercise {
        id: "wall_hspu_partial",
        name_zh: "半程靠牆倒立俯臥撐",
        cues_zh: &[
            "靠牆倒立，僅下放 1/4–1/2 行程",
            "手肘 45 度，頭頂輕觸至目標深度即推回",
            "全程保持肩膀推高、不塌肩",
        ],
        regression_zh: "再縮短行程或以彈力帶輔助",
        progression_zh: "逐步加大行程至頭頂碰地",
        base_sets: 4,
        base_reps: "4-6",
        rest_sec: 120,
    },
    Exercise {
        id: "wall_hspu_neg",
        name_zh: "靠牆倒立俯臥撐離心",
        cues_zh: &[
            "上牆後緩慢 3–5 秒下放至頭頂觸地",
            "落地後以跪姿或走下牆重置",
            "離心全程保持身體張力",
        ],
        regression_zh: "縮短離心時間至 2 秒",
        progression_zh: "加做向心推起（完整倒立俯臥撐）",
        base_sets: 4,
        base_reps: "3-5",
        rest_sec: 120,
    },
    Exercise {
        id: "wall_hspu",
        name_zh: "靠牆倒立俯臥撐",
        cues_zh: &[
            "靠牆倒立下放至頭頂觸地再推回",
            "下放時重心微向前，推起時不後傾",
            "呼吸：下放吸氣、推起吐氣",
        ],
        regression_zh: "改半程或離心版本",
        progression_zh: "放慢節奏、增加次數或改折腿倒立俯臥撐",
        base_sets: 4,
        base_reps: "3-6",
        rest_sec: 120,
    },
    // ── 階段 4：PTH 專項 ─────────────────────────────────────────
    Exercise {
        id: "pth_neg_elevated",
        name_zh: "高位 Press-to-Handstand 離心",
        cues_zh: &[
            "雙手撐地於墊高平面（手撐低台）起頭",
            "從倒立緩慢 5 秒壓下至站立前折",
            "全程手臂伸直、肩角打開",
        ],
        regression_zh: "提高站立側高度縮短行程",
        progression_zh: "降低高度直至地面離心",
        base_sets: 4,
        base_reps: "3-5",
        rest_sec: 120,
    },
    Exercise {
        id: "tuck_pth",
        name_zh: "屈腿 Press-to-Handstand",
        cues_zh: &[
            "前折站立、手掌紮實壓地",
            "重心前移至肩上，屈腿壓上成倒立",
            "上到頂點後再伸髖併腿",
        ],
        regression_zh: "以跳躍輔助完成上半程",
        progression_zh: "逐步伸直雙腿（straddle → 併腿）",
        base_sets: 4,
        base_reps: "3-5",
        rest_sec: 120,
    },
    Exercise {
        id: "pth_neg",
        name_zh: "Press-to-Handstand 離心",
        cues_zh: &[
            "從倒立以 5 秒慢速壓下至站立",
            "肩角全程打開、肘不彎",
            "落下時屈髖收腹吸收衝擊",
        ],
        regression_zh: "改屈腿離心",
        progression_zh: "嘗試地面直腿起頭的完整 PTH",
        base_sets: 4,
        base_reps: "3-4",
        rest_sec: 120,
    },
    Exercise {
        id: "pth_full",
        name_zh: "完整 Press-to-Handstand",
        cues_zh: &[
            "直腿前折、掌根壓地，重心前移過指尖",
            "肩角打開的同時收腹壓髖帶起身體",
            "到倒立頂點後主動推高並停穩",
        ],
        regression_zh: "分腿（straddle）版本降低力矩需求",
        progression_zh: "併腿直腿版本、放慢全程節奏",
        base_sets: 5,
        base_reps: "2-4",
        rest_sec: 150,
    },
];

/// 依 id 取得動作（找不到回傳 None；planner 內部 id 皆為編譯期常數）
pub fn get(id: &str) -> Option<&'static Exercise> {
    EXERCISES.iter().find(|e| e.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_exercises_have_content() {
        for e in EXERCISES {
            assert!(!e.cues_zh.is_empty(), "{} 缺要點", e.id);
            assert!(!e.regression_zh.is_empty(), "{} 缺退階", e.id);
            assert!(!e.progression_zh.is_empty(), "{} 缺進階", e.id);
            assert!(e.base_sets >= 1 && e.base_sets <= 6);
            assert!(e.rest_sec >= 20 && e.rest_sec <= 240);
        }
    }

    #[test]
    fn ids_are_unique() {
        let mut ids: Vec<&str> = EXERCISES.iter().map(|e| e.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), EXERCISES.len());
    }

    #[test]
    fn get_famous_id() {
        assert!(get("pushup").is_some());
        assert!(get("pth_full").is_some());
        assert!(get("nonexistent").is_none());
    }
}
