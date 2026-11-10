//! Package 3 Aura behavioral tests — part 07 (24 cards).

#[allow(clippy::wildcard_imports)]
use super::*;

fn ice_over() -> CardIndex {
    // oracle_id = "80f8a64e-b22e-413e-a143-f1943be3a574"
    card_index("80f8a64e-b22e-413e-a143-f1943be3a574")
}

fn thrull_retainer() -> CardIndex {
    // oracle_id = "81a47019-1343-4a63-893a-0d8ba5e848d5"
    card_index("81a47019-1343-4a63-893a-0d8ba5e848d5")
}

fn spirit_mantle() -> CardIndex {
    // oracle_id = "81a51328-b995-4f3b-90bc-a20ae21dd254"
    card_index("81a51328-b995-4f3b-90bc-a20ae21dd254")
}

fn multani_s_harmony() -> CardIndex {
    // oracle_id = "81c223a5-5ff2-43e5-bd0b-25c4f72b2150"
    card_index("81c223a5-5ff2-43e5-bd0b-25c4f72b2150")
}

fn epiphany_storm() -> CardIndex {
    // oracle_id = "833fe5f4-cfab-46ce-a4fe-250673896350"
    card_index("833fe5f4-cfab-46ce-a4fe-250673896350")
}

fn wurmweaver_coil() -> CardIndex {
    // oracle_id = "83419229-c266-4022-8297-d1b05ec5ee20"
    card_index("83419229-c266-4022-8297-d1b05ec5ee20")
}

fn robotics_mastery() -> CardIndex {
    // oracle_id = "855318cb-9382-4183-87c7-93bdfac86431"
    card_index("855318cb-9382-4183-87c7-93bdfac86431")
}

fn emblem_of_the_warmind() -> CardIndex {
    // oracle_id = "86e88219-0f8e-405d-8439-c3b4acf11330"
    card_index("86e88219-0f8e-405d-8439-c3b4acf11330")
}

fn slimebind() -> CardIndex {
    // oracle_id = "87aa2cbe-841c-40ff-9b1c-b259b4cc631e"
    card_index("87aa2cbe-841c-40ff-9b1c-b259b4cc631e")
}

fn super_speed() -> CardIndex {
    // oracle_id = "88923ca1-a793-42f0-b9f8-ed9ff9c1185d"
    card_index("88923ca1-a793-42f0-b9f8-ed9ff9c1185d")
}

fn strands_of_undeath() -> CardIndex {
    // oracle_id = "897c2c66-fedc-4201-b187-db79414583fb"
    card_index("897c2c66-fedc-4201-b187-db79414583fb")
}

fn aspect_of_manticore() -> CardIndex {
    // oracle_id = "899f3758-bb06-4f9c-81eb-7a9002f03245"
    card_index("899f3758-bb06-4f9c-81eb-7a9002f03245")
}

fn fire_rim_form() -> CardIndex {
    // oracle_id = "89fb21dc-4cf2-4c9a-b0ae-cc6e10277fb6"
    card_index("89fb21dc-4cf2-4c9a-b0ae-cc6e10277fb6")
}

fn sinking_feeling() -> CardIndex {
    // oracle_id = "8a5e925f-320d-4bb7-9374-0384e7ac41e7"
    card_index("8a5e925f-320d-4bb7-9374-0384e7ac41e7")
}

fn starlit_mantle() -> CardIndex {
    // oracle_id = "8a7643e4-b824-441a-bbe6-349fc7a3e11e"
    card_index("8a7643e4-b824-441a-bbe6-349fc7a3e11e")
}

fn glimmerdust_nap() -> CardIndex {
    // oracle_id = "8b476e35-77df-4332-b29d-8bf6f1bbbc3e"
    card_index("8b476e35-77df-4332-b29d-8bf6f1bbbc3e")
}

fn stonehands() -> CardIndex {
    // oracle_id = "8beccaff-d30f-4164-a264-cb7119cee4f1"
    card_index("8beccaff-d30f-4164-a264-cb7119cee4f1")
}

fn equestrian_skill() -> CardIndex {
    // oracle_id = "8c8c12ff-24ff-4279-9bde-da8899d2c2d5"
    card_index("8c8c12ff-24ff-4279-9bde-da8899d2c2d5")
}

fn keen_sense() -> CardIndex {
    // oracle_id = "8d47b78b-11cc-4351-91d6-c891eb63dd98"
    card_index("8d47b78b-11cc-4351-91d6-c891eb63dd98")
}

fn soul_kiss() -> CardIndex {
    // oracle_id = "8ea941aa-ee0c-4830-912a-0c672546aa0c"
    card_index("8ea941aa-ee0c-4830-912a-0c672546aa0c")
}

fn earthlore() -> CardIndex {
    // oracle_id = "91fbffd1-0886-45fa-bf3e-4c9864458431"
    card_index("91fbffd1-0886-45fa-bf3e-4c9864458431")
}

