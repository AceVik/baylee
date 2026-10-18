//! Package 3 Aura behavioral tests — part 08 (24 cards).

#[allow(clippy::wildcard_imports)]
use super::*;

fn cursed_flesh() -> CardIndex {
    // oracle_id = "96e244ef-21b9-4795-a881-c3db23b60cc9"
    card_index("96e244ef-21b9-4795-a881-c3db23b60cc9")
}

fn giant_spectacle() -> CardIndex {
    // oracle_id = "97e34c3a-070e-406a-8505-657f4d57def8"
    card_index("97e34c3a-070e-406a-8505-657f4d57def8")
}

fn asha_s_favor() -> CardIndex {
    // oracle_id = "97e7ada6-dee1-4bb3-a42d-38a61d946bd4"
    card_index("97e7ada6-dee1-4bb3-a42d-38a61d946bd4")
}

fn skeletal_grimace() -> CardIndex {
    // oracle_id = "99e03fa0-8e0c-4b5d-880f-d7de201d5f41"
    card_index("99e03fa0-8e0c-4b5d-880f-d7de201d5f41")
}

fn indestructibility() -> CardIndex {
    // oracle_id = "9aaa0a28-e342-48bd-8f59-0cd289491f98"
    card_index("9aaa0a28-e342-48bd-8f59-0cd289491f98")
}

fn shield_of_the_oversoul() -> CardIndex {
    // oracle_id = "9b598025-80a3-4144-be9a-863165161594"
    card_index("9b598025-80a3-4144-be9a-863165161594")
}

fn knightly_valor() -> CardIndex {
    // oracle_id = "9ba6cec6-97bf-4a83-9ffa-77e187d0945b"
    card_index("9ba6cec6-97bf-4a83-9ffa-77e187d0945b")
}

fn betrayal() -> CardIndex {
    // oracle_id = "9c3d2e1f-a5da-47f4-aea8-5861ef7e1b90"
    card_index("9c3d2e1f-a5da-47f4-aea8-5861ef7e1b90")
}

fn briar_shield() -> CardIndex {
    // oracle_id = "9d437391-d476-4976-8714-6a1d3185ab52"
    card_index("9d437391-d476-4976-8714-6a1d3185ab52")
}

fn threads_of_disloyalty() -> CardIndex {
    // oracle_id = "9d86735c-12e5-457a-beb4-057ab220d18b"
    card_index("9d86735c-12e5-457a-beb4-057ab220d18b")
}

fn parasitic_bond() -> CardIndex {
    // oracle_id = "9d973df5-b2e8-4af8-8379-0f8282e1e62a"
    card_index("9d973df5-b2e8-4af8-8379-0f8282e1e62a")
}

fn castaway_s_despair() -> CardIndex {
    // oracle_id = "9dc4b172-da33-40c0-ab29-d68807dbe93f"
    card_index("9dc4b172-da33-40c0-ab29-d68807dbe93f")
}

fn hardened_scale_armor() -> CardIndex {
    // oracle_id = "9eb58db8-7934-485c-8606-fb1a6cc60d42"
    card_index("9eb58db8-7934-485c-8606-fb1a6cc60d42")
}

fn battle_mastery() -> CardIndex {
    // oracle_id = "a06c7c1a-8534-40fb-8bb8-59f8f2530567"
    card_index("a06c7c1a-8534-40fb-8bb8-59f8f2530567")
}

fn feebleness() -> CardIndex {
    // oracle_id = "a1a7757f-7e9b-4b5b-b9df-e98d8918caac"
    card_index("a1a7757f-7e9b-4b5b-b9df-e98d8918caac")
}

fn siegecraft() -> CardIndex {
    // oracle_id = "a49a9418-72d1-4ab1-89d9-a1f65ae23d35"
    card_index("a49a9418-72d1-4ab1-89d9-a1f65ae23d35")
}

fn persuasion() -> CardIndex {
    // oracle_id = "a503d302-6b12-4d82-9ba6-73efbcb1b1a2"
    card_index("a503d302-6b12-4d82-9ba6-73efbcb1b1a2")
}

fn encase_in_ice() -> CardIndex {
    // oracle_id = "a61277d6-a27d-45d1-ba2a-7afe1b8f5ce1"
    card_index("a61277d6-a27d-45d1-ba2a-7afe1b8f5ce1")
}

fn spiteful_motives() -> CardIndex {
    // oracle_id = "a8f259b5-ef01-4f30-baa1-1b2a5419e61e"
    card_index("a8f259b5-ef01-4f30-baa1-1b2a5419e61e")
}

fn mardu_runemark() -> CardIndex {
    // oracle_id = "a912834b-e4bc-4ccc-ae13-915ebe5acd00"
    card_index("a912834b-e4bc-4ccc-ae13-915ebe5acd00")
}

fn messenger_s_speed() -> CardIndex {
    // oracle_id = "aa21315d-4da9-4bbd-9604-c406b83c0db2"
    card_index("aa21315d-4da9-4bbd-9604-c406b83c0db2")
}

fn marked_by_honor() -> CardIndex {
    // oracle_id = "aa5a7940-976d-4d96-9ddf-1c3891885c7e"
    card_index("aa5a7940-976d-4d96-9ddf-1c3891885c7e")
}

fn stab_wound() -> CardIndex {
    // oracle_id = "aad212b2-57ba-4e70-869b-636842eb7a88"
    card_index("aad212b2-57ba-4e70-869b-636842eb7a88")
}

