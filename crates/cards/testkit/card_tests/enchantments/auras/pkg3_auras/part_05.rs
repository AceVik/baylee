//! Package 3 Aura behavioral tests — part 05 (24 cards).

#[allow(clippy::wildcard_imports)]
use super::*;

fn melancholy() -> CardIndex {
    // oracle_id = "58cc7682-f8ce-47cf-a291-0301fea9ddfd"
    card_index("58cc7682-f8ce-47cf-a291-0301fea9ddfd")
}

fn temur_runemark() -> CardIndex {
    // oracle_id = "590c2455-90f7-4ff6-a6cf-5d0d25575cda"
    card_index("590c2455-90f7-4ff6-a6cf-5d0d25575cda")
}

fn mind_control() -> CardIndex {
    // oracle_id = "5912546a-acc2-448c-b042-64bdac5ec129"
    card_index("5912546a-acc2-448c-b042-64bdac5ec129")
}

fn sultai_runemark() -> CardIndex {
    // oracle_id = "59410454-93ff-41f0-ab62-cdaf156163d0"
    card_index("59410454-93ff-41f0-ab62-cdaf156163d0")
}

fn sleep_paralysis() -> CardIndex {
    // oracle_id = "5a8bd4a4-8b10-4eef-8341-28be2afa2c00"
    card_index("5a8bd4a4-8b10-4eef-8341-28be2afa2c00")
}

fn clinging_darkness() -> CardIndex {
    // oracle_id = "5ad3abf3-1ae5-4c4c-9950-ba7bff0d75ad"
    card_index("5ad3abf3-1ae5-4c4c-9950-ba7bff0d75ad")
}

fn feral_invocation() -> CardIndex {
    // oracle_id = "5b3c069b-5bed-4b70-b04c-b7143a4a6e96"
    card_index("5b3c069b-5bed-4b70-b04c-b7143a4a6e96")
}

fn chant_of_the_skifsang() -> CardIndex {
    // oracle_id = "5c902588-b16e-4679-b89e-7b11c928061f"
    card_index("5c902588-b16e-4679-b89e-7b11c928061f")
}

fn frantic_strength() -> CardIndex {
    // oracle_id = "5dbe4648-f4c9-4d26-98f5-58a7e53b149d"
    card_index("5dbe4648-f4c9-4d26-98f5-58a7e53b149d")
}

fn short_circuit() -> CardIndex {
    // oracle_id = "5ea0e3a5-9c38-4da4-98a6-41c14be512d9"
    card_index("5ea0e3a5-9c38-4da4-98a6-41c14be512d9")
}

fn gilt_leaf_s_embrace() -> CardIndex {
    // oracle_id = "5eb8bb1d-7567-4c24-913b-463b5fe463c4"
    card_index("5eb8bb1d-7567-4c24-913b-463b5fe463c4")
}

fn annex() -> CardIndex {
    // oracle_id = "5f1f49dd-b7ce-494a-9b4b-ac702d3664c5"
    card_index("5f1f49dd-b7ce-494a-9b4b-ac702d3664c5")
}

fn jeskai_runemark() -> CardIndex {
    // oracle_id = "5fe13418-c63c-4f3c-9a42-7ab06949e3cd"
    card_index("5fe13418-c63c-4f3c-9a42-7ab06949e3cd")
}

fn pin_to_the_earth() -> CardIndex {
    // oracle_id = "605624ee-b7ca-4728-993f-e13b18cc57f3"
    card_index("605624ee-b7ca-4728-993f-e13b18cc57f3")
}

fn almost_perfect() -> CardIndex {
    // oracle_id = "617eae63-b5f8-4014-8269-7c66f4388ac7"
    card_index("617eae63-b5f8-4014-8269-7c66f4388ac7")
}

fn thirst() -> CardIndex {
    // oracle_id = "61b3ad9d-485f-4479-b18f-506847af43f2"
    card_index("61b3ad9d-485f-4479-b18f-506847af43f2")
}

fn claustrophobia() -> CardIndex {
    // oracle_id = "62d8c8c8-bc24-42f2-9e2e-9efd08e47bb1"
    card_index("62d8c8c8-bc24-42f2-9e2e-9efd08e47bb1")
}

fn knight_s_pledge() -> CardIndex {
    // oracle_id = "63604320-68de-415e-b1f9-dd0f85c5d8a3"
    card_index("63604320-68de-415e-b1f9-dd0f85c5d8a3")
}

fn accelerated_evolution() -> CardIndex {
    // oracle_id = "6401d0ae-4635-497d-91ef-ce2eabefb893"
    card_index("6401d0ae-4635-497d-91ef-ce2eabefb893")
}

fn ghoulflesh() -> CardIndex {
    // oracle_id = "640c0424-b862-438f-a527-ff606151040a"
    card_index("640c0424-b862-438f-a527-ff606151040a")
}

fn triclopean_sight() -> CardIndex {
    // oracle_id = "6441456a-cc20-48bb-95e9-c981fea8ea98"
    card_index("6441456a-cc20-48bb-95e9-c981fea8ea98")
}

fn mantle_of_leadership() -> CardIndex {
    // oracle_id = "65c0d59b-3a94-49af-94cd-2ab4820a51f3"
    card_index("65c0d59b-3a94-49af-94cd-2ab4820a51f3")
}