fn one_with_the_wind() -> CardIndex {
    // oracle_id = "9203bee0-718a-4ede-9556-5521417c4080"
    card_index("9203bee0-718a-4ede-9556-5521417c4080")
}

fn blessing_of_leeches() -> CardIndex {
    // oracle_id = "923b725a-e189-4b61-b01a-c5056f1328ba"
    card_index("923b725a-e189-4b61-b01a-c5056f1328ba")
}

fn gremlin_infestation() -> CardIndex {
    // oracle_id = "94fa21f1-826a-4d51-9ccd-d72e5cf13f3d"
    card_index("94fa21f1-826a-4d51-9ccd-d72e5cf13f3d")
}

#[test]
#[allow(clippy::too_many_lines)]
fn auras_pkg3_part_07() {
    // Ice Over — {1}{U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: ice_over(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Blue, 1)],
        alt_mana: None,
        total_mana: 2,
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
    // Thrull Retainer — {B} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: thrull_retainer(),
        primary_mana: &[(ManaColor::Black, 1)],
        alt_mana: None,
        total_mana: 1,
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
    // Spirit Mantle — {1}{W} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: spirit_mantle(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 2,
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
    // Multani's Harmony — {G} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: multani_s_harmony(),
        primary_mana: &[(ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 1,
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
    // Epiphany Storm — {R} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: epiphany_storm(),
        primary_mana: &[(ManaColor::Red, 1)],
        alt_mana: None,
        total_mana: 1,
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
    // Wurmweaver Coil — {4}{G}{G} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: wurmweaver_coil(),
        primary_mana: &[(ManaColor::Colorless, 4), (ManaColor::Green, 2)],
        alt_mana: None,
        total_mana: 6,
        colors: &[Color::Green],
        host_kind: AuraHostKind::GreenCreature,
        expected_pt_delta: Some((6, 6)),
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Robotics Mastery — {4}{U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: robotics_mastery(),
        primary_mana: &[(ManaColor::Colorless, 4), (ManaColor::Blue, 1)],
        alt_mana: None,
        total_mana: 5,
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
    // Emblem of the Warmind — {1}{R} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: emblem_of_the_warmind(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Red, 1)],
        alt_mana: None,
        total_mana: 2,
        colors: &[Color::Red],
        host_kind: AuraHostKind::YourCreature,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Slimebind — {1}{U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: slimebind(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Blue, 1)],
        alt_mana: None,
        total_mana: 2,
        colors: &[Color::Blue],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((-4, 0)),
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Super Speed — {R} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: super_speed(),
        primary_mana: &[(ManaColor::Red, 1)],
        alt_mana: None,
        total_mana: 1,
        colors: &[Color::Red],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((1, 0)),
        expected_set_pt: None,
        expected_kws: &[KeywordSet::HASTE],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Strands of Undeath — {3}{B} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: strands_of_undeath(),
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
    // Aspect of Manticore — {2}{R} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: aspect_of_manticore(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Red, 1)],
        alt_mana: None,
        total_mana: 3,
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
    // Fire-Rim Form — {1}{R} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: fire_rim_form(),
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
    // Sinking Feeling — {2}{U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: sinking_feeling(),
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
        etb_taps_attached: false,
    });
    // Starlit Mantle — {1}{U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: starlit_mantle(),
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
    // Glimmerdust Nap — {2}{U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: glimmerdust_nap(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Blue, 1)],
        alt_mana: None,
        total_mana: 3,
        colors: &[Color::Blue],
        host_kind: AuraHostKind::TappedCreature,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: true,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Stonehands — {2}{R} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: stonehands(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Red, 1)],
        alt_mana: None,
        total_mana: 3,
        colors: &[Color::Red],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((0, 2)),
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Equestrian Skill — {3}{G} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: equestrian_skill(),
        primary_mana: &[(ManaColor::Colorless, 3), (ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 4,
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
    // Keen Sense — {G} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: keen_sense(),
        primary_mana: &[(ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 1,
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
    // Soul Kiss — {2}{B} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: soul_kiss(),
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
    // Earthlore — {G} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: earthlore(),
        primary_mana: &[(ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 1,
        colors: &[Color::Green],
        host_kind: AuraHostKind::YourLand,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // One With the Wind — {1}{U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: one_with_the_wind(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Blue, 1)],
        alt_mana: None,
        total_mana: 2,
        colors: &[Color::Blue],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((2, 2)),
        expected_set_pt: None,
        expected_kws: &[KeywordSet::FLYING],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Blessing of Leeches — {2}{B} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: blessing_of_leeches(),
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
    // Gremlin Infestation — {3}{R} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: gremlin_infestation(),
        primary_mana: &[(ManaColor::Colorless, 3), (ManaColor::Red, 1)],
        alt_mana: None,
        total_mana: 4,
        colors: &[Color::Red],
        host_kind: AuraHostKind::Artifact,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
}
