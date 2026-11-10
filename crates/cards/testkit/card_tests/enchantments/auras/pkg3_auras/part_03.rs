//! Package 3 Aura behavioral tests — part 03 (24 cards).

#[allow(clippy::wildcard_imports)]
use super::*;

fn dark_privilege() -> CardIndex {
    // oracle_id = "29f4c69a-ca59-4e9f-a808-87ba85b34f2f"
    card_index("29f4c69a-ca59-4e9f-a808-87ba85b34f2f")
}

fn bequeathal() -> CardIndex {
    // oracle_id = "2a9054ce-7276-4030-901a-cf6aa18f2b62"
    card_index("2a9054ce-7276-4030-901a-cf6aa18f2b62")
}

fn squirrel_nest() -> CardIndex {
    // oracle_id = "2d584333-b25e-4291-a2c9-80c6d9f8732a"
    card_index("2d584333-b25e-4291-a2c9-80c6d9f8732a")
}

fn capture_sphere() -> CardIndex {
    // oracle_id = "2dc59566-10cb-468a-b6a2-17d6c0186e6d"
    card_index("2dc59566-10cb-468a-b6a2-17d6c0186e6d")
}

fn twisted_embrace() -> CardIndex {
    // oracle_id = "2e29efee-1eed-4bef-8574-0f6ff99d3cda"
    card_index("2e29efee-1eed-4bef-8574-0f6ff99d3cda")
}

fn sea_legs() -> CardIndex {
    // oracle_id = "2e66dc2f-44f3-44f1-8ed7-7f991b3718be"
    card_index("2e66dc2f-44f3-44f1-8ed7-7f991b3718be")
}

fn fists_of_ironwood() -> CardIndex {
    // oracle_id = "2f6a5278-23bf-47b2-a2b9-dfff7d96f3dd"
    card_index("2f6a5278-23bf-47b2-a2b9-dfff7d96f3dd")
}

fn rousing_read() -> CardIndex {
    // oracle_id = "306549ad-84c7-4983-840b-9663799e9f40"
    card_index("306549ad-84c7-4983-840b-9663799e9f40")
}

fn runner_s_bane() -> CardIndex {
    // oracle_id = "31c30c4f-12f4-4cbc-b7c1-123f36b8df75"
    card_index("31c30c4f-12f4-4cbc-b7c1-123f36b8df75")
}

fn ancestral_vengeance() -> CardIndex {
    // oracle_id = "321c4586-d27e-45ae-ba1f-3efea334a697"
    card_index("321c4586-d27e-45ae-ba1f-3efea334a697")
}

fn insolence() -> CardIndex {
    // oracle_id = "33a6d78a-a3c7-44f1-8ce9-d6632123fec1"
    card_index("33a6d78a-a3c7-44f1-8ce9-d6632123fec1")
}

fn sleep_magic() -> CardIndex {
    // oracle_id = "33d6f4f0-e1ad-4363-a012-46a1bd9daeb7"
    card_index("33d6f4f0-e1ad-4363-a012-46a1bd9daeb7")
}

fn gift_of_the_deity() -> CardIndex {
    // oracle_id = "3548489b-053e-4f81-a0be-07422cbe357d"
    card_index("3548489b-053e-4f81-a0be-07422cbe357d")
}

fn caribou_range() -> CardIndex {
    // oracle_id = "39229fbc-1296-4ac7-9f79-65c6f0d09991"
    card_index("39229fbc-1296-4ac7-9f79-65c6f0d09991")
}

fn trollhide() -> CardIndex {
    // oracle_id = "397df813-c759-4246-8def-122c7ff82d88"
    card_index("397df813-c759-4246-8def-122c7ff82d88")
}

fn underworld_connections() -> CardIndex {
    // oracle_id = "39c10798-9dfc-47a2-9da3-b4038509aebe"
    card_index("39c10798-9dfc-47a2-9da3-b4038509aebe")
}

fn coerced_to_kill() -> CardIndex {
    // oracle_id = "3a007dbd-18fd-44d1-a305-7cc8dd6231eb"
    card_index("3a007dbd-18fd-44d1-a305-7cc8dd6231eb")
}

fn bloodshed_fever() -> CardIndex {
    // oracle_id = "3a018a54-6094-42bb-af6a-ef436c09f0b4"
    card_index("3a018a54-6094-42bb-af6a-ef436c09f0b4")
}

fn guard_duty() -> CardIndex {
    // oracle_id = "3b20e187-c200-40a0-8db2-481f0bf90168"
    card_index("3b20e187-c200-40a0-8db2-481f0bf90168")
}

fn unquestioned_authority() -> CardIndex {
    // oracle_id = "3d50266f-5a2f-4a48-ab15-87e733624fd4"
    card_index("3d50266f-5a2f-4a48-ab15-87e733624fd4")
}

fn primal_visitation() -> CardIndex {
    // oracle_id = "3d95e8b1-0ddb-4bd3-b4cc-de55b6f8c403"
    card_index("3d95e8b1-0ddb-4bd3-b4cc-de55b6f8c403")
}

fn riot_spikes() -> CardIndex {
    // oracle_id = "3df676fe-c4f8-41c0-9743-781090bdc547"
    card_index("3df676fe-c4f8-41c0-9743-781090bdc547")
}

fn setessan_training() -> CardIndex {
    // oracle_id = "3ff3b9c4-80ec-44d9-99ec-6fea3a86468b"
    card_index("3ff3b9c4-80ec-44d9-99ec-6fea3a86468b")
}

