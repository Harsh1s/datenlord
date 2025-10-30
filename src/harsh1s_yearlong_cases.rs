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
    YearlongCase { date: "2025-08-08", seed: 628732 },
    YearlongCase { date: "2025-08-13", seed: 55102 },
    YearlongCase { date: "2025-08-15", seed: 755025 },
    YearlongCase { date: "2025-08-17", seed: 684060 },
    YearlongCase { date: "2025-08-27", seed: 374128 },
    YearlongCase { date: "2025-08-29", seed: 224957 },
    YearlongCase { date: "2025-09-07", seed: 544068 },
    YearlongCase { date: "2025-09-15", seed: 402436 },
    YearlongCase { date: "2025-09-16", seed: 341769 },
    YearlongCase { date: "2025-09-25", seed: 731216 },
    YearlongCase { date: "2025-09-27", seed: 653599 },
    YearlongCase { date: "2025-10-05", seed: 536175 },
    YearlongCase { date: "2025-10-09", seed: 12556 },
    YearlongCase { date: "2025-10-11", seed: 526029 },
    YearlongCase { date: "2025-10-21", seed: 776883 },
    YearlongCase { date: "2025-10-24", seed: 684858 },
    YearlongCase { date: "2025-10-26", seed: 129973 },
    YearlongCase { date: "2025-10-30", seed: 405432 },
];

pub const fn case_count() -> usize {
    CASES.len()
}
