// Shared with the client's role test through include!: no renderer dependency.
/// English/German label limits. Count badges and key caps are not label text.
pub const ROLE_BUDGETS: &[(&str, usize, usize)] = &[
    ("button", 18, 20),
    ("chip", 14, 14),
    ("segment", 14, 14),
    ("tab", 16, 16),
    ("nav", 12, 13),
];
