//! Package 6 triggered creature behavioral tests — part 16.

#[allow(clippy::wildcard_imports)]
use super::*;

// Crackling Cyclops — {2}{R} — Creature — Cyclops Wizard
// Oracle: Whenever you cast a noncreature spell, this creature gets +3/+0 until end of turn.
#[test]
fn crackling_cyclops_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("16ab9451-f3da-4f61-8e2b-d9c84e0e5428"),
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
        base_pt: (0, 4),
        expected_pt: None,
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: false,
        has_spell_cast: true,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Crossroads Watcher — {2}{G} — Creature — Kithkin Ranger
// Oracle: Trample / Whenever another creature you control enters, this creature gets +1/+0 until end of turn.
#[test]
fn crossroads_watcher_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("ab5f8b8c-e3f7-4265-bbc6-814bbfd4693a"),
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
        base_pt: (3, 3),
        expected_pt: None,
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::TRAMPLE,
        has_enters_other: true,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Cunning Lethemancer — {2}{B} — Creature — Human Wizard
// Oracle: At the beginning of your upkeep, each player discards a card.
#[test]
fn cunning_lethemancer_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("d696aba9-ef3e-4f81-821a-c1df51431b8a"),
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
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Dauntless Aven — {2}{W} — Creature — Bird Warrior
// Oracle: Flying / Whenever this creature attacks, untap target creature you control.
#[test]
fn dauntless_aven_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("7104e471-06aa-426d-af1e-d5f0b1fb18c3"),
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
        etb_changes_tapped: true,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FLYING,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Daybreak Combatants — {2}{R} — Creature — Human Warrior
// Oracle: Haste (This creature can attack and {T} as soon as it comes under your control.) / When this creature enters, target creature gets +2/+0 until end of turn.
#[test]
fn daybreak_combatants_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("fbb3f959-ded7-4570-bacf-0e1394dbef80"),
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
        expected_pt: None,
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::HASTE,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Deadbridge Shaman — {2}{B} — Creature — Elf Shaman
// Oracle: When this creature dies, target opponent discards a card.
#[test]
fn deadbridge_shaman_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("a57389ad-7f2a-47a6-8ae6-2821503cbb28"),
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
        base_pt: (3, 1),
        expected_pt: Some((3, 1)),
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Devout Lightcaster — {W}{W}{W} — Creature — Kor Cleric
// Oracle: Protection from black / When this creature enters, exile target black permanent.
#[test]
fn devout_lightcaster_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("a6f14387-4dfd-4d6b-a6b1-2ae658a70de6"),
        primary_mana: &[(ManaColor::White, 3)],
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
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Dewdrop Spy — {1}{U}{U} — Creature — Faerie Rogue
// Oracle: Flash / Flying / When this creature enters, look at the top card of target player's library.
#[test]
fn dewdrop_spy_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("14bee98b-6dd8-48f3-8043-06f03cc93aa7"),
        primary_mana: &[(ManaColor::Blue, 2), (ManaColor::Colorless, 1)],
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
        base_pt: (2, 2),
        expected_pt: Some((2, 2)),
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FLASH.union(KeywordSet::FLYING),
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Dimir Cutpurse — {1}{U}{B} — Creature — Spirit
// Oracle: Whenever this creature deals combat damage to a player, that player discards a card and you draw a card.
#[test]
fn dimir_cutpurse_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("56c1a548-c80c-432b-bf92-d4ef6117b298"),
        primary_mana: &[
            (ManaColor::Blue, 1),
            (ManaColor::Black, 1),
            (ManaColor::Colorless, 1),
        ],
        alt_mana: None,
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
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Dimir Informant — {2}{U} — Creature — Human Rogue
// Oracle: When this creature enters, surveil 2. (Look at the top two cards of your library, then put any number of them into your graveyard and the rest on top of your library in any order.)
#[test]
fn dimir_informant_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("fa66d63b-dfb7-42b8-8249-4d60e89058ac"),
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
        base_pt: (1, 4),
        expected_pt: Some((1, 4)),
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Diregraf Captain — {1}{U}{B} — Creature — Zombie Soldier
// Oracle: Deathtouch / Other Zombie creatures you control get +1/+1. / Whenever another Zombie you control dies, target opponent loses 1 life.
#[test]
fn diregraf_captain_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("70de24d9-c585-4bf6-ac2c-c5b4b7aa298c"),
        primary_mana: &[
            (ManaColor::Blue, 1),
            (ManaColor::Black, 1),
            (ManaColor::Colorless, 1),
        ],
        alt_mana: None,
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
        expected_pt: None,
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::DEATHTOUCH,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Donatello, Way with Machines — {2}{U} — Legendary Creature — Mutant Ninja Turtle
// Oracle: Flying / Whenever an artifact you control enters, put a +1/+1 counter on Donatello.
#[test]
fn donatello_way_with_machines_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("194af2be-50c7-484b-bdd3-402bad70335b"),
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: true,
        is_snow: false,
        base_pt: (1, 2),
        expected_pt: None,
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FLYING,
        has_enters_other: true,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Dori, Bearer of Friends — {2}{R} — Legendary Creature — Dwarf Warrior
