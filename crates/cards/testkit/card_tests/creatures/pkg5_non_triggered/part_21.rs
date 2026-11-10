//! Package 5 non-triggered creature behavioral tests — part 21.

#[allow(clippy::wildcard_imports)]
use super::*;

// Crafty Pathmage — {2}{U} — Creature — Human Wizard
// Oracle: {T}: Target creature with power 2 or less can't be blocked this turn.
#[test]
fn crafty_pathmage_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("c4020178-71d9-4363-a0bd-b9a7eecd5d02"),
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue],
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

// Crusader of Odric — {2}{W} — Creature — Human Soldier
// Oracle: Crusader of Odric's power and toughness are each equal to the number of creatures you control.
#[test]
fn crusader_of_odric_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("ea384b0d-3091-4d50-b15f-1f6763647b7c"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White],
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

// Cunning Sparkmage — {2}{R} — Creature — Human Shaman
// Oracle: Haste / {T}: This creature deals 1 damage to any target.
#[test]
fn cunning_sparkmage_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("ab4a25d8-e1d0-454e-9d06-f7960a0352bc"),
        primary_mana: &[(ManaColor::Red, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (0, 1),
        expected_pt: Some((0, 1)),
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

// D'Avenant Healer — {1}{W}{W} — Creature — Human Cleric Archer
// Oracle: {T}: This creature deals 1 damage to target attacking or blocking creature. / {T}: Prevent the next 1 damage that would be dealt to any target this turn.
#[test]
fn d_avenant_healer_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("91770dc7-3de3-458f-a240-6540455fe05f"),
        primary_mana: &[(ManaColor::White, 2), (ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (1, 2),
        expected_pt: Some((1, 2)),
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

// Daring Mechanic — {2}{W} — Creature — Human Artificer
// Oracle: {3}{W}: Put a +1/+1 counter on target Mount or Vehicle.
#[test]
fn daring_mechanic_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("9b1d79b7-a812-4f48-9be0-789294da3b56"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
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

// Darkwatch Elves — {2}{G} — Creature — Elf
// Oracle: Protection from black / Cycling {2} ({2}, Discard this card: Draw a card.)
#[test]
fn darkwatch_elves_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("55ad9924-f217-4844-8a82-48746186bc53"),
        primary_mana: &[(ManaColor::Green, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Green],
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
        expected_keywords: KeywordSet::EMPTY,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: Some(&[(ManaColor::Colorless, 2)]),
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Dauthi Mercenary — {2}{B} — Creature — Dauthi Knight Mercenary
// Oracle: Shadow (This creature can block or be blocked by only creatures with shadow.) / {1}{B}: This creature gets +1/+0 until end of turn.
#[test]
fn dauthi_mercenary_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("1b57c480-e2c5-44cd-820f-e208041bfd18"),
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (2, 1),
        expected_pt: Some((2, 1)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::SHADOW,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Dauthi Trapper — {2}{B} — Creature — Dauthi Minion
// Oracle: {T}: Target creature gains shadow until end of turn. (It can block or be blocked by only creatures with shadow.)
#[test]
fn dauthi_trapper_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("3aa64915-8b1b-4414-b11e-d2078c1732a5"),
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
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

// Dawnglare Invoker — {2}{W} — Creature — Kor Wizard
// Oracle: Flying / {8}: Tap all creatures target player controls.
#[test]
fn dawnglare_invoker_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("0d603aef-2b92-43fb-81ba-f89953a198df"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (2, 1),
        expected_pt: Some((2, 1)),
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

// Death Baron — {1}{B}{B} — Creature — Zombie Wizard
// Oracle: Skeletons you control and other Zombies you control get +1/+1 and have deathtouch. (Any amount of damage they deal to a creature is enough to destroy it.)
#[test]
fn death_baron_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("99024aa8-5687-4d38-8a4b-feef42d6c1ff"),
        primary_mana: &[(ManaColor::Black, 2), (ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 3,
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

// Deathbloom Gardener — {2}{G} — Creature — Elf Druid
// Oracle: Deathtouch / {T}: Add one mana of any color.
#[test]
fn deathbloom_gardener_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("0c7f9701-c39e-4f14-844a-15f2e3c41ca9"),
        primary_mana: &[(ManaColor::Green, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Green],
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
        expected_keywords: KeywordSet::DEATHTOUCH,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Deathcult Rogue — {1}{U/B}{U/B} — Creature — Human Rogue
// Oracle: This creature can't be blocked except by Rogues.
#[test]
fn deathcult_rogue_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("fa8ae8ac-4ad2-4cda-9f6b-4ec634526658"),
        primary_mana: &[(ManaColor::Blue, 2), (ManaColor::Colorless, 1)],
        alt_mana: Some(&[(ManaColor::Black, 2), (ManaColor::Colorless, 1)]),
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue, Color::Black],
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

// Deepwood Ghoul — {2}{B} — Creature — Zombie
// Oracle: Pay 2 life: Regenerate this creature.
#[test]
fn deepwood_ghoul_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("d99cd2d0-32b8-4edc-8089-557c48dd4be0"),
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (2, 1),
        expected_pt: Some((2, 1)),
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

// Desiccated Naga — {2}{B} — Creature — Zombie Snake
// Oracle: {3}{B}: Target opponent loses 2 life and you gain 2 life. Activate only if you control a Liliana planeswalker.
#[test]
fn desiccated_naga_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("ab172e3e-e71e-4627-9b9a-a9295b891533"),
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (3, 2),
        expected_pt: Some((3, 2)),
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

// Devotee of Strength — {2}{G} — Creature — Snake Wizard
// Oracle: {4}{G}: Target creature gets +2/+2 until end of turn.
#[test]
fn devotee_of_strength_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("a6d8f944-f516-49d7-9aa8-77b98e862d6a"),
        primary_mana: &[(ManaColor::Green, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Green],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (3, 2),
        expected_pt: Some((3, 2)),
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

// Devouring Swarm — {1}{B}{B} — Creature — Insect
// Oracle: Flying / Sacrifice a creature: This creature gets +1/+1 until end of turn.
#[test]
fn devouring_swarm_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("11656b2a-b459-4b1c-b505-8547c3507856"),
        primary_mana: &[(ManaColor::Black, 2), (ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (2, 1),
        expected_pt: Some((2, 1)),
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

// Dire Wolf Prowler — {2}{G} — Creature — Wolf
// Oracle: {1}{G}: This creature gets +2/+2 and gains haste until end of turn. Activate only once each turn.
#[test]
fn dire_wolf_prowler_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("d558b465-75c6-450e-b1f2-5843ad67de71"),
        primary_mana: &[(ManaColor::Green, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Green],
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

// Dire Wolves — {2}{G} — Creature — Wolf
// Oracle: This creature has banding as long as you control a Plains. (Any creatures with banding, and up to one without, can attack in a band. Bands are blocked as a group. If any creatures with banding you control are blocking or being blocked by a creature, you divide that creature's combat damage, not its controller, among any of the creatures it's being blocked by or is blocking.)
#[test]
fn dire_wolves_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("7d72a2f4-cce6-4a2b-9507-0ee1c3e1e4ae"),
        primary_mana: &[(ManaColor::Green, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Green],
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

// Disruptive Student — {2}{U} — Creature — Human Wizard
// Oracle: {T}: Counter target spell unless its controller pays {1}.
#[test]
fn disruptive_student_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("74315869-46e3-43a2-abc0-d86d0a70d343"),
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue],
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

// Doomed Necromancer — {2}{B} — Creature — Human Cleric Mercenary
// Oracle: {B}, {T}, Sacrifice this creature: Return target creature card from your graveyard to the battlefield.
#[test]
fn doomed_necromancer_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("155422a0-a0cd-4399-8ed9-fa68ac2c80a6"),
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
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

// Draconic Disciple — {1}{R}{G} — Creature — Human Shaman
// Oracle: {T}: Add one mana of any color. / {7}, {T}, Sacrifice this creature: Create a 5/5 red Dragon creature token with flying.
#[test]
fn draconic_disciple_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("781cee5f-f9cb-42ed-b408-543d297bf96d"),
        primary_mana: &[
            (ManaColor::Red, 1),
            (ManaColor::Green, 1),
            (ManaColor::Colorless, 1),
        ],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red, Color::Green],
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

// Dragon Bell Monk — {2}{W} — Creature — Human Monk
// Oracle: Vigilance / Prowess (Whenever you cast a noncreature spell, this creature gets +1/+1 until end of turn.)
#[test]
fn dragon_bell_monk_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("83f7e8e5-d485-4571-9fa6-47b5e962d221"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
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
        expected_keywords: KeywordSet::VIGILANCE.union(KeywordSet::PROWESS),
        prowess_bonus: 1,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Dragonsoul Knight — {2}{R} — Creature — Human Knight
// Oracle: First strike / {W}{U}{B}{R}{G}: Until end of turn, this creature becomes a Dragon, gets +5/+3, and gains flying and trample.
#[test]
fn dragonsoul_knight_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("f8fb5a40-4e32-4817-bf56-484add976475"),
        primary_mana: &[(ManaColor::Red, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red],
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
        expected_keywords: KeywordSet::FIRST_STRIKE,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Dread Shade — {B}{B}{B} — Creature — Shade
// Oracle: {B}: This creature gets +1/+1 until end of turn.
#[test]
fn dread_shade_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("9b90c7d1-38c6-4210-bc76-68c5fcb91202"),
        primary_mana: &[(ManaColor::Black, 3)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black],
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

// Dread Warlock — {1}{B}{B} — Creature — Human Wizard Warlock
// Oracle: This creature can't be blocked except by black creatures.
#[test]
fn dread_warlock_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("5791710d-c718-4f64-9c14-3668154716f1"),
        primary_mana: &[(ManaColor::Black, 2), (ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 3,
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

// Drogskol Captain — {1}{W}{U} — Creature — Spirit Soldier
// Oracle: Flying / Other Spirit creatures you control get +1/+1 and have hexproof. (They can't be the targets of spells or abilities your opponents control.)
#[test]
fn drogskol_captain_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("c33604ac-636a-43ad-9efe-8d7b3a28bf81"),
        primary_mana: &[
            (ManaColor::White, 1),
            (ManaColor::Blue, 1),
            (ManaColor::Colorless, 1),
        ],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Blue],
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
        lord_subtype: Some(subtypes::creature::SPIRIT),
        lord_pt: Some((1, 1)),
        lord_controlled_by_you: true,
    });
}

// Drowner of Secrets — {2}{U} — Creature — Merfolk Wizard
// Oracle: Tap an untapped Merfolk you control: Target player mills a card.
#[test]
fn drowner_of_secrets_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("a5ce3fb6-9c70-4433-a079-467c911bfd4d"),
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (1, 3),
        expected_pt: Some((1, 3)),
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

// Druid of the Spade — {2}{G} — Creature — Rabbit Druid
// Oracle: As long as you control a token, this creature gets +2/+0 and has trample.
#[test]
fn druid_of_the_spade_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("9740ce6d-ef6b-4d70-839a-8bd5e0e1e0a1"),
        primary_mana: &[(ManaColor::Green, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Green],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
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

// Dune Diviner — {2}{G} — Creature — Snake Cleric
// Oracle: {1}, Tap an untapped Desert you control: You gain 1 life.
#[test]
fn dune_diviner_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("4bf308fc-a54e-4fba-b0fd-3bedb4db3b18"),
        primary_mana: &[(ManaColor::Green, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Green],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (2, 3),
        expected_pt: Some((2, 3)),
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

// Dungrove Elder — {2}{G} — Creature — Treefolk
// Oracle: Hexproof (This creature can't be the target of spells or abilities your opponents control.) / Dungrove Elder's power and toughness are each equal to the number of Forests you control.
#[test]
fn dungrove_elder_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("32083c9f-04c6-4b6f-9e24-cc8f3cae3e3e"),
        primary_mana: &[(ManaColor::Green, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
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
        expected_keywords: KeywordSet::HEXPROOF,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}
