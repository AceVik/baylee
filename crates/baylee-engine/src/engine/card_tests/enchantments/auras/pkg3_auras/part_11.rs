//! Package 3 Aura behavioral tests — part 11 (24 cards).

#[allow(clippy::wildcard_imports)]
use super::*;

fn malfunction() -> CardIndex {
    // oracle_id = "d2f98983-f395-4166-aa5a-309b4769baab"
    card_index("d2f98983-f395-4166-aa5a-309b4769baab")
}

fn containment_protocol() -> CardIndex {
    // oracle_id = "d33c7ea2-4d17-4cdb-aa58-c25c57836297"
    card_index("d33c7ea2-4d17-4cdb-aa58-c25c57836297")
}

fn fae_flight() -> CardIndex {
    // oracle_id = "d3912c82-37f8-456e-ba49-65c7f5b39d13"
    card_index("d3912c82-37f8-456e-ba49-65c7f5b39d13")
}

fn fate_foretold() -> CardIndex {
    // oracle_id = "d578ef54-6a6b-4839-a059-261728fdaef1"
    card_index("d578ef54-6a6b-4839-a059-261728fdaef1")
}

fn confiscate() -> CardIndex {
    // oracle_id = "d599a38c-7719-443e-bb59-55bd43a8fee6"
    card_index("d599a38c-7719-443e-bb59-55bd43a8fee6")
}

fn yavimaya_s_embrace() -> CardIndex {
    // oracle_id = "d5cb0370-a6f0-40c0-8b8f-553fca23778d"
    card_index("d5cb0370-a6f0-40c0-8b8f-553fca23778d")
}

fn sleeper_s_robe() -> CardIndex {
    // oracle_id = "d82783d9-9bfe-4fcd-a7d2-6e2477f99edc"
    card_index("d82783d9-9bfe-4fcd-a7d2-6e2477f99edc")
}

fn super_intelligence() -> CardIndex {
    // oracle_id = "d82929f2-57ad-45be-a834-984437046026"
    card_index("d82929f2-57ad-45be-a834-984437046026")
}

fn torpor_dust() -> CardIndex {
    // oracle_id = "da1ca3ec-8bef-4b7e-896e-4d50e8cffbe5"
    card_index("da1ca3ec-8bef-4b7e-896e-4d50e8cffbe5")
}

fn gift_of_orzhova() -> CardIndex {
    // oracle_id = "dcb6d317-5c55-4973-8e3d-2e98211dc30e"
    card_index("dcb6d317-5c55-4973-8e3d-2e98211dc30e")
}

fn tattoo_ward() -> CardIndex {
    // oracle_id = "ddb44519-1345-4990-a8a5-30e91d274a35"
    card_index("ddb44519-1345-4990-a8a5-30e91d274a35")
}

fn mightstone_s_animation() -> CardIndex {
    // oracle_id = "de68a69a-0c4e-43b9-821d-2fe2269b5e48"
    card_index("de68a69a-0c4e-43b9-821d-2fe2269b5e48")
}

fn akki_war_paint() -> CardIndex {
    // oracle_id = "e01ab6d8-8419-436e-a8fb-10aa46ca3d05"
    card_index("e01ab6d8-8419-436e-a8fb-10aa46ca3d05")
}

fn cradle_of_safety() -> CardIndex {
    // oracle_id = "e0717287-9bff-446b-838d-6fc0adccc768"
    card_index("e0717287-9bff-446b-838d-6fc0adccc768")
}

fn failed_conversion() -> CardIndex {
    // oracle_id = "e0930791-72b0-4d8b-b242-7b4094fed14e"
    card_index("e0930791-72b0-4d8b-b242-7b4094fed14e")
}

fn gift_of_granite() -> CardIndex {
    // oracle_id = "e1abac67-ae07-4bb2-a048-7e41e0fc0371"
    card_index("e1abac67-ae07-4bb2-a048-7e41e0fc0371")
}

fn dub() -> CardIndex {
    // oracle_id = "e3ca6b70-01d2-44e9-937a-a2d50bbf9d10"
    card_index("e3ca6b70-01d2-44e9-937a-a2d50bbf9d10")
}

fn shiv_s_embrace() -> CardIndex {
    // oracle_id = "e41162bf-1a4f-4e69-870a-faf9836c56d5"
    card_index("e41162bf-1a4f-4e69-870a-faf9836c56d5")
}

fn numbing_dose() -> CardIndex {
    // oracle_id = "e42d087e-c999-4f9a-bdc5-4327e8ce94b6"
    card_index("e42d087e-c999-4f9a-bdc5-4327e8ce94b6")
}

fn soul_bleed() -> CardIndex {
    // oracle_id = "e4f1acd6-b883-470b-88c2-3989011869d7"
    card_index("e4f1acd6-b883-470b-88c2-3989011869d7")
}

fn angelic_gift() -> CardIndex {
    // oracle_id = "e5e04968-d9b7-4bd5-b826-be9502360cd3"
    card_index("e5e04968-d9b7-4bd5-b826-be9502360cd3")
}

fn ocular_halo() -> CardIndex {
    // oracle_id = "e649614a-ff23-4234-92f4-6f564cbfe648"
    card_index("e649614a-ff23-4234-92f4-6f564cbfe648")
}

fn oblivion_crown() -> CardIndex {
    // oracle_id = "e690461b-2925-4484-b6fa-69ca1166f5a2"
    card_index("e690461b-2925-4484-b6fa-69ca1166f5a2")
}

fn overgrowth() -> CardIndex {
    // oracle_id = "e6ccebf4-f4f0-404f-a634-4d751ef9c8aa"
    card_index("e6ccebf4-f4f0-404f-a634-4d751ef9c8aa")
}

