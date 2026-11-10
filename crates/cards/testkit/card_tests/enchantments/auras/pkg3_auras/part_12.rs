//! Package 3 Aura behavioral tests — part 12 (13 cards).

#[allow(clippy::wildcard_imports)]
use super::*;

fn talons_of_falkenrath() -> CardIndex {
    // oracle_id = "e79b1224-1baf-460f-bf12-92b51cd8071f"
    card_index("e79b1224-1baf-460f-bf12-92b51cd8071f")
}

fn ophidian_eye() -> CardIndex {
    // oracle_id = "e83b8830-fa97-4e60-b042-005e552a3ceb"
    card_index("e83b8830-fa97-4e60-b042-005e552a3ceb")
}

fn magefire_wings() -> CardIndex {
    // oracle_id = "ea8c4a7d-f7fb-4620-b3a4-ae8280dd60d9"
    card_index("ea8c4a7d-f7fb-4620-b3a4-ae8280dd60d9")
}

fn sea_s_claim() -> CardIndex {
    // oracle_id = "ed11efb6-1423-4ce0-be1d-9e0320124758"
    card_index("ed11efb6-1423-4ce0-be1d-9e0320124758")
}

fn tractor_beam() -> CardIndex {
    // oracle_id = "ed99a0b8-d82d-4c80-af97-cabd333620df"
    card_index("ed99a0b8-d82d-4c80-af97-cabd333620df")
}

fn stratus_walk() -> CardIndex {
    // oracle_id = "eda37caa-d676-460a-804b-3ba9d8a8d044"
    card_index("eda37caa-d676-460a-804b-3ba9d8a8d044")
}

fn mystic_restraints() -> CardIndex {
    // oracle_id = "efb91243-2617-495f-adb0-1c26a669fb80"
    card_index("efb91243-2617-495f-adb0-1c26a669fb80")
}

fn furious_strength() -> CardIndex {
    // oracle_id = "efefb339-bb7c-4b26-abed-8434f955122b"
    card_index("efefb339-bb7c-4b26-abed-8434f955122b")
}

fn starlight_snare() -> CardIndex {
    // oracle_id = "f0691d46-5bf4-4617-8a0a-19d17afb7219"
    card_index("f0691d46-5bf4-4617-8a0a-19d17afb7219")
}

fn shadow_lance() -> CardIndex {
    // oracle_id = "f1a2a78c-2794-4bc9-9b06-49306378ef69"
    card_index("f1a2a78c-2794-4bc9-9b06-49306378ef69")
}

fn midnight_covenant() -> CardIndex {
    // oracle_id = "f3f2db4b-4a07-453c-ae18-aeec0adc8d3d"
    card_index("f3f2db4b-4a07-453c-ae18-aeec0adc8d3d")
}

fn chosen_by_heliod() -> CardIndex {
    // oracle_id = "f6475b40-e694-4aac-a25c-bbd48d45127d"
    card_index("f6475b40-e694-4aac-a25c-bbd48d45127d")
}

fn iona_s_blessing() -> CardIndex {
    // oracle_id = "fc5bde87-231e-424a-a8fa-cd9910a09d43"
    card_index("fc5bde87-231e-424a-a8fa-cd9910a09d43")
}

#[test]
#[allow(clippy::too_many_lines)]
fn auras_pkg3_part_12() {
    // Talons of Falkenrath — {1}{R} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: talons_of_falkenrath(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Red, 1)],
        alt_mana: None,
        total_mana: 2,
        colors: &[Color::Red],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Ophidian Eye — {2}{U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: ophidian_eye(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Blue, 1)],
        alt_mana: None,
        total_mana: 3,
        colors: &[Color::Blue],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Magefire Wings — {U}{R} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: magefire_wings(),
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Red, 1)],
        alt_mana: None,
        total_mana: 2,
        colors: &[Color::Red, Color::Blue],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((2, 0)),
        expected_set_pt: None,
        expected_kws: &[KeywordSet::FLYING],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Sea's Claim — {U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: sea_s_claim(),
        primary_mana: &[(ManaColor::Blue, 1)],
        alt_mana: None,
        total_mana: 1,
        colors: &[Color::Blue],
        host_kind: AuraHostKind::Land,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Tractor Beam — {2}{U}{U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: tractor_beam(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Blue, 2)],
        alt_mana: None,
        total_mana: 4,
        colors: &[Color::Blue],
        host_kind: AuraHostKind::CreatureOrSpacecraft,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: true,
        does_not_untap: true,
        etb_draws: 0,
        etb_taps_attached: true,
    });
    // Stratus Walk — {1}{U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: stratus_walk(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Blue, 1)],
        alt_mana: None,
        total_mana: 2,
        colors: &[Color::Blue],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[KeywordSet::FLYING],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 1,
        etb_taps_attached: false,
    });
    // Mystic Restraints — {2}{U}{U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: mystic_restraints(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Blue, 2)],
        alt_mana: None,
        total_mana: 4,
        colors: &[Color::Blue],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: true,
        etb_draws: 0,
        etb_taps_attached: true,
    });
    // Furious Strength — {2}{R} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: furious_strength(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Red, 1)],
        alt_mana: None,
        total_mana: 3,
        colors: &[Color::Red],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((2, 2)),
        expected_set_pt: None,
        expected_kws: &[KeywordSet::MENACE],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Starlight Snare — {2}{U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: starlight_snare(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Blue, 1)],
        alt_mana: None,
        total_mana: 3,
        colors: &[Color::Blue],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: true,
        etb_draws: 0,
        etb_taps_attached: true,
    });
    // Shadow Lance — {W} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: shadow_lance(),
        primary_mana: &[(ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 1,
        colors: &[Color::White],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[KeywordSet::FIRST_STRIKE],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Midnight Covenant — {1}{B} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: midnight_covenant(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Black, 1)],
        alt_mana: None,
        total_mana: 2,
        colors: &[Color::Black],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Chosen by Heliod — {1}{W} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: chosen_by_heliod(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 2,
        colors: &[Color::White],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((0, 2)),
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 1,
        etb_taps_attached: false,
    });
    // Iona's Blessing — {3}{W} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: iona_s_blessing(),
        primary_mana: &[(ManaColor::Colorless, 3), (ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 4,
        colors: &[Color::White],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((2, 2)),
        expected_set_pt: None,
        expected_kws: &[KeywordSet::VIGILANCE],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
}
