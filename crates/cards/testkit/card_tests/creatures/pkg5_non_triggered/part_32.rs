//! Package 5 non-triggered creature behavioral tests — part 32.

#[allow(clippy::wildcard_imports)]
use super::*;

// Fallaji Chaindancer — {3}{R} — Creature — Human Soldier
// Oracle: {2}: This creature gains double strike until end of turn.
#[test]
fn fallaji_chaindancer_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("cb690399-0fb8-4bd8-a73d-5e0e594ff2ae"),
        primary_mana: &[(ManaColor::Red, 1), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red],
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

// Fen Stalker — {3}{B} — Creature — Nightstalker
// Oracle: This creature has fear as long as you control no untapped lands. (It can't be blocked except by artifact creatures and/or black creatures.)
#[test]
fn fen_stalker_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("cfd00c0e-d034-4b39-a3d3-f8b8842045dd"),
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 4,
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

// Fire Nation Archers — {3}{R} — Creature — Human Archer
// Oracle: Reach (This creature can block creatures with flying.) / {5}: This creature deals 2 damage to each opponent. Create a 2/2 red Soldier creature token.
#[test]
fn fire_nation_archers_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("2e735849-1930-4f43-86b4-da660ade15bc"),
        primary_mana: &[(ManaColor::Red, 1), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 4,
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

// Fireborn Knight — {R/W}{R/W}{R/W}{R/W} — Creature — Human Knight
// Oracle: Double strike / {R/W}{R/W}{R/W}{R/W}: This creature gets +1/+1 until end of turn.
#[test]
fn fireborn_knight_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("b77695da-86fb-4675-b2d3-51732ad0325f"),
        primary_mana: &[(ManaColor::Red, 4)],
        alt_mana: Some(&[(ManaColor::White, 4)]),
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Red],
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
        expected_keywords: KeywordSet::DOUBLE_STRIKE,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Foriysian Brigade — {3}{W} — Creature — Human Soldier
