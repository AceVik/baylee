//! Package 5 non-triggered creature behavioral tests — part 13.

#[allow(clippy::wildcard_imports)]
use super::*;

// Marshal of Zhalfir — {W}{U} — Creature — Human Knight
// Oracle: Other Knights you control get +1/+1. / {W}{U}, {T}: Tap another target creature.
#[test]
fn marshal_of_zhalfir_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("9ad01fb2-2ba7-447a-a5ef-e57d46421635"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Blue, 1)],
        alt_mana: None,
        total_mana: 2,
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
        expected_keywords: KeywordSet::EMPTY,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: Some(subtypes::creature::KNIGHT),
        lord_pt: Some((1, 1)),
        lord_controlled_by_you: true,
    });
}

// Masked Blackguard — {1}{B} — Creature — Human Rogue
// Oracle: Flash (You may cast this spell any time you could cast an instant.) / {2}{B}: This creature gets +1/+1 until end of turn.
#[test]
fn masked_blackguard_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("8ccd20c7-b7fc-4ab1-a98b-3bbcc2079339"),
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 2,
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
        expected_keywords: KeywordSet::FLASH,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Master of the Pearl Trident — {U}{U} — Creature — Merfolk
// Oracle: Other Merfolk creatures you control get +1/+1 and have islandwalk. (They can't be blocked as long as defending player controls an Island.)
#[test]
fn master_of_the_pearl_trident_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("9f6ea9fe-eee5-4dca-a34e-c66d3e218716"),
        primary_mana: &[(ManaColor::Blue, 2)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue],
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
        lord_subtype: Some(subtypes::creature::MERFOLK),
        lord_pt: Some((1, 1)),
        lord_controlled_by_you: true,
    });
}

// Merciless Enforcers — {1}{B} — Creature — Human Mercenary Villain
// Oracle: Lifelink / {3}{B}: This creature deals 1 damage to each opponent.
#[test]
fn merciless_enforcers_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("b32a7b97-0883-4c9f-a23d-ad269efd8cc3"),
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 2,
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
        expected_keywords: KeywordSet::LIFELINK,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Merfolk Mesmerist — {1}{U} — Creature — Merfolk Wizard
// Oracle: {U}, {T}: Target player mills two cards.
#[test]
fn merfolk_mesmerist_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("590d524f-278d-4cf4-ae3f-efcced4517b1"),
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue],
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

// Merfolk Mistbinder — {G}{U} — Creature — Merfolk Shaman
// Oracle: Other Merfolk you control get +1/+1.
#[test]
fn merfolk_mistbinder_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("be56dfea-1ece-4f06-b1cc-d7425b5018b4"),
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue, Color::Green],
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
        lord_subtype: Some(subtypes::creature::MERFOLK),
        lord_pt: Some((1, 1)),
        lord_controlled_by_you: true,
    });
}

// Mesa Falcon — {1}{W} — Creature — Bird
// Oracle: Flying / {1}{W}: This creature gets +0/+1 until end of turn.
#[test]
fn mesa_falcon_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("bc4c5f33-0f3d-4815-9974-fb12ac725d08"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White],
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

// Metathran Zombie — {1}{U} — Creature — Metathran Zombie
// Oracle: {B}: Regenerate this creature.
#[test]
fn metathran_zombie_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("c631887c-e157-4a95-8a59-08a44854d07f"),
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 2,
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

// Metropolis Sprite — {1}{U} — Creature — Faerie Rogue
// Oracle: Flying / {U}: This creature gets +1/-1 until end of turn.
#[test]
fn metropolis_sprite_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("924c7c4e-9c4b-496a-ba49-7fb3918a8d07"),
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue],
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

