//! Package 3 Aura behavioral tests — part 02 (24 cards).

#[allow(clippy::wildcard_imports)]
use super::*;

fn most_wanted() -> CardIndex {
    // oracle_id = "140d38cc-0ae6-4e9b-a60f-a51d2947e07b"
    card_index("140d38cc-0ae6-4e9b-a60f-a51d2947e07b")
}

fn serra_s_boon() -> CardIndex {
    // oracle_id = "1521f26b-9972-4e95-9e14-3ccbb078a1c8"
    card_index("1521f26b-9972-4e95-9e14-3ccbb078a1c8")
}

fn spirit_away() -> CardIndex {
    // oracle_id = "15b618f0-281f-4745-8504-bd70e7b4a267"
    card_index("15b618f0-281f-4745-8504-bd70e7b4a267")
}

fn prodigious_growth() -> CardIndex {
    // oracle_id = "15d63e5d-dc4d-4e5f-8043-78039a49732b"
    card_index("15d63e5d-dc4d-4e5f-8043-78039a49732b")
}

fn edge_of_the_divinity() -> CardIndex {
    // oracle_id = "16ef24ea-4d84-43e5-bd4f-b6efadc87616"
    card_index("16ef24ea-4d84-43e5-bd4f-b6efadc87616")
}

fn keldon_mantle() -> CardIndex {
    // oracle_id = "170cc0c4-0fba-4751-8613-e934b0438364"
    card_index("170cc0c4-0fba-4751-8613-e934b0438364")
}

fn flowstone_embrace() -> CardIndex {
    // oracle_id = "18a45b81-1946-4be6-8f2c-8c21d11c86f8"
    card_index("18a45b81-1946-4be6-8f2c-8c21d11c86f8")
}

fn spawning_grounds() -> CardIndex {
    // oracle_id = "1961dd92-db0b-4f02-b9c8-08f760f4051b"
    card_index("1961dd92-db0b-4f02-b9c8-08f760f4051b")
}

fn defensive_stance() -> CardIndex {
    // oracle_id = "19959b6d-eb8f-4273-94c8-65210e1998ab"
    card_index("19959b6d-eb8f-4273-94c8-65210e1998ab")
}

fn cooperation() -> CardIndex {
    // oracle_id = "19f7a0f2-aa3f-4253-be44-b08056918a26"
    card_index("19f7a0f2-aa3f-4253-be44-b08056918a26")
}

fn frenzied_rage() -> CardIndex {
    // oracle_id = "1a04c31c-f5e4-43ff-a5b5-207426b8dca6"
    card_index("1a04c31c-f5e4-43ff-a5b5-207426b8dca6")
}

fn molting_snakeskin() -> CardIndex {
    // oracle_id = "1a668556-da98-4f1e-b1f7-26b7d63357a1"
    card_index("1a668556-da98-4f1e-b1f7-26b7d63357a1")
}

fn burden_of_guilt() -> CardIndex {
    // oracle_id = "1ae88ba6-4973-45d1-a251-4d924ab4bc73"
    card_index("1ae88ba6-4973-45d1-a251-4d924ab4bc73")
}

fn armor_of_faith() -> CardIndex {
    // oracle_id = "1c482b20-b423-48ef-b162-53960d43b4f7"
    card_index("1c482b20-b423-48ef-b162-53960d43b4f7")
}

fn shield_of_duty_and_reason() -> CardIndex {
    // oracle_id = "1c64ab87-1da1-4b07-a3f2-73631ae3d501"
    card_index("1c64ab87-1da1-4b07-a3f2-73631ae3d501")
}

fn captain_s_tutelage() -> CardIndex {
    // oracle_id = "1d41aacf-e15d-4457-b12a-c71bcdb7b5de"
    card_index("1d41aacf-e15d-4457-b12a-c71bcdb7b5de")
}

fn second_wind() -> CardIndex {
    // oracle_id = "1d8569c1-0b10-40a7-9a87-38571a782dd9"
    card_index("1d8569c1-0b10-40a7-9a87-38571a782dd9")
}

fn lightning_diadem() -> CardIndex {
    // oracle_id = "1e4de77b-27cb-4afd-8d4b-1cc89f2a6986"
    card_index("1e4de77b-27cb-4afd-8d4b-1cc89f2a6986")
}

fn binding_grasp() -> CardIndex {
    // oracle_id = "1ebdc091-16d5-4803-821a-ac8c4696b5bc"
    card_index("1ebdc091-16d5-4803-821a-ac8c4696b5bc")
}

fn boon_of_emrakul() -> CardIndex {
    // oracle_id = "20d30d4d-cf38-41fe-a2ce-8327f7c87c01"
    card_index("20d30d4d-cf38-41fe-a2ce-8327f7c87c01")
}

fn mask_of_law_and_grace() -> CardIndex {
    // oracle_id = "21474195-5d91-466d-8874-4818c900dea7"
    card_index("21474195-5d91-466d-8874-4818c900dea7")
}

fn curiosity() -> CardIndex {
    // oracle_id = "223fa044-d387-4884-bf4e-75f1b61c6a46"
    card_index("223fa044-d387-4884-bf4e-75f1b61c6a46")
}

fn air_bladder() -> CardIndex {
    // oracle_id = "25c8a3b0-c393-4fc6-a373-808cd8e9850c"
    card_index("25c8a3b0-c393-4fc6-a373-808cd8e9850c")
}

