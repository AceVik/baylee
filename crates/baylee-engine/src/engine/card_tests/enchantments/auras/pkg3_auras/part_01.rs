//! Package 3 Aura behavioral tests — part 01 (24 cards).

#[allow(clippy::wildcard_imports)]
use super::*;

fn sensory_deprivation() -> CardIndex {
    // oracle_id = "0006faf6-7a61-426c-9034-579f2cfcfa83"
    card_index("0006faf6-7a61-426c-9034-579f2cfcfa83")
}

fn waterknot() -> CardIndex {
    // oracle_id = "002100cd-1ab1-44da-93a0-1269d47a712a"
    card_index("002100cd-1ab1-44da-93a0-1269d47a712a")
}

fn ensoul_artifact() -> CardIndex {
    // oracle_id = "02628058-c9d8-4265-afd1-3fb697ec89f7"
    card_index("02628058-c9d8-4265-afd1-3fb697ec89f7")
}

fn sangrite_backlash() -> CardIndex {
    // oracle_id = "03220765-8a9d-4d4a-88d5-98df975ce13c"
    card_index("03220765-8a9d-4d4a-88d5-98df975ce13c")
}

fn bonds_of_quicksilver() -> CardIndex {
    // oracle_id = "03ce87f1-cdda-4121-8c0a-988642053055"
    card_index("03ce87f1-cdda-4121-8c0a-988642053055")
}

fn freed_from_the_real() -> CardIndex {
    // oracle_id = "03d32ace-dd99-48c6-bcae-9b915d041aef"
    card_index("03d32ace-dd99-48c6-bcae-9b915d041aef")
}

fn colossification() -> CardIndex {
    // oracle_id = "05c5ee65-651b-4bfb-b57f-305026507135"
    card_index("05c5ee65-651b-4bfb-b57f-305026507135")
}

fn sky_tether() -> CardIndex {
    // oracle_id = "06548af1-4fda-435c-ac32-bbf7afafb34e"
    card_index("06548af1-4fda-435c-ac32-bbf7afafb34e")
}

fn ephara_s_radiance() -> CardIndex {
    // oracle_id = "0870e9f8-9e02-4997-9554-41a4e8eec4cb"
    card_index("0870e9f8-9e02-4997-9554-41a4e8eec4cb")
}

fn chime_of_night() -> CardIndex {
    // oracle_id = "09b41783-0583-4486-99e8-3f8a11ae37fd"
    card_index("09b41783-0583-4486-99e8-3f8a11ae37fd")
}

fn river_s_favor() -> CardIndex {
    // oracle_id = "0a576352-ed10-4a54-b886-d664432277c2"
    card_index("0a576352-ed10-4a54-b886-d664432277c2")
}

fn abzan_runemark() -> CardIndex {
    // oracle_id = "0a9276fc-387c-4bde-a6d6-b0dd7abee94b"
    card_index("0a9276fc-387c-4bde-a6d6-b0dd7abee94b")
}

fn glaring_aegis() -> CardIndex {
    // oracle_id = "0b0c98ba-3505-40aa-b0f2-e03c9790f51e"
    card_index("0b0c98ba-3505-40aa-b0f2-e03c9790f51e")
}

fn lust_for_war() -> CardIndex {
    // oracle_id = "0ba206f3-c3f3-4e53-b072-e5e70feaa00d"
    card_index("0ba206f3-c3f3-4e53-b072-e5e70feaa00d")
}

fn corrupted_roots() -> CardIndex {
    // oracle_id = "0c6f343a-0088-477a-a6b5-256783ad0c27"
    card_index("0c6f343a-0088-477a-a6b5-256783ad0c27")
}

fn dragon_mantle() -> CardIndex {
    // oracle_id = "0cc5aaa1-07bf-4872-ac72-303cef9fb143"
    card_index("0cc5aaa1-07bf-4872-ac72-303cef9fb143")
}

fn inferno_fist() -> CardIndex {
    // oracle_id = "0ccc75a0-1bc2-461f-ab6f-0f182328a863"
    card_index("0ccc75a0-1bc2-461f-ab6f-0f182328a863")
}

fn lunarch_mantle() -> CardIndex {
    // oracle_id = "0da104ae-c46c-4106-919f-d975721d42d9"
    card_index("0da104ae-c46c-4106-919f-d975721d42d9")
}

fn roots() -> CardIndex {
    // oracle_id = "0dcee3a3-6cb1-45ce-958c-1f57ced80be4"
    card_index("0dcee3a3-6cb1-45ce-958c-1f57ced80be4")
}

fn savage_hunger() -> CardIndex {
    // oracle_id = "0e608f48-d3a6-422c-839d-f06bb8a32014"
    card_index("0e608f48-d3a6-422c-839d-f06bb8a32014")
}

fn unflinching_courage() -> CardIndex {
    // oracle_id = "0e969a27-1609-4ab4-b0db-46b1af8066a9"
    card_index("0e969a27-1609-4ab4-b0db-46b1af8066a9")
}

fn nimbus_wings() -> CardIndex {
    // oracle_id = "0f35e73b-6a38-4185-a7d3-d237d67ba1cd"
    card_index("0f35e73b-6a38-4185-a7d3-d237d67ba1cd")
}

