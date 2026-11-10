//! Package 3 Aura behavioral tests — part 06 (24 cards).

#[allow(clippy::wildcard_imports)]
use super::*;

fn goblin_war_paint() -> CardIndex {
    // oracle_id = "67ca79d7-9064-4605-8625-b1cfe5cb1b45"
    card_index("67ca79d7-9064-4605-8625-b1cfe5cb1b45")
}

fn convincing_mirage() -> CardIndex {
    // oracle_id = "685c4665-e04c-4012-aa5b-888a042c2a21"
    card_index("685c4665-e04c-4012-aa5b-888a042c2a21")
}

fn reach_for_the_sky() -> CardIndex {
    // oracle_id = "68930fb6-d833-40cf-8917-1a1bf60edbe3"
    card_index("68930fb6-d833-40cf-8917-1a1bf60edbe3")
}

fn angelic_reward() -> CardIndex {
    // oracle_id = "699380f9-a032-4838-baf1-1575f37eb642"
    card_index("699380f9-a032-4838-baf1-1575f37eb642")
}

fn prophetic_ravings() -> CardIndex {
    // oracle_id = "69af7bff-fb8c-4f59-88a1-6bfa87ba400a"
    card_index("69af7bff-fb8c-4f59-88a1-6bfa87ba400a")
}

fn stoneskin() -> CardIndex {
    // oracle_id = "6d1d53e3-60ee-40a1-89a5-0648760cd83f"
    card_index("6d1d53e3-60ee-40a1-89a5-0648760cd83f")
}

fn clout_of_the_dominus() -> CardIndex {
    // oracle_id = "6e412163-a93a-4738-a8f0-a4d27c960360"
    card_index("6e412163-a93a-4738-a8f0-a4d27c960360")
}

fn soul_channeling() -> CardIndex {
    // oracle_id = "7041b3d5-c61a-4b94-811c-c5eb9f1ba664"
    card_index("7041b3d5-c61a-4b94-811c-c5eb9f1ba664")
}

fn candlelight_vigil() -> CardIndex {
    // oracle_id = "7153bf15-53f8-445e-acbb-35b4f4b3d551"
    card_index("7153bf15-53f8-445e-acbb-35b4f4b3d551")
}

fn tightening_coils() -> CardIndex {
    // oracle_id = "71af7e54-e22d-4354-a35c-80e4e5f4031c"
    card_index("71af7e54-e22d-4354-a35c-80e4e5f4031c")
}

fn galvanic_arc() -> CardIndex {
    // oracle_id = "72a8620e-1ba2-40a8-9b22-11cc57864adf"
    card_index("72a8620e-1ba2-40a8-9b22-11cc57864adf")
}

fn mantle_of_webs() -> CardIndex {
    // oracle_id = "7549206e-22a9-4d56-b3bc-f6f2f5e87e8a"
    card_index("7549206e-22a9-4d56-b3bc-f6f2f5e87e8a")
}

fn wolfwillow_haven() -> CardIndex {
    // oracle_id = "7579f57d-c76e-4703-a030-34fb7160cb23"
    card_index("7579f57d-c76e-4703-a030-34fb7160cb23")
}

fn iroas_s_blessing() -> CardIndex {
    // oracle_id = "76905904-b3e4-44ee-bfcb-e518f0815af5"
    card_index("76905904-b3e4-44ee-bfcb-e518f0815af5")
}

fn flowstone_blade() -> CardIndex {
    // oracle_id = "77a77622-dc5b-4ff2-a98b-732d44c8856e"
    card_index("77a77622-dc5b-4ff2-a98b-732d44c8856e")
}

fn dying_wail() -> CardIndex {
    // oracle_id = "78a9cb8d-52bb-4182-ad7d-e4f8f0bb9797"
    card_index("78a9cb8d-52bb-4182-ad7d-e4f8f0bb9797")
}

fn gift_of_strands() -> CardIndex {
    // oracle_id = "7a7789e5-f2ac-49b2-943e-7d49e4727fb6"
    card_index("7a7789e5-f2ac-49b2-943e-7d49e4727fb6")
}

fn volcanic_strength() -> CardIndex {
    // oracle_id = "7c60b7bd-06cc-487f-ba42-b382ac429b12"
    card_index("7c60b7bd-06cc-487f-ba42-b382ac429b12")
}

fn cave_sense() -> CardIndex {
    // oracle_id = "7d2fcdac-c8d5-43ca-81bc-130744ed8d3d"
    card_index("7d2fcdac-c8d5-43ca-81bc-130744ed8d3d")
}

fn aspect_of_lamprey() -> CardIndex {
    // oracle_id = "7d3c25bb-5175-4fb1-992f-62a44722f535"
    card_index("7d3c25bb-5175-4fb1-992f-62a44722f535")
}

fn controlled_instincts() -> CardIndex {
    // oracle_id = "7d56d2e2-e83b-4505-af8e-1fd207e161af"
    card_index("7d56d2e2-e83b-4505-af8e-1fd207e161af")
}

fn reduce_in_stature() -> CardIndex {
    // oracle_id = "7ebaeef0-4e4c-4bf0-85dd-07c6b6be0741"
    card_index("7ebaeef0-4e4c-4bf0-85dd-07c6b6be0741")
}

fn holy_mantle() -> CardIndex {
    // oracle_id = "7ed8f5bc-022b-4b1b-8592-57694bf59a0b"
    card_index("7ed8f5bc-022b-4b1b-8592-57694bf59a0b")
}