fn arcane_flight() -> CardIndex {
    // oracle_id = "40eee955-bec6-455a-addf-68cdb84c76b9"
    card_index("40eee955-bec6-455a-addf-68cdb84c76b9")
}

#[test]
#[allow(clippy::too_many_lines)]
fn auras_pkg3_part_03() {
    // Dark Privilege — {1}{B} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: dark_privilege(),
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
    // Bequeathal — {G} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: bequeathal(),
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
    // Squirrel Nest — {1}{G}{G} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: squirrel_nest(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Green, 2)],
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
    // Capture Sphere — {3}{U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: capture_sphere(),
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
    // Twisted Embrace — {2}{B}{B} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: twisted_embrace(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Black, 2)],
        alt_mana: None,
        total_mana: 4,
        colors: &[Color::Black],
        host_kind: AuraHostKind::YourArtifactOrCreature,
        expected_pt_delta: Some((1, 1)),
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Sea Legs — {U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: sea_legs(),
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
    // Fists of Ironwood — {1}{G} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: fists_of_ironwood(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 2,
        colors: &[Color::Green],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[KeywordSet::TRAMPLE],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Rousing Read — {2}{U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: rousing_read(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::Blue, 1)],
        alt_mana: None,
        total_mana: 3,
        colors: &[Color::Blue],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((1, 1)),
        expected_set_pt: None,
        expected_kws: &[KeywordSet::FLYING],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Runner's Bane — {1}{U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: runner_s_bane(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Blue, 1)],
        alt_mana: None,
        total_mana: 2,
        colors: &[Color::Blue],
        host_kind: AuraHostKind::CreaturePower3OrLess,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: true,
        etb_draws: 0,
        etb_taps_attached: true,
    });
    // Ancestral Vengeance — {B}{B} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: ancestral_vengeance(),
        primary_mana: &[(ManaColor::Black, 2)],
        alt_mana: None,
        total_mana: 2,
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
    // Insolence — {2}{R} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: insolence(),
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
    // Sleep Magic — {U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: sleep_magic(),
        primary_mana: &[(ManaColor::Blue, 1)],
        alt_mana: None,
        total_mana: 1,
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
    // Gift of the Deity — {4}{B/G} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: gift_of_the_deity(),
        primary_mana: &[(ManaColor::Colorless, 4), (ManaColor::Black, 1)],
        alt_mana: Some(&[(ManaColor::Colorless, 4), (ManaColor::Green, 1)]),
        total_mana: 5,
        colors: &[Color::Black, Color::Green],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Caribou Range — {2}{W}{W} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: caribou_range(),
        primary_mana: &[(ManaColor::Colorless, 2), (ManaColor::White, 2)],
        alt_mana: None,
        total_mana: 4,
        colors: &[Color::White],
        host_kind: AuraHostKind::YourLand,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Trollhide — {2}{G} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: trollhide(),
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
    // Underworld Connections — {1}{B}{B} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: underworld_connections(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Black, 2)],
        alt_mana: None,
        total_mana: 3,
        colors: &[Color::Black],
        host_kind: AuraHostKind::Land,
        expected_pt_delta: None,
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Coerced to Kill — {3}{U}{B} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: coerced_to_kill(),
        primary_mana: &[
            (ManaColor::Colorless, 3),
            (ManaColor::Blue, 1),
            (ManaColor::Black, 1),
        ],
        alt_mana: None,
        total_mana: 5,
        colors: &[Color::Black, Color::Blue],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: None,
        expected_set_pt: Some((1, 1)),
        expected_kws: &[KeywordSet::DEATHTOUCH],
        gain_control: true,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Bloodshed Fever — {R} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: bloodshed_fever(),
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
    // Guard Duty — {W} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: guard_duty(),
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
    // Unquestioned Authority — {2}{W} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: unquestioned_authority(),
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
        etb_draws: 1,
        etb_taps_attached: false,
    });
    // Primal Visitation — {3}{R}{G} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: primal_visitation(),
        primary_mana: &[
            (ManaColor::Colorless, 3),
            (ManaColor::Red, 1),
            (ManaColor::Green, 1),
        ],
        alt_mana: None,
        total_mana: 5,
        colors: &[Color::Green, Color::Red],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((3, 3)),
        expected_set_pt: None,
        expected_kws: &[KeywordSet::HASTE],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Riot Spikes — {B/R} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: riot_spikes(),
        primary_mana: &[(ManaColor::Black, 1)],
        alt_mana: Some(&[(ManaColor::Red, 1)]),
        total_mana: 1,
        colors: &[Color::Black, Color::Red],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((2, -1)),
        expected_set_pt: None,
        expected_kws: &[],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
    // Setessan Training — {1}{G} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: setessan_training(),
        primary_mana: &[(ManaColor::Colorless, 1), (ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 2,
        colors: &[Color::Green],
        host_kind: AuraHostKind::YourCreature,
        expected_pt_delta: Some((1, 0)),
        expected_set_pt: None,
        expected_kws: &[KeywordSet::TRAMPLE],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 1,
        etb_taps_attached: false,
    });
    // Arcane Flight — {U} — Enchantment — Aura
    verify_aura(&AuraSpec {
        card: arcane_flight(),
        primary_mana: &[(ManaColor::Blue, 1)],
        alt_mana: None,
        total_mana: 1,
        colors: &[Color::Blue],
        host_kind: AuraHostKind::Creature,
        expected_pt_delta: Some((1, 1)),
        expected_set_pt: None,
        expected_kws: &[KeywordSet::FLYING],
        gain_control: false,
        does_not_untap: false,
        etb_draws: 0,
        etb_taps_attached: false,
    });
}
