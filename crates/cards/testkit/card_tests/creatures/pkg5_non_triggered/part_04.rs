//! Package 5 non-triggered creature behavioral tests — part 04.

#[allow(clippy::wildcard_imports)]
use super::*;

// Jaspera Sentinel — {G} — Creature — Elf Rogue
// Oracle: Reach / {T}, Tap an untapped creature you control: Add one mana of any color.
#[test]
fn jaspera_sentinel_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("e2f98813-4f39-4135-b05b-407e9eb797f6"),
        primary_mana: &[(ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 1,
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

// Judge's Familiar — {W/U} — Creature — Bird
// Oracle: Flying / Sacrifice this creature: Counter target instant or sorcery spell unless its controller pays {1}.
#[test]
fn judge_s_familiar_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("22b7975c-37a5-4895-b209-bdeeec0c6aea"),
        primary_mana: &[(ManaColor::White, 1)],
        alt_mana: Some(&[(ManaColor::Blue, 1)]),
        total_mana: 1,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Blue],
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

// Jungle Delver — {G} — Creature — Merfolk Warrior
// Oracle: {3}{G}: Put a +1/+1 counter on this creature.
#[test]
fn jungle_delver_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("2f299e69-d82b-48c9-b7f5-cf07cd9dced2"),
        primary_mana: &[(ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 1,
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

// Kami of False Hope — {W} — Creature — Spirit
// Oracle: Sacrifice this creature: Prevent all combat damage that would be dealt this turn.
#[test]
fn kami_of_false_hope_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("49984248-4800-4f46-933a-fedd970ae910"),
        primary_mana: &[(ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 1,
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

// Kelsinko Ranger — {W} — Creature — Human Ranger
// Oracle: {1}{W}: Target green creature gains first strike until end of turn.
#[test]
fn kelsinko_ranger_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("4d47e598-690e-4850-97e4-7136bc7b497e"),
        primary_mana: &[(ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 1,
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

// Kjeldoran Warrior — {W} — Creature — Human Warrior
// Oracle: Banding (Any creatures with banding, and up to one without, can attack in a band. Bands are blocked as a group. If any creatures with banding you control are blocking or being blocked by a creature, you divide that creature's combat damage, not its controller, among any of the creatures it's being blocked by or is blocking.)
#[test]
fn kjeldoran_warrior_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("73b70a48-b62c-4fdf-9c2d-c6d6688f9ac8"),
        primary_mana: &[(ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 1,
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

// Krark-Clan Shaman — {R} — Creature — Goblin Shaman
// Oracle: Sacrifice an artifact: This creature deals 1 damage to each creature without flying.
#[test]
fn krark_clan_shaman_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("198d1792-d638-4760-a476-7ec3af495610"),
        primary_mana: &[(ManaColor::Red, 1)],
        alt_mana: None,
        total_mana: 1,
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

// Lavastep Raider — {R} — Creature — Goblin Warrior
// Oracle: {2}{R}: This creature gets +2/+0 until end of turn.
#[test]
fn lavastep_raider_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("3a96782b-e9c8-4bc5-845d-45f6bf7b8dc4"),
        primary_mana: &[(ManaColor::Red, 1)],
        alt_mana: None,
        total_mana: 1,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red],
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

// Law-Rune Enforcer — {W} — Creature — Human Soldier
// Oracle: {1}, {T}: Tap target creature with mana value 2 or greater.
#[test]
fn law_rune_enforcer_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("bd13392e-e72a-4dbb-ba9d-68f68e47e30e"),
        primary_mana: &[(ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 1,
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

// Liberated Dwarf — {R} — Creature — Dwarf
// Oracle: {R}, Sacrifice this creature: Target green creature gets +1/+0 and gains first strike until end of turn.
#[test]
fn liberated_dwarf_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("c5081f2d-ebe2-4ff3-a8ef-fe50f8ecde60"),
        primary_mana: &[(ManaColor::Red, 1)],
        alt_mana: None,
        total_mana: 1,
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

// Lionheart Maverick — {W} — Creature — Human Knight
// Oracle: Vigilance / {4}{W}: This creature gets +1/+2 until end of turn.
#[test]
fn lionheart_maverick_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("5058b9f1-9941-495d-9eed-554ae2f6d500"),
        primary_mana: &[(ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 1,
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

// Llanowar Mentor — {G} — Creature — Elf Spellshaper
// Oracle: {G}, {T}, Discard a card: Create a 1/1 green Elf Druid creature token named Llanowar Elves. It has "{T}: Add {G}."
#[test]
fn llanowar_mentor_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("bc7b6508-6522-4772-93b0-3985e7e75048"),
        primary_mana: &[(ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 1,
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

// Loam Dryad — {G} — Creature — Dryad Horror
// Oracle: {T}, Tap an untapped creature you control: Add one mana of any color.
#[test]
fn loam_dryad_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("1420b7db-859b-4ad5-9844-10413cc70478"),
        primary_mana: &[(ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 1,
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

// Manaforge Cinder — {B/R} — Creature — Elemental Shaman
// Oracle: {1}: Add {B} or {R}. Activate no more than three times each turn.
#[test]
fn manaforge_cinder_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("3c9535a2-7995-42de-a56f-0a445f2dd697"),
        primary_mana: &[(ManaColor::Black, 1)],
        alt_mana: Some(&[(ManaColor::Red, 1)]),
        total_mana: 1,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black, Color::Red],
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

// Mardu Hateblade — {W} — Creature — Human Warrior
// Oracle: {B}: This creature gains deathtouch until end of turn. (Any amount of damage it deals to a creature is enough to destroy it.)
#[test]
fn mardu_hateblade_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("acff2573-cf8a-4aee-96a3-28721a4128f6"),
        primary_mana: &[(ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 1,
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

// Master of Dark Rites — {B} — Creature — Vampire Cleric
// Oracle: {T}, Sacrifice another creature: Add {B}{B}{B}. Spend this mana only to cast Vampire, Cleric, and/or Demon spells.
#[test]
fn master_of_dark_rites_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("a65250f6-55e2-47ed-a355-d690047db6f2"),
        primary_mana: &[(ManaColor::Black, 1)],
        alt_mana: None,
        total_mana: 1,
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

// Mindlash Sliver — {B} — Creature — Sliver
// Oracle: All Slivers have "{1}, Sacrifice this permanent: Each player discards a card."
#[test]
fn mindlash_sliver_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("7a08b371-1353-476d-ae72-379594f801da"),
        primary_mana: &[(ManaColor::Black, 1)],
        alt_mana: None,
        total_mana: 1,
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

// Monastery Swiftspear — {R} — Creature — Human Monk
// Oracle: Haste / Prowess (Whenever you cast a noncreature spell, this creature gets +1/+1 until end of turn.)
#[test]
fn monastery_swiftspear_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("dafd2713-d1bc-474b-b390-d2ff20b5375e"),
        primary_mana: &[(ManaColor::Red, 1)],
        alt_mana: None,
        total_mana: 1,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red],
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
        expected_keywords: KeywordSet::HASTE.union(KeywordSet::PROWESS),
        prowess_bonus: 1,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Moopsy — {G} — Creature — Beast
// Oracle: Deathtouch / {2}: This creature gains reach until end of turn.
#[test]
fn moopsy_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("89b9efc8-d1cd-4b2a-acf3-4f0dd1629167"),
        primary_mana: &[(ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 1,
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

// Mothdust Changeling — {U} — Creature — Shapeshifter
// Oracle: Changeling (This card is every creature type.) / Tap an untapped creature you control: This creature gains flying until end of turn.
#[test]
fn mothdust_changeling_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("52237e6b-19f6-405c-a668-545c4ccb1b6d"),
        primary_mana: &[(ManaColor::Blue, 1)],
        alt_mana: None,
        total_mana: 1,
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
        expected_keywords: KeywordSet::CHANGELING,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Mothrider Patrol — {W} — Creature — Fox Warrior
// Oracle: Flying / {3}{W}, {T}: Tap target creature.
#[test]
fn mothrider_patrol_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("c4073754-7d92-4976-8e7d-d6eb85b91608"),
        primary_mana: &[(ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 1,
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

// Necra Disciple — {B} — Creature — Human Wizard
// Oracle: {G}, {T}: Add one mana of any color. / {W}, {T}: Prevent the next 1 damage that would be dealt to any target this turn.
#[test]
fn necra_disciple_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("ef85a5eb-9cde-47b7-97e4-daa9aa9d4676"),
        primary_mana: &[(ManaColor::Black, 1)],
        alt_mana: None,
        total_mana: 1,
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

// Neverwinter Dryad — {G} — Creature — Dryad
// Oracle: {2}, Sacrifice this creature: Search your library for a basic Forest card, put it onto the battlefield tapped, then shuffle.
#[test]
fn neverwinter_dryad_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("a6e654fb-3567-4b92-ae9b-d064bba7112a"),
        primary_mana: &[(ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 1,
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

// Nova Cleric — {W} — Creature — Human Cleric
// Oracle: {2}{W}, {T}, Sacrifice this creature: Destroy all enchantments.
#[test]
fn nova_cleric_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("d71bc551-3fd9-43a9-9477-2acc8683a784"),
        primary_mana: &[(ManaColor::White, 1)],
        alt_mana: None,
        total_mana: 1,
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

// Oashra Cultivator — {G} — Creature — Human Druid
// Oracle: {2}{G}, {T}, Sacrifice this creature: Search your library for a basic land card, put it onto the battlefield tapped, then shuffle.
#[test]
fn oashra_cultivator_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("c07bbe66-96ae-4fab-82b9-fc6dc4f549c3"),
        primary_mana: &[(ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 1,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Green],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (0, 3),
        expected_pt: Some((0, 3)),
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

// Odious Trow — {B/G} — Creature — Troll
// Oracle: {1}{B/G}: Regenerate this creature.
#[test]
fn odious_trow_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("0e9864e2-1bb4-4527-b2af-62433a90b669"),
        primary_mana: &[(ManaColor::Black, 1)],
        alt_mana: Some(&[(ManaColor::Green, 1)]),
        total_mana: 1,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black, Color::Green],
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

// Pit Imp — {B} — Creature — Imp
// Oracle: Flying / {B}: This creature gets +1/+0 until end of turn. Activate no more than twice each turn.
#[test]
fn pit_imp_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("3decbc45-3e79-455f-9d62-878ee31b458d"),
        primary_mana: &[(ManaColor::Black, 1)],
        alt_mana: None,
        total_mana: 1,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black],
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

// Portcullis Vine — {G} — Creature — Plant Wall
// Oracle: Defender (This creature can't attack.) / {2}, {T}, Sacrifice a creature with defender: Draw a card.
#[test]
fn portcullis_vine_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("a217f386-11b5-48f0-b173-3b0700ba295f"),
        primary_mana: &[(ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 1,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Green],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (0, 3),
        expected_pt: Some((0, 3)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::DEFENDER,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Priest of Iroas — {R} — Creature — Human Cleric
// Oracle: {3}{W}, Sacrifice this creature: Destroy target enchantment.
#[test]
fn priest_of_iroas_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("706f7d56-f4da-48ba-ac94-fc09580beec7"),
        primary_mana: &[(ManaColor::Red, 1)],
        alt_mana: None,
        total_mana: 1,
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

// Prophet of Distortion — {U} — Creature — Eldrazi Drone
// Oracle: Devoid (This card has no color.) / {3}{C}: Draw a card. ({C} represents colorless mana.)
#[test]
fn prophet_of_distortion_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("6bc82093-0246-4c60-bf30-0b605bef01cd"),
        primary_mana: &[(ManaColor::Blue, 1)],
        alt_mana: None,
        total_mana: 1,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[],
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