fn nature_s_chosen() -> CardIndex {
    // oracle_id = "7f425406-2746-45a1-86bf-f9645015ffc2"
    card_index("7f425406-2746-45a1-86bf-f9645015ffc2")
}

#[test]
#[allow(clippy::too_many_lines)]
fn auras_pkg3_part_06() {
    // Goblin War Paint — {1}{R} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: goblin_war_paint(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Red, 1)],
        alt_mana: None,
        total_mana: 2,
        colors: &[Color::Red],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((2, 2)),
        expected_set_pt: None,
        expected_kws: &[KeywordSet::HASTE],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Convincing Mirage — {1}{U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: convincing_mirage(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Blue, 1)],
        alt_mana: None,
        total_mana: 2,
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
    // Reach for the Sky — {3}{G} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: reach_for_the_sky(),
        primary_mana: &[(ManaColor::Colorless, 3), (ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 4,
        colors: &[Color::Green],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((3, 2)),
        expected_set_pt: None,
        expected_kws: &[KeywordSet::REACH],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Angelic Reward — {3}{W}{W} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: angelic_reward(),
        primary_mana: &[(ManaColor::Colorless, 3), (ManaColor::White, 2)],
        alt_mana: None,
        total_mana: 5,
        colors: &[Color::White],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((3, 3)),
        expected_set_pt: None,
        expected_kws: &[KeywordSet::FLYING],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Prophetic Ravings — {R} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: prophetic_ravings(),
        primary_mana: &[(ManaColor::Red, 1)],
        alt_mana: None,
        total_mana: 1,
        colors: &[Color::Red],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[KeywordSet::HASTE],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Stoneskin — {2}{W} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: stoneskin(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 3,
        colors: &[Color::White],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((0, 10)),
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Clout of the Dominus — {U/R} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: clout_of_the_dominus(),
        primary_mana: &[(ManaColor::Blue, 1)],
        alt_mana: Some(&[(ManaColor::Red, 1)]),
        total_mana: 1,
        colors: &[Color::Red, Color::Blue],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Soul Channeling — {2}{B} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: soul_channeling(),
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
    // Candlelight Vigil — {3}{W} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: candlelight_vigil(),
        primary_mana: &[(ManaColor::Colorless, 3), (ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 4,
        colors: &[Color::White],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((3, 2)),
        expected_set_pt: None,
        expected_kws: &[KeywordSet::VIGILANCE],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Tightening Coils — {1}{U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: tightening_coils(),
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
    // Galvanic Arc — {2}{R} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: galvanic_arc(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Red, 1)],
        alt_mana: None,
        total_mana: 3,
        colors: &[Color::Red],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[KeywordSet::FIRST_STRIKE],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Mantle of Webs — {1}{G} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: mantle_of_webs(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 2,
        colors: &[Color::Green],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((1, 3)),
        expected_set_pt: None,
        expected_kws: &[KeywordSet::REACH],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Wolfwillow Haven — {1}{G} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: wolfwillow_haven(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 2,
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
    // Iroas's Blessing — {3}{R} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: iroas_s_blessing(),
        primary_mana: &[(ManaColor::Colorless, 3), (ManaColor::Red, 1)],
        alt_mana: None,
        total_mana: 4,
        colors: &[Color::Red],
        host_kind: AuraHostKind::YourCreature,
        expected_pt_delta: Some((1, 1)),
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Flowstone Blade — {R} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: flowstone_blade(),
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
    // Dying Wail — {1}{B} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: dying_wail(),
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
    // Gift of Strands — {3}{G} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: gift_of_strands(),
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
    // Volcanic Strength — {1}{R} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: volcanic_strength(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Red, 1)],
        alt_mana: None,
        total_mana: 2,
        colors: &[Color::Red],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((2, 2)),
        expected_set_pt: None,
        expected_kws: &[KeywordSet::MOUNTAINWALK],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Cave Sense — {1}{R} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: cave_sense(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Red, 1)],
        alt_mana: None,
        total_mana: 2,
        colors: &[Color::Red],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((1, 1)),
        expected_set_pt: None,
        expected_kws: &[KeywordSet::MOUNTAINWALK],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Aspect of Lamprey — {3}{B} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: aspect_of_lamprey(),
        primary_mana: &[(ManaColor::Colorless, 3), (ManaColor::Black, 1)],
        alt_mana: None,
        total_mana: 4,
        colors: &[Color::Black],
        host_kind: AuraHostKind::YourCreature,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[KeywordSet::LIFELINK],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Controlled Instincts — {U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: controlled_instincts(),
        primary_mana: &[(ManaColor::Blue, 1)],
        alt_mana: None,
        total_mana: 1,
        colors: &[Color::Blue],
        host_kind: AuraHostKind::RedOrGreenCreature,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: true,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Reduce in Stature — {2}{U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: reduce_in_stature(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Blue, 1)],
        alt_mana: None,
        total_mana: 3,
        colors: &[Color::Blue],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: None,
        expected_set_pt: Some((0, 2)),
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Holy Mantle — {2}{W}{W} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: holy_mantle(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::White, 2)],
        alt_mana: None,
        total_mana: 4,
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
    // Nature's Chosen — {G} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: nature_s_chosen(),
        primary_mana: &[(ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 1,
        colors: &[Color::Green],
        host_kind: AuraHostKind::YourCreature,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
}
