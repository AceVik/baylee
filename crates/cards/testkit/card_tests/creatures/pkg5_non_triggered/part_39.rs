//! Package 5 non-triggered creature behavioral tests — part 39.

#[allow(clippy::wildcard_imports)]
use super::*;

// Deity of Scars — {B/G}{B/G}{B/G}{B/G}{B/G} — Creature — Spirit Avatar
// Oracle: Trample / This creature enters with two -1/-1 counters on it. / {B/G}, Remove a -1/-1 counter from this creature: Regenerate this creature.
#[test]
fn deity_of_scars_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("bf7c1fa8-0460-4be5-b00d-72c0960c4149"),
        primary_mana: &[(ManaColor::Black, 5)],
        alt_mana: Some(&[(ManaColor::Green, 5)]),
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black, Color::Green],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (7, 7),
        expected_pt: Some((5, 5)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: Some((CounterKind::M1M1, 2)),
        expected_keywords: KeywordSet::TRAMPLE,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Dementia Bat — {4}{B} — Creature — Phyrexian Bat
// Oracle: Flying / {4}{B}, Sacrifice this creature: Target player discards two cards.
#[test]
fn dementia_bat_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("95c76175-e3a6-4bf4-a8da-d1179851efe5"),
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (2, 2),
        expected_pt: Some((2, 2)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FLYING,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Doom Whisperer — {3}{B}{B} — Creature — Nightmare Demon
// Oracle: Flying, trample / Pay 2 life: Surveil 2. (Look at the top two cards of your library, then put any number of them into your graveyard and the rest on top of your library in any order.)
#[test]
fn doom_whisperer_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("4a01db2e-cd43-4b1a-a480-169018f82501"),
        primary_mana: &[(ManaColor::Black, 2), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (6, 6),
        expected_pt: Some((6, 6)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FLYING.union(KeywordSet::TRAMPLE),
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Dowsing Shaman — {4}{G} — Creature — Centaur Shaman
// Oracle: {2}{G}, {T}: Return target enchantment card from your graveyard to your hand.
#[test]
fn dowsing_shaman_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("c9cb34df-75b1-4dda-8f22-daadf59c5332"),
        primary_mana: &[(ManaColor::Green, 1), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Green],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (3, 4),
        expected_pt: Some((3, 4)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Dragon-Style Twins — {3}{R}{R} — Creature — Human Monk
// Oracle: Double strike / Prowess (Whenever you cast a noncreature spell, this creature gets +1/+1 until end of turn.)
#[test]
fn dragon_style_twins_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("d150f456-c33a-4b19-ac73-b1d818d5db8f"),
        primary_mana: &[(ManaColor::Red, 2), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (3, 3),
        expected_pt: Some((3, 3)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::DOUBLE_STRIKE.union(KeywordSet::PROWESS),
        prowess_bonus: 1,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Eron the Relentless — {3}{R}{R} — Legendary Creature — Human Rogue
// Oracle: Haste / {R}{R}{R}: Regenerate Eron.
#[test]
fn eron_the_relentless_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("cabcc2b7-0cdb-4ec2-b6a3-47644335abff"),
        primary_mana: &[(ManaColor::Red, 2), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: true,
        is_snow: false,
        base_pt: (5, 2),
        expected_pt: Some((5, 2)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::HASTE,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Faerie Formation — {4}{U} — Creature — Faerie
// Oracle: Flying / {3}{U}: Create a 1/1 blue Faerie creature token with flying. Draw a card.
#[test]
fn faerie_formation_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("0366c2f6-6e78-4526-808c-fa7bace6006e"),
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (5, 4),
        expected_pt: Some((5, 4)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FLYING,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Furnace Strider — {4}{R} — Creature — Phyrexian Beast
// Oracle: This creature enters with two oil counters on it. / Remove an oil counter from this creature: Target creature you control gains haste until end of turn.
#[test]
fn furnace_strider_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("85488433-3c23-4a29-b063-53dba71f26c4"),
        primary_mana: &[(ManaColor::Red, 1), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (4, 5),
        expected_pt: Some((4, 5)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: Some((counters::OIL, 2)),
        expected_keywords: KeywordSet::EMPTY,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Gallows Warden — {4}{W} — Creature — Spirit
// Oracle: Flying / Other Spirit creatures you control get +0/+1.
#[test]
fn gallows_warden_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("06157e34-dbe1-4936-a718-3b1d70726794"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (3, 3),
        expected_pt: Some((3, 3)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FLYING,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: Some(subtypes::creature::SPIRIT),
        lord_pt: Some((0, 1)),
        lord_controlled_by_you: true,
    });
}

// Garrison Sergeant — {3}{R}{W} — Creature — Lizard Soldier
// Oracle: This creature has double strike as long as you control a Gate.
#[test]
fn garrison_sergeant_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("cdf8b698-57d6-4d17-8168-87680060c0d6"),
        primary_mana: &[
            (ManaColor::White, 1),
            (ManaColor::Red, 1),
            (ManaColor::Colorless, 3),
        ],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Red],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (3, 3),
        expected_pt: Some((3, 3)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Glorifier of Dusk — {3}{W}{W} — Creature — Vampire Soldier
// Oracle: Pay 2 life: This creature gains flying until end of turn. / Pay 2 life: This creature gains vigilance until end of turn.
#[test]
fn glorifier_of_dusk_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("6205c485-f46b-40af-8b73-03c668f9ea42"),
        primary_mana: &[(ManaColor::White, 2), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (4, 4),
        expected_pt: Some((4, 4)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Gobbling Ooze — {4}{G} — Creature — Ooze
// Oracle: {G}, Sacrifice another creature: Put a +1/+1 counter on this creature.
#[test]
fn gobbling_ooze_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("765a12fc-9b66-4bb7-88f6-6e7bd2d2f8c7"),
        primary_mana: &[(ManaColor::Green, 1), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Green],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (3, 3),
        expected_pt: Some((3, 3)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Godhead of Awe — {W/U}{W/U}{W/U}{W/U}{W/U} — Creature — Spirit Avatar
// Oracle: Flying / Other creatures have base power and toughness 1/1.
#[test]
fn godhead_of_awe_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("1ed5751a-4b45-4cd2-b19c-ac84508de4f8"),
        primary_mana: &[(ManaColor::White, 5)],
        alt_mana: Some(&[(ManaColor::Blue, 5)]),
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Blue],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (4, 4),
        expected_pt: Some((4, 4)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FLYING,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Golgari Rotwurm — {3}{B}{G} — Creature — Zombie Wurm
// Oracle: {B}, Sacrifice a creature: Target player loses 1 life.
#[test]
fn golgari_rotwurm_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("75df9e6a-a17c-4311-a4e7-ff0d75728337"),
        primary_mana: &[
            (ManaColor::Black, 1),
            (ManaColor::Green, 1),
            (ManaColor::Colorless, 3),
        ],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black, Color::Green],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (5, 4),
        expected_pt: Some((5, 4)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Greater Forgeling — {3}{R}{R} — Creature — Elemental
// Oracle: {1}{R}: This creature gets +3/-3 until end of turn.
#[test]
fn greater_forgeling_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("c0d51f49-383b-40f1-9192-5925db1cdd1b"),
        primary_mana: &[(ManaColor::Red, 2), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (3, 4),
        expected_pt: Some((3, 4)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Gumato — {4}{G} — Creature — Ape Beast
// Oracle: Vigilance (Attacking doesn't cause this creature to tap.) / {T}: Add {G}{G}{G}.
#[test]
fn gumato_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("67114cbb-cbf4-4250-a5e9-e1c4bc5841f6"),
        primary_mana: &[(ManaColor::Green, 1), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Green],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (4, 5),
        expected_pt: Some((4, 5)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::VIGILANCE,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Haunter of Nightveil — {3}{U}{B} — Creature — Spirit
// Oracle: Creatures your opponents control get -1/-0.
#[test]
fn haunter_of_nightveil_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("496c852b-ec62-4320-afbb-e19f534954c9"),
        primary_mana: &[
            (ManaColor::Blue, 1),
            (ManaColor::Black, 1),
            (ManaColor::Colorless, 3),
        ],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue, Color::Black],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (3, 4),
        expected_pt: Some((3, 4)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Heidar, Rimewind Master — {4}{U} — Legendary Creature — Human Wizard
// Oracle: {2}, {T}: Return target permanent to its owner's hand. Activate only if you control four or more snow permanents.
#[test]
fn heidar_rimewind_master_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("58ac4cce-506e-4d6e-a19c-0e875fbfcd81"),
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: true,
        is_snow: false,
        base_pt: (3, 3),
        expected_pt: Some((3, 3)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Horror of the Dim — {4}{B} — Creature — Horror
// Oracle: {U}: This creature gains hexproof until end of turn. (It can't be the target of spells or abilities your opponents control.)
#[test]
fn horror_of_the_dim_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("83e91e0a-2680-4bd3-98eb-7567791e77d5"),
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (3, 4),
        expected_pt: Some((3, 4)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Hurler Cyclops — {3}{R}{R} — Creature — Cyclops
// Oracle: {1}, Sacrifice another creature: This creature deals 1 damage to any target.
#[test]
fn hurler_cyclops_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("fac57730-8fd8-4702-8b35-4ef0ed5fa41a"),
        primary_mana: &[(ManaColor::Red, 2), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (5, 4),
        expected_pt: Some((5, 4)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Icatian Phalanx — {4}{W} — Creature — Human Soldier
// Oracle: Banding (Any creatures with banding, and up to one without, can attack in a band. Bands are blocked as a group. If any creatures with banding you control are blocking or being blocked by a creature, you divide that creature's combat damage, not its controller, among any of the creatures it's being blocked by or is blocking.)
#[test]
fn icatian_phalanx_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("a1160b8d-c4e4-4869-b7a5-c6a826eb1ba4"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (2, 4),
        expected_pt: Some((2, 4)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::BANDING,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Kaysa — {3}{G}{G} — Legendary Creature — Elf Druid
// Oracle: Green creatures you control get +1/+1.
#[test]
fn kaysa_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("0fd18a9b-c112-4e48-8fd7-e53fe4500943"),
        primary_mana: &[(ManaColor::Green, 2), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Green],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: true,
        is_snow: false,
        base_pt: (2, 3),
        expected_pt: None,
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Kjeldoran Skycaptain — {4}{W} — Creature — Human Soldier
// Oracle: Flying; first strike; banding (Any creatures with banding, and up to one without, can attack in a band. Bands are blocked as a group. If any creatures with banding you control are blocking or being blocked by a creature, you divide that creature's combat damage, not its controller, among any of the creatures it's being blocked by or is blocking.)
#[test]
fn kjeldoran_skycaptain_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("7160ebce-ee3f-42ac-a3f7-b55f17f2c49f"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (2, 2),
        expected_pt: Some((2, 2)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FLYING
            .union(KeywordSet::FIRST_STRIKE)
            .union(KeywordSet::BANDING),
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Kozilek's Channeler — {5} — Creature — Eldrazi
// Oracle: {T}: Add {C}{C}.
#[test]
fn kozilek_s_channeler_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("33826f91-7680-4a4c-be39-db6c2c0bd465"),
        primary_mana: &[(ManaColor::Colorless, 5)],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (4, 4),
        expected_pt: Some((4, 4)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Kozilek's Translator — {4}{B} — Creature — Eldrazi Drone
// Oracle: Devoid (This card has no color.) / Pay 1 life: Add {C}. Activate only once each turn. ({C} represents colorless mana.)
#[test]
fn kozilek_s_translator_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("6766c5e4-baa7-4db4-a900-c08fbcd98f5d"),
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (3, 5),
        expected_pt: Some((3, 5)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::DEVOID,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Kranioceros — {4}{R} — Creature — Beast
// Oracle: {1}{W}: This creature gets +0/+3 until end of turn.
#[test]
fn kranioceros_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("c66342bb-bbc1-4aaf-8d99-0af68292081e"),
        primary_mana: &[(ManaColor::Red, 1), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (5, 2),
        expected_pt: Some((5, 2)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Llanowar Behemoth — {3}{G}{G} — Creature — Elemental
// Oracle: Tap an untapped creature you control: This creature gets +1/+1 until end of turn.
#[test]
fn llanowar_behemoth_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("4364bef8-f7ec-487c-9b64-b55c4afb864e"),
        primary_mana: &[(ManaColor::Green, 2), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Green],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (4, 4),
        expected_pt: Some((4, 4)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Loxodon Stalwart — {3}{W}{W} — Creature — Elephant Soldier
// Oracle: Vigilance / {W}: This creature gets +0/+1 until end of turn.
#[test]
fn loxodon_stalwart_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("6b2eb774-08f3-46cb-aeca-2ab82fdd5f03"),
        primary_mana: &[(ManaColor::White, 2), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (3, 3),
        expected_pt: Some((3, 3)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::VIGILANCE,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Lyra Dawnbringer — {3}{W}{W} — Legendary Creature — Angel
// Oracle: Flying / First strike (This creature deals combat damage before creatures without first strike.) / Lifelink (Damage dealt by this creature also causes you to gain that much life.) / Other Angels you control get +1/+1 and have lifelink.
#[test]
fn lyra_dawnbringer_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("592c91fc-6430-4c76-9460-65f047350f67"),
        primary_mana: &[(ManaColor::White, 2), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: true,
        is_snow: false,
        base_pt: (5, 5),
        expected_pt: Some((5, 5)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FLYING
            .union(KeywordSet::FIRST_STRIKE)
            .union(KeywordSet::LIFELINK),
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: Some(subtypes::creature::ANGEL),
        lord_pt: Some((1, 1)),
        lord_controlled_by_you: true,
    });
}

// Magmaw — {3}{R}{R} — Creature — Elemental
// Oracle: {1}, Sacrifice a nonland permanent: This creature deals 1 damage to any target.
#[test]
fn magmaw_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("bc891258-2f22-4248-9972-4f41905744c6"),
        primary_mana: &[(ManaColor::Red, 2), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (4, 4),
        expected_pt: Some((4, 4)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}
