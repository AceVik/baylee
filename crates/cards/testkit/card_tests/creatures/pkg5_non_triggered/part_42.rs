//! Package 5 non-triggered creature behavioral tests — part 42.

#[allow(clippy::wildcard_imports)]
use super::*;

// Uktabi Wildcats — {4}{G} — Creature — Cat
// Oracle: Uktabi Wildcats's power and toughness are each equal to the number of Forests you control. / {G}, Sacrifice a Forest: Regenerate this creature.
#[test]
fn uktabi_wildcats_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("dd25afe0-a69a-4e0b-b364-b88d3a522ed3"),
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
        base_pt: (0, 0),
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

// Undercity Shade — {4}{B} — Creature — Shade
// Oracle: Fear (This creature can't be blocked except by artifact creatures and/or black creatures.) / {B}: This creature gets +1/+1 until end of turn.
#[test]
fn undercity_shade_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("28f3d535-9503-4d35-aeed-d3e59259f760"),
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
        base_pt: (1, 1),
        expected_pt: Some((1, 1)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FEAR,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Vampire Warlord — {4}{B} — Creature — Vampire Warrior
// Oracle: Sacrifice another creature: Regenerate this creature. (The next time this creature would be destroyed this turn, instead tap it, remove it from combat, and heal all damage on it.)
#[test]
fn vampire_warlord_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("393586d3-1f1a-4f3a-96b8-ed1a8fdf00c6"),
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
        base_pt: (4, 2),
        expected_pt: Some((4, 2)),
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

// Viscid Lemures — {4}{B} — Creature — Spirit
// Oracle: {0}: This creature gets -1/-0 and gains swampwalk until end of turn. (It can't be blocked as long as defending player controls a Swamp.)
#[test]
fn viscid_lemures_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("738d8d1c-4ceb-4115-89e1-1f2149fb4f5f"),
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
        base_pt: (4, 3),
        expected_pt: Some((4, 3)),
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

// Vona, Butcher of Magan — {3}{W}{B} — Legendary Creature — Vampire Knight
// Oracle: Vigilance, lifelink / {T}, Pay 7 life: Destroy target nonland permanent. Activate only during your turn.
#[test]
fn vona_butcher_of_magan_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("b1c56e23-eae6-47a1-a1cd-3a2d01621768"),
        primary_mana: &[
            (ManaColor::White, 1),
            (ManaColor::Black, 1),
            (ManaColor::Colorless, 3),
        ],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Black],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: true,
        is_snow: false,
        base_pt: (4, 4),
        expected_pt: Some((4, 4)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::VIGILANCE.union(KeywordSet::LIFELINK),
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Watcher in the Web — {4}{G} — Creature — Spider
// Oracle: Reach (This creature can block creatures with flying.) / This creature can block an additional seven creatures each combat.
#[test]
fn watcher_in_the_web_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("232a4128-133e-446d-9f1c-dd3875094473"),
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
        base_pt: (2, 5),
        expected_pt: Some((2, 5)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::REACH,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Whimwader — {4}{U} — Creature — Elemental
// Oracle: This creature can't attack unless defending player controls a blue permanent.
#[test]
fn whimwader_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("21d9ce2c-eb6a-4f43-a79b-0b99b3dc4a00"),
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
        base_pt: (6, 4),
        expected_pt: Some((6, 4)),
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

// Whirlwind Adept — {4}{U} — Creature — Djinn Monk
// Oracle: Hexproof (This creature can't be the target of spells or abilities your opponents control.) / Prowess (Whenever you cast a noncreature spell, this creature gets +1/+1 until end of turn.)
#[test]
fn whirlwind_adept_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("0c7e261f-45d5-4c86-8cba-9b6edce8ef49"),
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
        base_pt: (4, 2),
        expected_pt: Some((4, 2)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::HEXPROOF.union(KeywordSet::PROWESS),
        prowess_bonus: 1,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Woodborn Behemoth — {3}{G}{G} — Creature — Elemental
// Oracle: As long as you control eight or more lands, this creature gets +4/+4 and has trample. (It can deal excess combat damage to the player or planeswalker it's attacking.)
#[test]
fn woodborn_behemoth_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("9bc458ac-5566-4978-aeaa-73b600d4ec29"),
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

// Wormhole Serpent — {4}{U} — Creature — Serpent
// Oracle: {3}{U}: Target creature can't be blocked this turn.
#[test]
fn wormhole_serpent_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("6eb21aca-116e-427b-9e75-8adaee0cc62a"),
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
        base_pt: (3, 5),
        expected_pt: Some((3, 5)),
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

// Wormwood Treefolk — {3}{G}{G} — Creature — Treefolk
// Oracle: {G}{G}: This creature gains forestwalk until end of turn and deals 2 damage to you. (It can't be blocked as long as defending player controls a Forest.) / {B}{B}: This creature gains swampwalk until end of turn and deals 2 damage to you. (It can't be blocked as long as defending player controls a Swamp.)
#[test]
fn wormwood_treefolk_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("2a014ef1-0450-4485-8a8c-8210a985be4c"),
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

// Yavimaya Ancients — {3}{G}{G} — Creature — Treefolk
// Oracle: {G}: This creature gets +1/-2 until end of turn.
#[test]
fn yavimaya_ancients_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("0e91a873-a680-4ca2-b073-26aa39376d77"),
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
        base_pt: (2, 7),
        expected_pt: Some((2, 7)),
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

// Aboshan, Cephalid Emperor — {4}{U}{U} — Legendary Creature — Octopus Noble
// Oracle: Tap an untapped Octopus you control: Tap target permanent. / {U}{U}{U}: Tap all creatures without flying.
#[test]
fn aboshan_cephalid_emperor_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("d4d44264-b276-4bd7-8fbe-6af292b08773"),
        primary_mana: &[(ManaColor::Blue, 2), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 6,
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

// Ancient Silverback — {4}{G}{G} — Creature — Ape
// Oracle: {G}: Regenerate this creature. (The next time this creature would be destroyed this turn, instead tap it, remove it from combat, and heal all damage on it.)
#[test]
fn ancient_silverback_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("5f12134c-d626-4a5e-8653-700681a491fc"),
        primary_mana: &[(ManaColor::Green, 2), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 6,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Green],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (6, 5),
        expected_pt: Some((6, 5)),
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

// Angel of the God-Pharaoh — {4}{W}{W} — Creature — Angel
// Oracle: Flying / Cycling {2} ({2}, Discard this card: Draw a card.)
#[test]
fn angel_of_the_god_pharaoh_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("701912d5-9995-4ac7-9858-188f78b2b8a3"),
        primary_mana: &[(ManaColor::White, 2), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 6,
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
        expected_keywords: KeywordSet::FLYING,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: Some(&[(ManaColor::Colorless, 2)]),
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Ascendant Evincar — {4}{B}{B} — Legendary Creature — Phyrexian Vampire Noble
// Oracle: Flying (This creature can't be blocked except by creatures with flying or reach.) / Other black creatures get +1/+1. / Nonblack creatures get -1/-1.
#[test]
fn ascendant_evincar_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("e32c0de7-132d-4a39-85fb-da0ed1132878"),
        primary_mana: &[(ManaColor::Black, 2), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 6,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: true,
        is_snow: false,
        base_pt: (3, 3),
        expected_pt: None,
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

// Azorius Skyguard — {4}{W}{U} — Creature — Human Knight
// Oracle: Flying, first strike / Creatures your opponents control get -1/-0.
#[test]
fn azorius_skyguard_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("bbfdb5eb-2140-4d50-a667-1b4ac75d03a5"),
        primary_mana: &[
            (ManaColor::White, 1),
            (ManaColor::Blue, 1),
            (ManaColor::Colorless, 4),
        ],
        alt_mana: None,
        total_mana: 6,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Blue],
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
        expected_keywords: KeywordSet::FLYING.union(KeywordSet::FIRST_STRIKE),
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Battering Sliver — {5}{R} — Creature — Sliver
// Oracle: All Sliver creatures have trample.
#[test]
fn battering_sliver_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("4adf959b-2e09-47ff-a1b1-b889348735b8"),
        primary_mana: &[(ManaColor::Red, 1), (ManaColor::Colorless, 5)],
        alt_mana: None,
        total_mana: 6,
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

// Cabal Patriarch — {3}{B}{B}{B} — Legendary Creature — Human Wizard
// Oracle: {2}{B}, Sacrifice a creature: Target creature gets -2/-2 until end of turn. / {2}{B}, Exile a creature card from your graveyard: Target creature gets -2/-2 until end of turn.
#[test]
fn cabal_patriarch_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("70d1caee-1c35-4fa3-bbc3-a7dcc226be90"),
        primary_mana: &[(ManaColor::Black, 3), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 6,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black],
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

// Carnassid — {4}{G}{G} — Creature — Beast
// Oracle: Trample / {1}{G}: Regenerate this creature.
#[test]
fn carnassid_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("97b386da-4bc0-4c39-a5cc-17e441c45127"),
        primary_mana: &[(ManaColor::Green, 2), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 6,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Green],
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

// Carnivorous Moss-Beast — {4}{G}{G} — Creature — Plant Elemental Beast
// Oracle: {5}{G}{G}: Put a +1/+1 counter on this creature.
#[test]
fn carnivorous_moss_beast_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("32f5f67b-bba3-427c-ab24-13a50acdc611"),
        primary_mana: &[(ManaColor::Green, 2), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 6,
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

// Cateran Slaver — {4}{B}{B} — Creature — Horror Mercenary
// Oracle: Swampwalk (This creature can't be blocked as long as defending player controls a Swamp.) / {5}, {T}: Search your library for a Mercenary permanent card with mana value 5 or less, put it onto the battlefield, then shuffle.
#[test]
fn cateran_slaver_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("e7989ef3-d2a5-41c6-944c-f969d850f557"),
        primary_mana: &[(ManaColor::Black, 2), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 6,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (5, 5),
        expected_pt: Some((5, 5)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::SWAMPWALK,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Cavern Thoctar — {5}{G} — Creature — Beast
// Oracle: {1}{R}: This creature gets +1/+0 until end of turn.
#[test]
fn cavern_thoctar_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("4a1b8db2-8814-45b3-ab8a-a1c8522d7fa1"),
        primary_mana: &[(ManaColor::Green, 1), (ManaColor::Colorless, 5)],
        alt_mana: None,
        total_mana: 6,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Green],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (5, 5),
        expected_pt: Some((5, 5)),
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

// Centaur Veteran — {5}{G} — Creature — Centaur
// Oracle: Trample / {G}, Discard a card: Regenerate this creature.
#[test]
fn centaur_veteran_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("bd4c9726-c7bb-4da4-99e5-e98d174d6a52"),
        primary_mana: &[(ManaColor::Green, 1), (ManaColor::Colorless, 5)],
        alt_mana: None,
        total_mana: 6,
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

// Cloud Djinn — {5}{U} — Creature — Djinn
// Oracle: Flying / This creature can block only creatures with flying.
#[test]
fn cloud_djinn_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("4914d38d-ed6b-4531-b72c-06f0968c44e9"),
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Colorless, 5)],
        alt_mana: None,
        total_mana: 6,
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

// Cloud Dragon — {5}{U} — Creature — Illusion Dragon
// Oracle: Flying / This creature can block only creatures with flying.
#[test]
fn cloud_dragon_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("3c46f309-69ae-43b1-adf6-bdb26599c1f4"),
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Colorless, 5)],
        alt_mana: None,
        total_mana: 6,
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

// Corrupted Harvester — {4}{B}{B} — Creature — Phyrexian Horror
// Oracle: {B}, Sacrifice a creature: Regenerate this creature.
#[test]
fn corrupted_harvester_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("f105ccdd-65bc-4880-bbfe-f57fc0e060ea"),
        primary_mana: &[(ManaColor::Black, 2), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 6,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (6, 3),
        expected_pt: Some((6, 3)),
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

// Deathless Angel — {4}{W}{W} — Creature — Angel
// Oracle: Flying / {W}{W}: Target creature gains indestructible until end of turn.
#[test]
fn deathless_angel_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("6098ba4b-ebfe-48da-a33d-b94c021ac559"),
        primary_mana: &[(ManaColor::White, 2), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 6,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (5, 7),
        expected_pt: Some((5, 7)),
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

// Deep-Sea Serpent — {4}{U}{U} — Creature — Serpent
// Oracle: This creature can't attack unless defending player controls an Island.
#[test]
fn deep_sea_serpent_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("7fbb98cc-585c-4184-97f5-9b3d3ebdb1e5"),
        primary_mana: &[(ManaColor::Blue, 2), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 6,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (5, 5),
        expected_pt: Some((5, 5)),
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

// Desert Cerodon — {5}{R} — Creature — Beast
// Oracle: Cycling {R} ({R}, Discard this card: Draw a card.)
#[test]
fn desert_cerodon_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("6ad18589-ece4-4809-80f6-6764d2822372"),
        primary_mana: &[(ManaColor::Red, 1), (ManaColor::Colorless, 5)],
        alt_mana: None,
        total_mana: 6,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (6, 4),
        expected_pt: Some((6, 4)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: Some(&[(ManaColor::Red, 1)]),
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}