#[test]
#[allow(clippy::too_many_lines)]
fn auras_pkg3_part_11() {
    // Malfunction — {3}{U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: malfunction(),
        primary_mana: &[(ManaColor::Colorless, 3), (ManaColor::Blue, 1)],
        alt_mana: None,
        total_mana: 4,
        colors: &[Color::Blue],
        host_kind: AuraHostKind::ArtifactOrCreature,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: true,
        etb_draws: 0,
        etb_taps_attached: true,
    });
    // Containment Protocol — {2}{U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: containment_protocol(),
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
    // Fae Flight — {1}{U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: fae_flight(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Blue, 1)],
        alt_mana: None,
        total_mana: 2,
        colors: &[Color::Blue],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((1, 0)),
        expected_set_pt: None,
        expected_kws: &[KeywordSet::FLYING],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Fate Foretold — {1}{U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: fate_foretold(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Blue, 1)],
        alt_mana: None,
        total_mana: 2,
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
    // Confiscate — {4}{U}{U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: confiscate(),
        primary_mana: &[(ManaColor::Colorless, 4), (ManaColor::Blue, 2)],
        alt_mana: None,
        total_mana: 6,
        colors: &[Color::Blue],
        host_kind: AuraHostKind::Permanent,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: true,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Yavimaya's Embrace — {5}{G}{U}{U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: yavimaya_s_embrace(),
        primary_mana: &[
            (ManaColor::Colorless, 5),
            (ManaColor::Blue, 2),
            (ManaColor::Green, 1),
        ],
        alt_mana: None,
        total_mana: 8,
        colors: &[Color::Green, Color::Blue],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((2, 2)),
        expected_set_pt: None,
        expected_kws: &[KeywordSet::TRAMPLE],
        gain_control: true,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Sleeper's Robe — {U}{B} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: sleeper_s_robe(),
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Black, 1)],
        alt_mana: None,
        total_mana: 2,
        colors: &[Color::Black, Color::Blue],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[KeywordSet::FEAR],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Super Intelligence — {U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: super_intelligence(),
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
    // Torpor Dust — {2}{U/B} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: torpor_dust(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Blue, 1)],
        alt_mana: Some(&[(ManaColor::Colorless, 2), (ManaColor::Black, 1)]),
        total_mana: 3,
        colors: &[Color::Black, Color::Blue],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((-3, 0)),
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Gift of Orzhova — {1}{W/B}{W/B} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: gift_of_orzhova(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::White, 2)],
        alt_mana: Some(&[(ManaColor::Colorless, 1), (ManaColor::Black, 2)]),
        total_mana: 3,
        colors: &[Color::Black, Color::White],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((1, 1)),
        expected_set_pt: None,
        expected_kws: &[KeywordSet::LIFELINK.union(KeywordSet::FLYING)],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Tattoo Ward — {2}{W} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: tattoo_ward(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 3,
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
    // Mightstone's Animation — {3}{U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: mightstone_s_animation(),
        primary_mana: &[(ManaColor::Colorless, 3), (ManaColor::Blue, 1)],
        alt_mana: None,
        total_mana: 4,
        colors: &[Color::Blue],
        host_kind: AuraHostKind::Artifact,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 1,
        etb_taps_attached: false,
    });
    // Akki War Paint — {R} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: akki_war_paint(),
        primary_mana: &[(ManaColor::Red, 1)],
        alt_mana: None,
        total_mana: 1,
        colors: &[Color::Red],
        host_kind: AuraHostKind::ArtifactOrCreature,
        expected_pt_delta: Some((2, 1)),
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Cradle of Safety — {1}{U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: cradle_of_safety(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Blue, 1)],
        alt_mana: None,
        total_mana: 2,
        colors: &[Color::Blue],
        host_kind: AuraHostKind::YourCreature,
        expected_pt_delta: Some((1, 1)),
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Failed Conversion — {4}{B} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: failed_conversion(),
        primary_mana: &[(ManaColor::Colorless, 4), (ManaColor::Black, 1)],
        alt_mana: None,
        total_mana: 5,
        colors: &[Color::Black],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((-4, -4)),
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Gift of Granite — {W} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: gift_of_granite(),
        primary_mana: &[(ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 1,
        colors: &[Color::White],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((0, 2)),
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Dub — {2}{W} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: dub(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 3,
        colors: &[Color::White],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((2, 2)),
        expected_set_pt: None,
        expected_kws: &[KeywordSet::FIRST_STRIKE],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Shiv's Embrace — {2}{R}{R} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: shiv_s_embrace(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Red, 2)],
        alt_mana: None,
        total_mana: 4,
        colors: &[Color::Red],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((2, 2)),
        expected_set_pt: None,
        expected_kws: &[KeywordSet::FLYING],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Numbing Dose — {3}{U}{U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: numbing_dose(),
        primary_mana: &[(ManaColor::Colorless, 3), (ManaColor::Blue, 2)],
        alt_mana: None,
        total_mana: 5,
        colors: &[Color::Blue],
        host_kind: AuraHostKind::ArtifactOrCreature,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: true,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Soul Bleed — {2}{B} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: soul_bleed(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Black, 1)],
        alt_mana: None,
        total_mana: 3,
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
    // Angelic Gift — {1}{W} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: angelic_gift(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 2,
        colors: &[Color::White],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[KeywordSet::FLYING],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 1,
        etb_taps_attached: false,
    });
    // Ocular Halo — {3}{U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: ocular_halo(),
        primary_mana: &[(ManaColor::Colorless, 3), (ManaColor::Blue, 1)],
        alt_mana: None,
        total_mana: 4,
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
    // Oblivion Crown — {1}{B} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: oblivion_crown(),
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
    // Overgrowth — {2}{G} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: overgrowth(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 3,
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
}
