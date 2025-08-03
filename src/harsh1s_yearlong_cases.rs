#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct YearlongCase {
    pub date: &'static str,
    pub seed: u32,
}

pub const CASES: &[YearlongCase] = &[
    YearlongCase { date: "2025-06-06", seed: 133816 },
    YearlongCase { date: "2025-06-14", seed: 320767 },
    YearlongCase { date: "2025-06-20", seed: 587777 },
    YearlongCase { date: "2025-06-29", seed: 803637 },
    YearlongCase { date: "2025-07-05", seed: 1191 },
    YearlongCase { date: "2025-07-06", seed: 592586 },
    YearlongCase { date: "2025-07-11", seed: 349854 },
    YearlongCase { date: "2025-07-15", seed: 603943 },
    YearlongCase { date: "2025-07-26", seed: 347791 },
    YearlongCase { date: "2025-08-02", seed: 827642 },
    YearlongCase { date: "2025-08-03", seed: 65675 },
];

pub const fn case_count() -> usize {
    CASES.len()
}