// Oracle: This creature can block an additional creature each combat.
#[test]
fn foriysian_brigade_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("7261dc6d-7669-4682-977b-1304f57a201d"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 4,
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

// Foriysian Interceptor — {3}{W} — Creature — Human Soldier
// Oracle: Flash (You may cast this spell any time you could cast an instant.) / Defender / This creature can block an additional creature each combat.
#[test]
fn foriysian_interceptor_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("52989a30-773d-4daf-aea3-8dbee2a947fa"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (0, 5),
        expected_pt: Some((0, 5)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FLASH.union(KeywordSet::DEFENDER),
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Furnace Brood — {3}{R} — Creature — Elemental
// Oracle: {R}: Target creature can't be regenerated this turn.
#[test]
fn furnace_brood_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("cc87a268-253b-4377-bdc5-474940de8878"),
        primary_mana: &[(ManaColor::Red, 1), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 4,
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

// Ghor-Clan Bloodscale — {3}{R} — Creature — Lizard Warrior
// Oracle: First strike / {3}{G}: This creature gets +2/+2 until end of turn. Activate only once each turn.
#[test]
fn ghor_clan_bloodscale_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("e3367ed8-16a3-4b04-bc6a-e0a3c794e6d3"),
        primary_mana: &[(ManaColor::Red, 1), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red],
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

// Ghost Ship — {2}{U}{U} — Creature — Spirit
// Oracle: Flying / {U}{U}{U}: Regenerate this creature.
#[test]
fn ghost_ship_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("09de1136-8a8f-4539-92a2-611dcdd0790b"),
        primary_mana: &[(ManaColor::Blue, 2), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue],
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

// Giant Solifuge — {2}{R/G}{R/G} — Creature — Insect
// Oracle: ({R/G} can be paid with either {R} or {G}.) / Trample; haste; shroud (This creature can't be the target of spells or abilities.)
#[test]
fn giant_solifuge_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("c590c1bd-430c-4515-9343-ba818c02bc5f"),
        primary_mana: &[(ManaColor::Red, 2), (ManaColor::Colorless, 2)],
        alt_mana: Some(&[(ManaColor::Green, 2), (ManaColor::Colorless, 2)]),
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red, Color::Green],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (4, 1),
        expected_pt: Some((4, 1)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::TRAMPLE
            .union(KeywordSet::HASTE)
            .union(KeywordSet::SHROUD),
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Gigantoad — {3}{G} — Creature — Frog
// Oracle: As long as you control seven or more lands, this creature gets +2/+2.
#[test]
fn gigantoad_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("fa5e4aa5-e2cb-4c8c-91cb-554bec0f21d4"),
        primary_mana: &[(ManaColor::Green, 1), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 4,
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

// Glen Elendra Liege — {1}{U/B}{U/B}{U/B} — Creature — Faerie Knight
// Oracle: Flying / Other blue creatures you control get +1/+1. / Other black creatures you control get +1/+1.
#[test]
fn glen_elendra_liege_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("946bba74-0951-408c-b06f-167739b10934"),
        primary_mana: &[(ManaColor::Blue, 3), (ManaColor::Colorless, 1)],
        alt_mana: Some(&[(ManaColor::Black, 3), (ManaColor::Colorless, 1)]),
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue, Color::Black],
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

// Glittermonger — {3}{G} — Creature — Elf Rogue
// Oracle: {T}: Create a Treasure token. (It's an artifact with "{T}, Sacrifice this token: Add one mana of any color.")
#[test]
fn glittermonger_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("184c763c-b52a-471c-bd87-d01729ebe5e4"),
        primary_mana: &[(ManaColor::Green, 1), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Green],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (1, 4),
        expected_pt: Some((1, 4)),
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

// Goblin Trashmaster — {2}{R}{R} — Creature — Goblin Warrior
// Oracle: Other Goblins you control get +1/+1. / Sacrifice a Goblin: Destroy target artifact.
#[test]
fn goblin_trashmaster_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("0283bf5e-ddf2-4a4a-a7cf-d3e27eed7e7d"),
        primary_mana: &[(ManaColor::Red, 2), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 4,
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
        lord_subtype: Some(subtypes::creature::GOBLIN),
        lord_pt: Some((1, 1)),
        lord_controlled_by_you: true,
    });
}

// Goldmeadow Lookout — {3}{W} — Creature — Kithkin Spellshaper
// Oracle: {W}, {T}, Discard a card: Create a 1/1 white Kithkin Soldier creature token named Goldmeadow Harrier. It has "{W}, {T}: Tap target creature."
#[test]
fn goldmeadow_lookout_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("28b2d31f-1ff7-4374-8ea3-0dd4a96c4a32"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 4,
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

// Gorilla Chieftain — {2}{G}{G} — Creature — Ape
// Oracle: {1}{G}: Regenerate this creature.
#[test]
fn gorilla_chieftain_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("4137ee86-bbf0-49dc-87c5-b48a384baac4"),
        primary_mana: &[(ManaColor::Green, 2), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 4,
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

// Gravblade Heavy — {3}{B} — Creature — Human Soldier
// Oracle: As long as you control an artifact, this creature gets +1/+0 and has deathtouch.
#[test]
fn gravblade_heavy_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("67821712-e58f-4fee-81af-021059ac72ea"),
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (3, 4),
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

// Greenhilt Trainee — {3}{G} — Creature — Elf Warrior
// Oracle: {T}: Target creature gets +4/+4 until end of turn. Activate only if this creature's power is 4 or greater.
#[test]
fn greenhilt_trainee_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("1d7d340d-fe6f-4314-97ba-e0bf36a4ef6c"),
        primary_mana: &[(ManaColor::Green, 1), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 4,
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

// Grizzly Ghoul — {2}{B}{G} — Creature — Zombie Bear
// Oracle: Trample / This creature enters with a +1/+1 counter on it for each creature that died this turn.
#[test]
fn grizzly_ghoul_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("86783f8d-26c9-4b8c-a697-ab6facafeb67"),
        primary_mana: &[
            (ManaColor::Black, 1),
            (ManaColor::Green, 1),
            (ManaColor::Colorless, 2),
        ],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black, Color::Green],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (4, 3),
        expected_pt: Some((8, 7)),
        enters_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: Some((CounterKind::P1P1, 4)),
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

// Guardian of the Great Conduit — {3}{G} — Creature — Elemental
// Oracle: Reach (This creature can block creatures with flying.) / As long as you control a Nissa planeswalker, this creature gets +2/+0 and has vigilance. (Attacking doesn't cause it to tap.)
#[test]
fn guardian_of_the_great_conduit_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("1342bd59-91d3-4a5b-b853-542a52d7da6d"),
        primary_mana: &[(ManaColor::Green, 1), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Green],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (2, 4),
        expected_pt: None,
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

// Half-Elf Monk — {3}{W} — Creature — Human Elf Monk
// Oracle: Vigilance / Stunning Strike — {1}{W}, {T}: Tap target creature.
#[test]
fn half_elf_monk_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("00d11662-9d10-4915-b80c-c1ad54b4e71b"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (1, 4),
        expected_pt: Some((1, 4)),
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

// Havoc Sower — {3}{B} — Creature — Eldrazi Drone
// Oracle: Devoid (This card has no color.) / {1}{C}: This creature gets +2/+1 until end of turn. ({C} represents colorless mana.)
#[test]
fn havoc_sower_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("098e6926-4a6a-4bbb-96bc-1f35cc7982ae"),
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[],
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

// Heavy Ballista — {3}{W} — Creature — Human Soldier
// Oracle: {T}: This creature deals 2 damage to target attacking or blocking creature.
#[test]
fn heavy_ballista_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("b4219836-c802-49e2-9a66-f03ca8f13895"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White],
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

// Heedless One — {3}{G} — Creature — Elf Avatar
// Oracle: Trample / Heedless One's power and toughness are each equal to the number of Elves on the battlefield.
#[test]
fn heedless_one_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("c4ec45e9-1a4d-42a0-beca-e4dcdfc88fa3"),
        primary_mana: &[(ManaColor::Green, 1), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 4,
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

// Homarid Shaman — {2}{U}{U} — Creature — Homarid Shaman
// Oracle: {U}: Tap target green creature.
#[test]
fn homarid_shaman_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("137e3974-74e1-4941-8597-10aeba6cac37"),
        primary_mana: &[(ManaColor::Blue, 2), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 4,
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

// Horde of Boggarts — {3}{R} — Creature — Goblin
// Oracle: Menace (This creature can't be blocked except by two or more creatures.) / Horde of Boggarts's power and toughness are each equal to the number of red permanents you control.
#[test]
fn horde_of_boggarts_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("1229b82f-fc50-4259-83c6-31f706ac2303"),
        primary_mana: &[(ManaColor::Red, 1), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red],
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
        expected_keywords: KeywordSet::MENACE,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Hoverguard Observer — {2}{U}{U} — Creature — Drone
// Oracle: Flying / This creature can block only creatures with flying.
#[test]
fn hoverguard_observer_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("5ca8d756-a55a-4b36-b0a6-562a39484952"),
        primary_mana: &[(ManaColor::Blue, 2), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue],
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
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Inquisitor Greyfax — {1}{W}{U}{B} — Legendary Creature — Human Inquisitor
// Oracle: Vigilance / Unquestionable Wisdom — Other creatures you control get +1/+0 and have vigilance. / Hunt for Heresy — {1}, {T}: Tap target creature an opponent controls. Investigate. (Create a Clue token. It's an artifact with "{2}, Sacrifice this token: Draw a card.")
#[test]
fn inquisitor_greyfax_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("9439d6eb-4c10-451d-85c4-3752611c671c"),
        primary_mana: &[
            (ManaColor::White, 1),
            (ManaColor::Blue, 1),
            (ManaColor::Black, 1),
            (ManaColor::Colorless, 1),
        ],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Blue, Color::Black],
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
        expected_keywords: KeywordSet::VIGILANCE,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: Some((1, 0)),
        lord_controlled_by_you: true,
    });
}

// Jeskai Brushmaster — {1}{U}{R}{W} — Creature — Orc Monk
// Oracle: Double strike / Prowess (Whenever you cast a noncreature spell, this creature gets +1/+1 until end of turn.)
#[test]
fn jeskai_brushmaster_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("eff690a7-c2c7-4750-894c-0667d3151165"),
        primary_mana: &[
            (ManaColor::White, 1),
            (ManaColor::Blue, 1),
            (ManaColor::Red, 1),
            (ManaColor::Colorless, 1),
        ],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Blue, Color::Red],
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

// Jetmir, Nexus of Revels — {1}{R}{G}{W} — Legendary Creature — Cat Demon
// Oracle: Creatures you control get +1/+0 and have vigilance as long as you control three or more creatures. / Creatures you control also get +1/+0 and have trample as long as you control six or more creatures. / Creatures you control also get +1/+0 and have double strike as long as you control nine or more creatures.
#[test]
fn jetmir_nexus_of_revels_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("da72a4bc-ce6f-4b72-bc66-2ee33cfa87df"),
        primary_mana: &[
            (ManaColor::White, 1),
            (ManaColor::Red, 1),
            (ManaColor::Green, 1),
            (ManaColor::Colorless, 1),
        ],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Red, Color::Green],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: true,
        is_snow: false,
        base_pt: (5, 4),
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