fn aerial_modification() -> CardIndex {
    // oracle_id = "290cd0b1-55d6-4b30-962d-075a6f75d562"
    card_index("290cd0b1-55d6-4b30-962d-075a6f75d562")
}

#[test]
#[allow(clippy::too_many_lines)]
fn auras_pkg3_part_02() {
    // Most Wanted — {2}{G} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: most_wanted(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 3,
        colors: &[Color::Green],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((2, 1)),
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Serra's Boon — {2}{W} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: serra_s_boon(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 3,
        colors: &[Color::White],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Spirit Away — {5}{U}{U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: spirit_away(),
        primary_mana: &[(ManaColor::Colorless, 5), (ManaColor::Blue, 2)],
        alt_mana: None,
        total_mana: 7,
        colors: &[Color::Blue],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((2, 2)),
        expected_set_pt: None,
        expected_kws: &[KeywordSet::FLYING],
        gain_control: true,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Prodigious Growth — {4}{G}{G} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: prodigious_growth(),
        primary_mana: &[(ManaColor::Colorless, 4), (ManaColor::Green, 2)],
        alt_mana: None,
        total_mana: 6,
        colors: &[Color::Green],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((7, 7)),
        expected_set_pt: None,
        expected_kws: &[KeywordSet::TRAMPLE],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Edge of the Divinity — {W/B} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: edge_of_the_divinity(),
        primary_mana: &[(ManaColor::White, 1)],
        alt_mana: Some(&[(ManaColor::Black, 1)]),
        total_mana: 1,
        colors: &[Color::Black, Color::White],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Keldon Mantle — {1}{R} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: keldon_mantle(),
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
    // Flowstone Embrace — {1}{R} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: flowstone_embrace(),
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
    // Spawning Grounds — {6}{G}{G} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: spawning_grounds(),
        primary_mana: &[(ManaColor::Colorless, 6), (ManaColor::Green, 2)],
        alt_mana: None,
        total_mana: 8,
        colors: &[Color::Green],
        host_kind: AuraHostKind::Land,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Defensive Stance — {U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: defensive_stance(),
        primary_mana: &[(ManaColor::Blue, 1)],
        alt_mana: None,
        total_mana: 1,
        colors: &[Color::Blue],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((-1, 1)),
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Cooperation — {2}{W} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: cooperation(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 3,
        colors: &[Color::White],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[KeywordSet::BANDING],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Frenzied Rage — {1}{R} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: frenzied_rage(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Red, 1)],
        alt_mana: None,
        total_mana: 2,
        colors: &[Color::Red],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((2, 1)),
        expected_set_pt: None,
        expected_kws: &[KeywordSet::MENACE],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Molting Snakeskin — {B} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: molting_snakeskin(),
        primary_mana: &[(ManaColor::Black, 1)],
        alt_mana: None,
        total_mana: 1,
        colors: &[Color::Black],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((2, 0)),
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Burden of Guilt — {W} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: burden_of_guilt(),
        primary_mana: &[(ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 1,
        colors: &[Color::White],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Armor of Faith — {W} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: armor_of_faith(),
        primary_mana: &[(ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 1,
        colors: &[Color::White],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((1, 1)),
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Shield of Duty and Reason — {W} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: shield_of_duty_and_reason(),
        primary_mana: &[(ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 1,
        colors: &[Color::White],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Captain's Tutelage — {1}{R} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: captain_s_tutelage(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Red, 1)],
        alt_mana: None,
        total_mana: 2,
        colors: &[Color::Red],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((2, 0)),
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Second Wind — {2}{U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: second_wind(),
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
    // Lightning Diadem — {5}{R} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: lightning_diadem(),
        primary_mana: &[(ManaColor::Colorless, 5), (ManaColor::Red, 1)],
        alt_mana: None,
        total_mana: 6,
        colors: &[Color::Red],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((2, 2)),
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Binding Grasp — {3}{U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: binding_grasp(),
        primary_mana: &[(ManaColor::Colorless, 3), (ManaColor::Blue, 1)],
        alt_mana: None,
        total_mana: 4,
        colors: &[Color::Blue],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((0, 1)),
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: true,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Boon of Emrakul — {2}{B} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: boon_of_emrakul(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Black, 1)],
        alt_mana: None,
        total_mana: 3,
        colors: &[Color::Black],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((3, -3)),
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Mask of Law and Grace — {W} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: mask_of_law_and_grace(),
        primary_mana: &[(ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 1,
        colors: &[Color::White],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Curiosity — {U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: curiosity(),
        primary_mana: &[(ManaColor::Blue, 1)],
        alt_mana: None,
        total_mana: 1,
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
    // Air Bladder — {U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: air_bladder(),
        primary_mana: &[(ManaColor::Blue, 1)],
        alt_mana: None,
        total_mana: 1,
        colors: &[Color::Blue],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[KeywordSet::FLYING],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Aerial Modification — {4}{W} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: aerial_modification(),
        primary_mana: &[(ManaColor::Colorless, 4), (ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 5,
        colors: &[Color::White],
        host_kind: AuraHostKind::CreatureOrVehicle,
        expected_pt_delta: Some((2, 2)),
        expected_set_pt: None,
        expected_kws: &[KeywordSet::FLYING],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
}