fn mire_s_grasp() -> CardIndex {
    // oracle_id = "1009d2a2-a145-46e1-ba14-7a42735062fc"
    card_index("1009d2a2-a145-46e1-ba14-7a42735062fc")
}

fn deviant_glee() -> CardIndex {
    // oracle_id = "13aeccd5-cca2-4249-904d-d5516f977440"
    card_index("13aeccd5-cca2-4249-904d-d5516f977440")
}

#[test]
#[allow(clippy::too_many_lines)]
fn auras_pkg3_part_01() {
    // Sensory Deprivation — {U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: sensory_deprivation(),
        primary_mana: &[(ManaColor::Blue, 1)],
        alt_mana: None,
        total_mana: 1,
        colors: &[Color::Blue],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((-3, 0)),
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Waterknot — {1}{U}{U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: waterknot(),
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
    // Ensoul Artifact — {1}{U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: ensoul_artifact(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Blue, 1)],
        alt_mana: None,
        total_mana: 2,
        colors: &[Color::Blue],
        host_kind: AuraHostKind::Artifact,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Sangrite Backlash — {B/G}{R} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: sangrite_backlash(),
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::Red, 1)],
        alt_mana: Some(&[(ManaColor::Red, 1), (ManaColor::Green, 1)]),
        total_mana: 2,
        colors: &[Color::Black, Color::Green, Color::Red],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((3, -3)),
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Bonds of Quicksilver — {3}{U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: bonds_of_quicksilver(),
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
        etb_taps_attached: false,
    });
    // Freed from the Real — {2}{U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: freed_from_the_real(),
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
    // Colossification — {5}{G}{G} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: colossification(),
        primary_mana: &[(ManaColor::Colorless, 5), (ManaColor::Green, 2)],
        alt_mana: None,
        total_mana: 7,
        colors: &[Color::Green],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((20, 20)),
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: true,
    });
    // Sky Tether — {W} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: sky_tether(),
        primary_mana: &[(ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 1,
        colors: &[Color::White],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[KeywordSet::DEFENDER],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Ephara's Radiance — {W} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: ephara_s_radiance(),
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
    // Chime of Night — {1}{B} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: chime_of_night(),
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
    // River's Favor — {U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: river_s_favor(),
        primary_mana: &[(ManaColor::Blue, 1)],
        alt_mana: None,
        total_mana: 1,
        colors: &[Color::Blue],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((1, 1)),
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Abzan Runemark — {2}{W} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: abzan_runemark(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 3,
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
    // Glaring Aegis — {W} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: glaring_aegis(),
        primary_mana: &[(ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 1,
        colors: &[Color::White],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((1, 3)),
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Lust for War — {2}{R} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: lust_for_war(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Red, 1)],
        alt_mana: None,
        total_mana: 3,
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
    // Corrupted Roots — {B} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: corrupted_roots(),
        primary_mana: &[(ManaColor::Black, 1)],
        alt_mana: None,
        total_mana: 1,
        colors: &[Color::Black],
        host_kind: AuraHostKind::ForestOrPlains,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Dragon Mantle — {R} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: dragon_mantle(),
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
        etb_draws: 1,
        etb_taps_attached: false,
    });
    // Inferno Fist — {1}{R} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: inferno_fist(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Red, 1)],
        alt_mana: None,
        total_mana: 2,
        colors: &[Color::Red],
        host_kind: AuraHostKind::YourCreature,
        expected_pt_delta: Some((2, 0)),
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Lunarch Mantle — {1}{W} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: lunarch_mantle(),
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
    // Roots — {3}{G} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: roots(),
        primary_mana: &[(ManaColor::Colorless, 3), (ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 4,
        colors: &[Color::Green],
        host_kind: AuraHostKind::CreatureWithoutFlying,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: true,
        etb_draws: 0,
        etb_taps_attached: true,
    });
    // Savage Hunger — {2}{G} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: savage_hunger(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 3,
        colors: &[Color::Green],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((1, 0)),
        expected_set_pt: None,
        expected_kws: &[KeywordSet::TRAMPLE],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Unflinching Courage — {1}{G}{W} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: unflinching_courage(),
        primary_mana: &[
            (ManaColor::Colorless, 1),
            (ManaColor::White, 1),
            (ManaColor::Green, 1),
        ],
        alt_mana: None,
        total_mana: 3,
        colors: &[Color::Green, Color::White],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((2, 2)),
        expected_set_pt: None,
        expected_kws: &[KeywordSet::TRAMPLE.union(KeywordSet::LIFELINK)],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Nimbus Wings — {1}{W} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: nimbus_wings(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 2,
        colors: &[Color::White],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((1, 2)),
        expected_set_pt: None,
        expected_kws: &[KeywordSet::FLYING],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Mire's Grasp — {1}{B} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: mire_s_grasp(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Black, 1)],
        alt_mana: None,
        total_mana: 2,
        colors: &[Color::Black],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((-3, -3)),
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Deviant Glee — {B} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: deviant_glee(),
        primary_mana: &[(ManaColor::Black, 1)],
        alt_mana: None,
        total_mana: 1,
        colors: &[Color::Black],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((2, 1)),
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
}