// Oracle: Trample / When Dori enters, create a Treasure token. (It's an artifact with "{T}, Sacrifice this token: Add one mana of any color.")
#[test]
fn dori_bearer_of_friends_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("58acd6b2-730c-4472-a34a-f2784977aca8"),
        primary_mana: &[(ManaColor::Red, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: true,
        is_snow: false,
        base_pt: (3, 2),
        expected_pt: Some((3, 2)),
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::TRAMPLE,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Dragon Egg — {2}{R} — Creature — Dragon Egg
// Oracle: Defender / When this creature dies, create a 2/2 red Dragon creature token with flying and "{R}: This token gets +1/+0 until end of turn."
#[test]
fn dragon_egg_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("8f561498-66e7-4cbf-8cb9-e6fd4717993d"),
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
        base_pt: (0, 2),
        expected_pt: Some((0, 2)),
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::DEFENDER,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Dragoon's Wyvern — {2}{U} — Creature — Drake
// Oracle: Flying / When this creature enters, create a 1/1 colorless Hero creature token.
#[test]
fn dragoon_s_wyvern_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("f328012f-4696-47bb-ab43-247bdcb13f03"),
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
        base_pt: (2, 1),
        expected_pt: Some((2, 1)),
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FLYING,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Drana's Emissary — {1}{W}{B} — Creature — Vampire Cleric Ally
// Oracle: Flying / At the beginning of your upkeep, each opponent loses 1 life and you gain 1 life.
#[test]
fn drana_s_emissary_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("ef49b53d-e5b6-423a-88f0-51a7b2021c46"),
        primary_mana: &[
            (ManaColor::White, 1),
            (ManaColor::Black, 1),
            (ManaColor::Colorless, 1),
        ],
        alt_mana: None,
        total_mana: 3,
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
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FLYING,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Dross Harvester — {1}{B}{B} — Creature — Horror
// Oracle: Protection from white / At the beginning of your end step, you lose 4 life. / Whenever a creature dies, you gain 2 life.
#[test]
fn dross_harvester_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("5520ada0-f173-4b5e-a0b8-a5938a1c0a4b"),
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
        base_pt: (4, 4),
        expected_pt: Some((4, 4)),
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Duergar Hedge-Mage — {2}{R/W} — Creature — Dwarf Shaman
// Oracle: When this creature enters, if you control two or more Mountains, you may destroy target artifact. / When this creature enters, if you control two or more Plains, you may destroy target enchantment.
#[test]
fn duergar_hedge_mage_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("ecb2fd58-b880-4d9e-93cb-c4d2b28233c9"),
        primary_mana: &[(ManaColor::Red, 1), (ManaColor::Colorless, 2)],
        alt_mana: Some(&[(ManaColor::White, 1), (ManaColor::Colorless, 2)]),
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Red],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (2, 2),
        expected_pt: Some((2, 2)),
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Duskwatch Hunter — {2}{B/G} — Creature — Wolf
// Oracle: This creature can't be blocked by tokens. / When this creature enters, put a +1/+1 counter on target creature.
#[test]
fn duskwatch_hunter_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("6283e5ca-9924-478b-89a5-36924accb9aa"),
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::Colorless, 2)],
        alt_mana: Some(&[(ManaColor::Green, 1), (ManaColor::Colorless, 2)]),
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black, Color::Green],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (3, 1),
        expected_pt: None,
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Eager Trufflesnout — {2}{G} — Creature — Boar
// Oracle: Trample (This creature can deal excess combat damage to the player or planeswalker it's attacking.) / Whenever this creature deals combat damage to a player, create a Food token. (It's an artifact with "{2}, {T}, Sacrifice this token: You gain 3 life.")
#[test]
fn eager_trufflesnout_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("7f2ba562-8d5e-4915-967e-54916c3c5876"),
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
        base_pt: (4, 2),
        expected_pt: Some((4, 2)),
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::TRAMPLE,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Elder Auntie — {2}{R} — Creature — Goblin Warlock
// Oracle: When this creature enters, create a 1/1 black and red Goblin creature token.
#[test]
fn elder_auntie_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("079f0d7f-57a5-4101-a9e3-c5a57148b3fb"),
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
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Elderfang Ritualist — {2}{B} — Creature — Elf Cleric
// Oracle: When this creature dies, return another target Elf card from your graveyard to your hand.
#[test]
fn elderfang_ritualist_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("29c1f80e-f7e2-425e-a225-e88351fcbaab"),
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
        base_pt: (3, 1),
        expected_pt: Some((3, 1)),
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Eldrazi Linebreaker — {1}{C}{R} — Creature — Eldrazi
// Oracle: Devoid (This card has no color.) / Trample / At the beginning of combat on your turn, target creature you control gains haste and gets +X/+0 until end of turn, where X is the number of Eldrazi you control.
#[test]
fn eldrazi_linebreaker_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("db432387-58d3-4473-9442-b5b586dbea38"),
        primary_mana: &[(ManaColor::Red, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 1,
        snow_pips: 0,
        has_x: false,
        colors: &[],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (3, 3),
        expected_pt: None,
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::DEVOID.union(KeywordSet::TRAMPLE),
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Eldrazi Skyspawner — {2}{U} — Creature — Eldrazi Drone
// Oracle: Devoid (This card has no color.) / Flying / When this creature enters, create a 1/1 colorless Eldrazi Scion creature token. It has "Sacrifice this token: Add {C}."
#[test]
fn eldrazi_skyspawner_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("b5661289-a03c-40fc-a1e4-f602adb56f81"),
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (2, 1),
        expected_pt: Some((2, 1)),
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::DEVOID.union(KeywordSet::FLYING),
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Elite Instructor — {2}{U} — Creature — Human Wizard
// Oracle: When this creature enters, draw a card, then discard a card.
#[test]
fn elite_instructor_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("6536f72e-ff4e-4cda-bc7d-909145adf829"),
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
        base_pt: (2, 2),
        expected_pt: Some((2, 2)),
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Emancipation Angel — {1}{W}{W} — Creature — Angel
// Oracle: Flying / When this creature enters, return a permanent you control to its owner's hand.
#[test]
fn emancipation_angel_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("3834d3b0-9b66-465a-a8dc-22875a819fd9"),
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
        base_pt: (3, 3),
        expected_pt: Some((3, 3)),
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FLYING,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Enraged Flamecaster — {2}{R} — Creature — Elemental Sorcerer
// Oracle: Reach / Whenever you cast a spell with mana value 4 or greater, this creature deals 2 damage to each opponent.
#[test]
fn enraged_flamecaster_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("e35d7886-8c03-471c-b598-118d7eceddb4"),
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
        base_pt: (3, 2),
        expected_pt: Some((3, 2)),
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::REACH,
        has_enters_other: false,
        has_spell_cast: true,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// EPF Point Squad — {1}{R/W}{R/W} — Creature — Human Soldier
// Oracle: Alliance — Whenever another creature you control enters, put a +1/+1 counter on this creature.
#[test]
fn epf_point_squad_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("2dea1935-c227-46d5-9d98-e7d2cd612f98"),
        primary_mana: &[(ManaColor::Red, 2), (ManaColor::Colorless, 1)],
        alt_mana: Some(&[(ManaColor::White, 2), (ManaColor::Colorless, 1)]),
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Red],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (2, 1),
        expected_pt: None,
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: true,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Erdwal Ripper — {1}{R}{R} — Creature — Vampire
// Oracle: Haste / Whenever this creature deals combat damage to a player, put a +1/+1 counter on it.
#[test]
fn erdwal_ripper_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("6a207599-188f-49fc-8932-1974fc716f0b"),
        primary_mana: &[(ManaColor::Red, 2), (ManaColor::Colorless, 1)],
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
        base_pt: (2, 1),
        expected_pt: None,
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::HASTE,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Eutropia the Twice-Favored — {1}{G}{U} — Legendary Creature — Human Wizard
// Oracle: Constellation — Whenever an enchantment you control enters, put a +1/+1 counter on target creature. That creature gains flying until end of turn.
#[test]
fn eutropia_the_twice_favored_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("4de1319f-b3c7-42d4-9342-3e9c4f526ee0"),
        primary_mana: &[
            (ManaColor::Blue, 1),
            (ManaColor::Green, 1),
            (ManaColor::Colorless, 1),
        ],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue, Color::Green],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: true,
        is_snow: false,
        base_pt: (2, 2),
        expected_pt: None,
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: true,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}