// Might Weaver — {1}{G} — Creature — Human Wizard
// Oracle: {2}: Target red or white creature gains trample until end of turn. (It can deal excess combat damage to the player or planeswalker it's attacking.)
#[test]
fn might_weaver_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("c2cfbc1a-ae73-409c-b73a-5c8625365905"),
        primary_mana: &[(ManaColor::Green, 1), (ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Green],
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

// Mire Boa — {1}{G} — Creature — Snake
// Oracle: Swampwalk (This creature can't be blocked as long as defending player controls a Swamp.) / {G}: Regenerate this creature. (The next time this creature would be destroyed this turn, instead tap it, remove it from combat, and heal all damage on it.)
#[test]
fn mire_boa_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("e9a3a466-df78-49a0-add3-975b94c5cd09"),
        primary_mana: &[(ManaColor::Green, 1), (ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Green],
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

// Mistmeadow Skulk — {1}{W} — Creature — Kithkin Rogue
// Oracle: Lifelink, protection from mana value 3 or greater
#[test]
fn mistmeadow_skulk_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("490df3b5-ece2-45bb-af38-fcd7f83e37b0"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White],
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
        expected_keywords: KeywordSet::LIFELINK,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Moorland Inquisitor — {1}{W} — Creature — Human Soldier
// Oracle: {2}{W}: This creature gains first strike until end of turn. (It deals combat damage before creatures without first strike.)
#[test]
fn moorland_inquisitor_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("76bbd280-dcbc-43b3-a8a1-603443b2ccaa"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 2,
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

// Mystic Archaeologist — {1}{U} — Creature — Human Wizard
// Oracle: {3}{U}{U}: Draw two cards.
#[test]
fn mystic_archaeologist_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("b77dfe2a-ebc9-46b0-9134-2ecb2abdd8be"),
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue],
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

// Nectar Faerie — {1}{B} — Creature — Faerie Wizard
// Oracle: Flying / {B}, {T}: Target Faerie or Elf gains lifelink until end of turn. (Damage dealt by the creature also causes its controller to gain that much life.)
#[test]
fn nectar_faerie_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("6d0a2e5f-af10-4754-81d7-b7afa6282d7d"),
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 2,
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

// Nightshade Dryad — {1}{G} — Creature — Dryad
// Oracle: Deathtouch / {T}: Add {C}. / {T}: Add one mana of any color.
#[test]
fn nightshade_dryad_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("f8263274-8cb4-4eab-b541-c7655b555830"),
        primary_mana: &[(ManaColor::Green, 1), (ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Green],
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

// Nimble-Blade Khenra — {1}{R} — Creature — Jackal Warrior
// Oracle: Prowess (Whenever you cast a noncreature spell, this creature gets +1/+1 until end of turn.)
#[test]
fn nimble_blade_khenra_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("09c68fb5-2000-4bb8-b03b-94b093e50bca"),
        primary_mana: &[(ManaColor::Red, 1), (ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red],
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
        expected_keywords: KeywordSet::PROWESS,
        prowess_bonus: 1,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Nimbus Swimmer — {X}{G}{U} — Creature — Leviathan
// Oracle: Flying / This creature enters with X +1/+1 counters on it.
#[test]
fn nimbus_swimmer_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("ce02ef24-d53e-4025-bc45-ca27aba89fe9"),
        primary_mana: &[
            (ManaColor::Blue, 1),
            (ManaColor::Green, 1),
            (ManaColor::Colorless, 2),
        ],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: true,
        colors: &[Color::Blue, Color::Green],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (0, 0),
        expected_pt: Some((2, 2)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: Some((CounterKind::P1P1, 2)),
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

// Noose Constrictor — {1}{G} — Creature — Snake
// Oracle: Reach / Discard a card: This creature gets +1/+1 until end of turn.
#[test]
fn noose_constrictor_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("fb1d521f-cb64-4b87-b3f4-a74e91a60349"),
        primary_mana: &[(ManaColor::Green, 1), (ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 2,
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

// Noxious Newt — {1}{G} — Creature — Salamander
// Oracle: Deathtouch / {T}: Add {G}.
#[test]
fn noxious_newt_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("1232c461-643c-40a3-901e-09b665f817da"),
        primary_mana: &[(ManaColor::Green, 1), (ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Green],
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

// Olivia's Dragoon — {1}{B} — Creature — Vampire Berserker
// Oracle: Discard a card: This creature gains flying until end of turn.
#[test]
fn olivia_s_dragoon_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("a5019399-91a8-4233-b16f-399718c4be9c"),
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 2,
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

// Ondu War Cleric — {1}{W} — Creature — Human Cleric Ally
// Oracle: Cohort — {T}, Tap an untapped Ally you control: You gain 2 life.
#[test]
fn ondu_war_cleric_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("d011153e-d12a-4eb1-b105-64729e37be12"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 2,
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

// Onyx Mage — {1}{B} — Creature — Human Wizard
// Oracle: {1}{B}: Target creature you control gains deathtouch until end of turn. (Any amount of damage it deals to a creature is enough to destroy it.)
#[test]
fn onyx_mage_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("8a398777-2fa5-4cac-adee-4582e59d410d"),
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 2,
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

// Oran-Rief Invoker — {1}{G} — Creature — Human Shaman
// Oracle: {8}: This creature gets +5/+5 and gains trample until end of turn.
#[test]
fn oran_rief_invoker_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("c5dcab94-f4c0-4e5e-ad99-7b3871be6e89"),
        primary_mana: &[(ManaColor::Green, 1), (ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 2,
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

// Orcish Healer — {R}{R} — Creature — Orc Cleric
// Oracle: {R}{R}, {T}: Target creature can't be regenerated this turn. / {B}{B}{R}, {T}: Regenerate target black or green creature. / {R}{G}{G}, {T}: Regenerate target black or green creature.
#[test]
fn orcish_healer_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("9b7a0c9d-1045-4fc0-b5e3-18616106da08"),
        primary_mana: &[(ManaColor::Red, 2)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red],
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

// Orcish Vandal — {1}{R} — Creature — Orc Warrior
// Oracle: {T}, Sacrifice an artifact: This creature deals 2 damage to any target.
#[test]
fn orcish_vandal_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("933a1811-7e1a-4173-b1c9-34b2c858ba68"),
        primary_mana: &[(ManaColor::Red, 1), (ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red],
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

// Order of Leitbur — {W}{W} — Creature — Human Cleric Knight
// Oracle: Protection from black / {W}: This creature gains first strike until end of turn. / {W}{W}: This creature gets +1/+0 until end of turn.
#[test]
fn order_of_leitbur_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("d8f5279f-c58e-4560-9c68-77d226a3c86c"),
        primary_mana: &[(ManaColor::White, 2)],
        alt_mana: None,
        total_mana: 2,
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

// Order of the Ebon Hand — {B}{B} — Creature — Cleric Knight
// Oracle: Protection from white / {B}: This creature gains first strike until end of turn. / {B}{B}: This creature gets +1/+0 until end of turn.
#[test]
fn order_of_the_ebon_hand_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("2f65f16f-c6c2-48f0-b556-63cf7d162000"),
        primary_mana: &[(ManaColor::Black, 2)],
        alt_mana: None,
        total_mana: 2,
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

// Order of the White Shield — {W}{W} — Creature — Human Knight
// Oracle: Protection from black / {W}: This creature gains first strike until end of turn. / {W}{W}: This creature gets +1/+0 until end of turn.
#[test]
fn order_of_the_white_shield_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("e0b3cfc1-68d4-46a8-9ae4-71bdb9c87889"),
        primary_mana: &[(ManaColor::White, 2)],
        alt_mana: None,
        total_mana: 2,
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

// Orzhov Guildmage — {W/B}{W/B} — Creature — Human Wizard
// Oracle: {2}{W}: Target player gains 1 life. / {2}{B}: Each player loses 1 life.
#[test]
fn orzhov_guildmage_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("877bfa3f-6e30-4cb3-831e-d56849b01334"),
        primary_mana: &[(ManaColor::White, 2)],
        alt_mana: Some(&[(ManaColor::Black, 2)]),
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Black],
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
