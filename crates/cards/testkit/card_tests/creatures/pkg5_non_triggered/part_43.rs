//! Package 5 non-triggered creature behavioral tests — part 43.

#[allow(clippy::wildcard_imports)]
use super::*;

// Elite Archers — {5}{W} — Creature — Human Soldier Archer
// Oracle: {T}: This creature deals 3 damage to target attacking or blocking creature.
#[test]
fn elite_archers_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("2230ec18-4dcf-435e-b495-1905cb94e8b8"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Colorless, 5)],
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

// Face of Fear — {5}{B} — Creature — Horror
// Oracle: {2}{B}, Discard a card: This creature gains fear until end of turn. (It can't be blocked except by artifact creatures and/or black creatures.)
#[test]
fn face_of_fear_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("605787fe-64e9-4de8-974a-473277b0ba2e"),
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::Colorless, 5)],
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

// Floodchaser — {5}{U} — Creature — Elemental
// Oracle: This creature enters with six +1/+1 counters on it. / This creature can't attack unless defending player controls an Island. / {U}, Remove a +1/+1 counter from this creature: Target land becomes an Island until end of turn.
#[test]
fn floodchaser_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("1bf97485-191b-466c-9590-3449466b348c"),
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
        base_pt: (0, 0),
        expected_pt: Some((6, 6)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: Some((CounterKind::P1P1, 6)),
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

// Foxfire Oak — {5}{G} — Creature — Treefolk Shaman
// Oracle: {R/G}{R/G}{R/G}: This creature gets +3/+0 until end of turn.
#[test]
fn foxfire_oak_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("925775c6-b6d2-4793-a363-bd5aca871aa8"),
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
        base_pt: (3, 6),
        expected_pt: Some((3, 6)),
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

// Frilled Sea Serpent — {4}{U}{U} — Creature — Serpent
// Oracle: {5}{U}{U}: This creature can't be blocked this turn.
#[test]
fn frilled_sea_serpent_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("6dc534e8-e22b-4e3c-aa30-c6e4c84f698c"),
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
        base_pt: (4, 6),
        expected_pt: Some((4, 6)),
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

// Fury Sliver — {5}{R} — Creature — Sliver
// Oracle: All Sliver creatures have double strike.
#[test]
fn fury_sliver_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("44623693-51d6-49ad-8cd7-140505caf02f"),
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

// Gnathosaur — {4}{R}{R} — Creature — Dinosaur
// Oracle: Sacrifice an artifact: This creature gains trample until end of turn.
#[test]
fn gnathosaur_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("7f52993c-a20a-48fe-b463-2a33873b489a"),
        primary_mana: &[(ManaColor::Red, 2), (ManaColor::Colorless, 4)],
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

// Granitic Titan — {4}{R}{R} — Creature — Elemental
// Oracle: Menace / Cycling {2} ({2}, Discard this card: Draw a card.)
#[test]
fn granitic_titan_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("d398868b-7574-40d5-8915-6393ee489067"),
        primary_mana: &[(ManaColor::Red, 2), (ManaColor::Colorless, 4)],
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
        base_pt: (5, 4),
        expected_pt: Some((5, 4)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::MENACE,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: Some(&[(ManaColor::Colorless, 2)]),
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Hammerfist Giant — {4}{R}{R} — Creature — Giant Warrior
// Oracle: {T}: This creature deals 4 damage to each creature without flying and each player.
#[test]
fn hammerfist_giant_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("ce916c92-350c-4546-882b-555dc450413d"),
        primary_mana: &[(ManaColor::Red, 2), (ManaColor::Colorless, 4)],
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

// Herald of the Sun — {4}{W}{W} — Creature — Angel
// Oracle: Flying / {3}{W}: Put a +1/+1 counter on another target creature with flying.
#[test]
fn herald_of_the_sun_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("8331fcd2-b91d-45eb-8361-74aa73e4ff69"),
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
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Imposing Vantasaur — {5}{W} — Creature — Dinosaur
// Oracle: Vigilance / Cycling {1} ({1}, Discard this card: Draw a card.)
#[test]
fn imposing_vantasaur_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("a0d5fce0-a439-496a-9fcf-9271503f3ce8"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Colorless, 5)],
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
        base_pt: (3, 6),
        expected_pt: Some((3, 6)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::VIGILANCE,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: Some(&[(ManaColor::Colorless, 1)]),
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Kjeldoran Phalanx — {5}{W} — Creature — Human Soldier
// Oracle: First strike; banding (Any creatures with banding, and up to one without, can attack in a band. Bands are blocked as a group. If any creatures with banding you control are blocking or being blocked by a creature, you divide that creature's combat damage, not its controller, among any of the creatures it's being blocked by or is blocking.)
#[test]
fn kjeldoran_phalanx_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("a8563a4f-a23b-4cb5-b6e5-795bf8d74ee6"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Colorless, 5)],
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
        base_pt: (2, 5),
        expected_pt: Some((2, 5)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FIRST_STRIKE.union(KeywordSet::BANDING),
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Lava Serpent — {5}{R} — Creature — Elemental Serpent
// Oracle: Haste / Cycling {2} ({2}, Discard this card: Draw a card.)
#[test]
fn lava_serpent_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("4d94d807-97d9-4cdf-a63c-d8b82c2acc6e"),
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
        base_pt: (5, 5),
        expected_pt: Some((5, 5)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::HASTE,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: Some(&[(ManaColor::Colorless, 2)]),
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Loxodon Mender — {5}{W} — Creature — Elephant Cleric
// Oracle: {W}, {T}: Regenerate target artifact.
#[test]
fn loxodon_mender_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("b260b5b4-048b-4a70-ab3a-b0dd1ceb7aa8"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Colorless, 5)],
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

// Maraxus of Keld — {4}{R}{R} — Legendary Creature — Human Warrior
// Oracle: Maraxus's power and toughness are each equal to the number of untapped artifacts, creatures, and lands you control.
#[test]
fn maraxus_of_keld_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("af4b67c5-53dc-4e4d-9ec4-fa51cc3cd49a"),
        primary_mana: &[(ManaColor::Red, 2), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 6,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: true,
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

// Miner's Bane — {4}{R}{R} — Creature — Elemental
// Oracle: {2}{R}: This creature gets +1/+0 and gains trample until end of turn. (It can deal excess combat damage to the player or planeswalker it's attacking.)
#[test]
fn miner_s_bane_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("3ca75f61-60bf-487f-8cb3-b955cdd81e2a"),
        primary_mana: &[(ManaColor::Red, 2), (ManaColor::Colorless, 4)],
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

// Morselhoarder — {4}{R/G}{R/G} — Creature — Elemental
// Oracle: This creature enters with two -1/-1 counters on it. / Remove a -1/-1 counter from this creature: Add one mana of any color.
#[test]
fn morselhoarder_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("8fe687aa-d134-4a29-9c93-64086c4a1977"),
        primary_mana: &[(ManaColor::Red, 2), (ManaColor::Colorless, 4)],
        alt_mana: Some(&[(ManaColor::Green, 2), (ManaColor::Colorless, 4)]),
        total_mana: 6,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red, Color::Green],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (6, 4),
        expected_pt: Some((4, 2)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: Some((CounterKind::M1M1, 2)),
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

// Pearl Dragon — {4}{W}{W} — Creature — Dragon
// Oracle: Flying / {1}{W}: This creature gets +0/+1 until end of turn.
#[test]
fn pearl_dragon_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("f4e0e2e0-8d64-40a3-92da-3951a696b5f3"),
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
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Rampaging Hippo — {4}{G}{G} — Creature — Hippo
// Oracle: Trample / Cycling {2} ({2}, Discard this card: Draw a card.)
#[test]
fn rampaging_hippo_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("e48aa13b-7488-4644-9080-6c342a8ac463"),
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
        base_pt: (5, 6),
        expected_pt: Some((5, 6)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::TRAMPLE,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: Some(&[(ManaColor::Colorless, 2)]),
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Riverwheel Aerialists — {5}{U} — Creature — Djinn Monk
// Oracle: Flying / Prowess (Whenever you cast a noncreature spell, this creature gets +1/+1 until end of turn.)
#[test]
fn riverwheel_aerialists_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("f9221367-24a5-41a3-8943-c6a3cb5d605d"),
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
        base_pt: (4, 5),
        expected_pt: Some((4, 5)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FLYING.union(KeywordSet::PROWESS),
        prowess_bonus: 1,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Sea Monster — {4}{U}{U} — Creature — Serpent
// Oracle: This creature can't attack unless defending player controls an Island.
#[test]
fn sea_monster_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("21b07ce7-b4f9-438c-8c09-e624557d62d2"),
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
        base_pt: (6, 6),
        expected_pt: Some((6, 6)),
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

// Serra's Guardian — {4}{W}{W} — Creature — Angel
// Oracle: Flying (This creature can't be blocked except by creatures with flying or reach.) / Vigilance (Attacking doesn't cause this creature to tap.) / Other creatures you control have vigilance.
#[test]
fn serra_s_guardian_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("c50cd7e7-8d53-44af-ac96-3e104587bdb3"),
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
        base_pt: (5, 5),
        expected_pt: Some((5, 5)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FLYING.union(KeywordSet::VIGILANCE),
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Shambling Strider — {4}{G}{G} — Creature — Yeti
// Oracle: {R}{G}: This creature gets +1/-1 until end of turn.
#[test]
fn shambling_strider_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("fd65468c-d75a-4be2-8a0c-9c4e808e3dd0"),
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

// Silvos, Rogue Elemental — {3}{G}{G}{G} — Legendary Creature — Elemental
// Oracle: Trample / {G}: Regenerate Silvos.
#[test]
fn silvos_rogue_elemental_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("665b8bbf-c30b-4875-a5c2-fc3b9767ecac"),
        primary_mana: &[(ManaColor::Green, 3), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 6,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Green],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: true,
        is_snow: false,
        base_pt: (8, 5),
        expected_pt: Some((8, 5)),
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

// Skyscythe Engulfer — {5}{G} — Creature — Phyrexian Beast
// Oracle: Reach, trample / This creature can't be blocked by creatures with flying.
#[test]
fn skyscythe_engulfer_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("b7cc831c-c1a1-4ea4-ae78-5be2b6173407"),
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
        base_pt: (6, 5),
        expected_pt: Some((6, 5)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::REACH.union(KeywordSet::TRAMPLE),
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Smog Elemental — {4}{B}{B} — Creature — Elemental
// Oracle: Flying / Creatures with flying your opponents control get -1/-1.
#[test]
fn smog_elemental_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("8e51a07d-8618-4908-8798-bd28bc9b5bbb"),
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

// Sphinx of Magosi — {3}{U}{U}{U} — Creature — Sphinx
// Oracle: Flying / {2}{U}: Draw a card, then put a +1/+1 counter on this creature.
#[test]
fn sphinx_of_magosi_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("1c9cf20c-51e4-4a87-b9f8-231b6baf5d6b"),
        primary_mana: &[(ManaColor::Blue, 3), (ManaColor::Colorless, 3)],
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
        base_pt: (6, 6),
        expected_pt: Some((6, 6)),
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

// Stalking Bloodsucker — {4}{B}{B} — Creature — Vampire
// Oracle: Flying / {1}{B}, Discard a card: This creature gets +2/+2 until end of turn.
#[test]
fn stalking_bloodsucker_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("5ed1f550-b558-4e4b-ae89-9e19df035c81"),
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

// Stonehorn Chanter — {5}{W} — Creature — Rhino Cleric
// Oracle: {5}{W}: This creature gains vigilance and lifelink until end of turn. (Attacking doesn't cause it to tap. Damage dealt by it also causes you to gain that much life.)
#[test]
fn stonehorn_chanter_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("22c5187b-e3db-4221-a173-e3b83a581fca"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Colorless, 5)],
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

// Swampbenders — {4}{G}{G} — Creature — Human Druid Ally
// Oracle: Swampbenders's power and toughness are each equal to the number of Swamps on the battlefield. / Lands you control are Swamps in addition to their other types.
#[test]
fn swampbenders_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("feb00c6d-ff43-49bc-84d5-57e2d4651258"),
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