fn presence_of_gond() -> CardIndex {
    // oracle_id = "ab42398c-f0a1-4b94-ac5f-b8768e1b4e05"
    card_index("ab42398c-f0a1-4b94-ac5f-b8768e1b4e05")
}

#[test]
#[allow(clippy::too_many_lines)]
fn auras_pkg3_part_08() {
    // Cursed Flesh — {B} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: cursed_flesh(),
        primary_mana: &[(ManaColor::Black, 1)],
        alt_mana: None,
        total_mana: 1,
        colors: &[Color::Black],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((-1, -1)),
        expected_set_pt: None,
        expected_kws: &[KeywordSet::FEAR],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Giant Spectacle — {1}{R} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: giant_spectacle(),
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
    // Asha's Favor — {2}{W} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: asha_s_favor(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 3,
        colors: &[Color::White],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[KeywordSet::FLYING
            .union(KeywordSet::FIRST_STRIKE)
            .union(KeywordSet::VIGILANCE)],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Skeletal Grimace — {1}{B} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: skeletal_grimace(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Black, 1)],
        alt_mana: None,
        total_mana: 2,
        colors: &[Color::Black],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((1, 1)),
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Indestructibility — {3}{W} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: indestructibility(),
        primary_mana: &[(ManaColor::Colorless, 3), (ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 4,
        colors: &[Color::White],
        host_kind: AuraHostKind::Permanent,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[KeywordSet::INDESTRUCTIBLE],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Shield of the Oversoul — {2}{G/W} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: shield_of_the_oversoul(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Green, 1)],
        alt_mana: Some(&[(ManaColor::Colorless, 2), (ManaColor::White, 1)]),
        total_mana: 3,
        colors: &[Color::Green, Color::White],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Knightly Valor — {4}{W} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: knightly_valor(),
        primary_mana: &[(ManaColor::Colorless, 4), (ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 5,
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
    // Betrayal — {U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: betrayal(),
        primary_mana: &[(ManaColor::Blue, 1)],
        alt_mana: None,
        total_mana: 1,
        colors: &[Color::Blue],
        host_kind: AuraHostKind::OpponentCreature,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Briar Shield — {G} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: briar_shield(),
        primary_mana: &[(ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 1,
        colors: &[Color::Green],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((1, 1)),
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Threads of Disloyalty — {1}{U}{U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: threads_of_disloyalty(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Blue, 2)],
        alt_mana: None,
        total_mana: 3,
        colors: &[Color::Blue],
        host_kind: AuraHostKind::CreatureMv2OrLess,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: true,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Parasitic Bond — {3}{B} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: parasitic_bond(),
        primary_mana: &[(ManaColor::Colorless, 3), (ManaColor::Black, 1)],
        alt_mana: None,
        total_mana: 4,
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
    // Castaway's Despair — {3}{U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: castaway_s_despair(),
        primary_mana: &[(ManaColor::Colorless, 3), (ManaColor::Blue, 1)],
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
    // Hardened-Scale Armor — {2}{G} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: hardened_scale_armor(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 3,
        colors: &[Color::Green],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((3, 3)),
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Battle Mastery — {2}{W} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: battle_mastery(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 3,
        colors: &[Color::White],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[KeywordSet::DOUBLE_STRIKE],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Feebleness — {1}{B} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: feebleness(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Black, 1)],
        alt_mana: None,
        total_mana: 2,
        colors: &[Color::Black],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((-2, -1)),
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Siegecraft — {3}{W} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: siegecraft(),
        primary_mana: &[(ManaColor::Colorless, 3), (ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 4,
        colors: &[Color::White],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((2, 4)),
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Persuasion — {3}{U}{U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: persuasion(),
        primary_mana: &[(ManaColor::Colorless, 3), (ManaColor::Blue, 2)],
        alt_mana: None,
        total_mana: 5,
        colors: &[Color::Blue],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: true,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Encase in Ice — {1}{U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: encase_in_ice(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Blue, 1)],
        alt_mana: None,
        total_mana: 2,
        colors: &[Color::Blue],
        host_kind: AuraHostKind::RedOrGreenCreature,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: true,
        etb_draws: 0,
        etb_taps_attached: true,
    });
    // Spiteful Motives — {3}{R} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: spiteful_motives(),
        primary_mana: &[(ManaColor::Colorless, 3), (ManaColor::Red, 1)],
        alt_mana: None,
        total_mana: 4,
        colors: &[Color::Red],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((3, 0)),
        expected_set_pt: None,
        expected_kws: &[KeywordSet::FIRST_STRIKE],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Mardu Runemark — {2}{R} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: mardu_runemark(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Red, 1)],
        alt_mana: None,
        total_mana: 3,
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
    // Messenger's Speed — {R} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: messenger_s_speed(),
        primary_mana: &[(ManaColor::Red, 1)],
        alt_mana: None,
        total_mana: 1,
        colors: &[Color::Red],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[KeywordSet::TRAMPLE.union(KeywordSet::HASTE)],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Marked by Honor — {3}{W} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: marked_by_honor(),
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
    // Stab Wound — {2}{B} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: stab_wound(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Black, 1)],
        alt_mana: None,
        total_mana: 3,
        colors: &[Color::Black],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((-2, -2)),
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Presence of Gond — {2}{G} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: presence_of_gond(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 3,
        colors: &[Color::Green],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
}