fn scourgemark() -> CardIndex {
    // oracle_id = "66efb37f-9eab-4ff4-a1d4-65fe9ca7332b"
    card_index("66efb37f-9eab-4ff4-a1d4-65fe9ca7332b")
}

fn runes_of_the_deus() -> CardIndex {
    // oracle_id = "674b9469-3213-4cb0-be7e-db209bc09f92"
    card_index("674b9469-3213-4cb0-be7e-db209bc09f92")
}

#[test]
#[allow(clippy::too_many_lines)]
fn auras_pkg3_part_05() {
    // Melancholy — {2}{B} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: melancholy(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Black, 1)],
        alt_mana: None,
        total_mana: 3,
        colors: &[Color::Black],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: true,
        etb_draws: 0,
        etb_taps_attached: true,
    });
    // Temur Runemark — {2}{G} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: temur_runemark(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 3,
        colors: &[Color::Green],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((2, 2)),
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Mind Control — {3}{U}{U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: mind_control(),
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
    // Sultai Runemark — {2}{B} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: sultai_runemark(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Black, 1)],
        alt_mana: None,
        total_mana: 3,
        colors: &[Color::Black],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((2, 2)),
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Sleep Paralysis — {3}{U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: sleep_paralysis(),
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
    // Clinging Darkness — {1}{B} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: clinging_darkness(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Black, 1)],
        alt_mana: None,
        total_mana: 2,
        colors: &[Color::Black],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((-4, -1)),
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Feral Invocation — {2}{G} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: feral_invocation(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 3,
        colors: &[Color::Green],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((2, 2)),
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Chant of the Skifsang — {2}{U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: chant_of_the_skifsang(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Blue, 1)],
        alt_mana: None,
        total_mana: 3,
        colors: &[Color::Blue],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((-13, 0)),
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Frantic Strength — {2}{G} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: frantic_strength(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 3,
        colors: &[Color::Green],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((2, 2)),
        expected_set_pt: None,
        expected_kws: &[KeywordSet::TRAMPLE],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Short Circuit — {1}{U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: short_circuit(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Blue, 1)],
        alt_mana: None,
        total_mana: 2,
        colors: &[Color::Blue],
        host_kind: AuraHostKind::ArtifactOrCreature,
        expected_pt_delta: Some((-3, 0)),
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Gilt-Leaf's Embrace — {2}{G} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: gilt_leaf_s_embrace(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 3,
        colors: &[Color::Green],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((2, 0)),
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Annex — {2}{U}{U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: annex(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Blue, 2)],
        alt_mana: None,
        total_mana: 4,
        colors: &[Color::Blue],
        host_kind: AuraHostKind::Land,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: true,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Jeskai Runemark — {2}{U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: jeskai_runemark(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Blue, 1)],
        alt_mana: None,
        total_mana: 3,
        colors: &[Color::Blue],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((2, 2)),
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Pin to the Earth — {1}{U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: pin_to_the_earth(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Blue, 1)],
        alt_mana: None,
        total_mana: 2,
        colors: &[Color::Blue],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((-6, 0)),
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Almost Perfect — {4}{G}{W} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: almost_perfect(),
        primary_mana: &[
            (ManaColor::Colorless, 4),
            (ManaColor::White, 1),
            (ManaColor::Green, 1),
        ],
        alt_mana: None,
        total_mana: 6,
        colors: &[Color::Green, Color::White],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: None,
        expected_set_pt: Some((9, 10)),
        expected_kws: &[KeywordSet::INDESTRUCTIBLE],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Thirst — {2}{U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: thirst(),
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
    // Claustrophobia — {1}{U}{U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: claustrophobia(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Blue, 2)],
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
    // Knight's Pledge — {1}{W} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: knight_s_pledge(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 2,
        colors: &[Color::White],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((2, 2)),
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Accelerated Evolution — {2}{G} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: accelerated_evolution(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 3,
        colors: &[Color::Green],
        host_kind: AuraHostKind::YourCreature,
        expected_pt_delta: Some((2, 2)),
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Ghoulflesh — {B} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: ghoulflesh(),
        primary_mana: &[(ManaColor::Black, 1)],
        alt_mana: None,
        total_mana: 1,
        colors: &[Color::Black],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((-1, -1)),
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Triclopean Sight — {1}{W} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: triclopean_sight(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 2,
        colors: &[Color::White],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((1, 1)),
        expected_set_pt: None,
        expected_kws: &[KeywordSet::VIGILANCE],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Mantle of Leadership — {1}{W} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: mantle_of_leadership(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 2,
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
    // Scourgemark — {1}{B} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: scourgemark(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Black, 1)],
        alt_mana: None,
        total_mana: 2,
        colors: &[Color::Black],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((1, 0)),
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 1,
        etb_taps_attached: false,
    });
    // Runes of the Deus — {4}{R/G} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: runes_of_the_deus(),
        primary_mana: &[(ManaColor::Colorless, 4), (ManaColor::Red, 1)],
        alt_mana: Some(&[(ManaColor::Colorless, 4), (ManaColor::Green, 1)]),
        total_mana: 5,
        colors: &[Color::Green, Color::Red],
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
