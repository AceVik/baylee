//! Package 6 triggered creature behavioral tests — part 06.

#[allow(clippy::wildcard_imports)]
use super::*;

// Compassionate Healer — {1}{W} — Creature — Human Cleric Ally
// Oracle: Whenever this creature becomes tapped, you gain 1 life and scry 1. (Look at the top card of your library. You may put it on the bottom.)
#[test]
fn compassionate_healer_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("6ef464bd-2ed0-4f99-af4a-ebf276f55384"),
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
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: true,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Contraband Kingpin — {U}{B} — Creature — Aetherborn Rogue
// Oracle: Lifelink / Whenever an artifact you control enters, scry 1.
#[test]
fn contraband_kingpin_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("680da572-6cd5-482a-b01d-32944566d8c4"),
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Black, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue, Color::Black],
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
        expected_keywords: KeywordSet::LIFELINK,
        has_enters_other: true,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Corpse Knight — {W}{B} — Creature — Zombie Knight
// Oracle: Whenever another creature you control enters, each opponent loses 1 life.
#[test]
fn corpse_knight_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("0c77b805-728d-4ae4-88a9-223f4b7b52e0"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Black, 1)],
        alt_mana: None,
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

// Corrupt Court Official — {1}{B} — Creature — Human Advisor
// Oracle: When this creature enters, target opponent discards a card.
#[test]
fn corrupt_court_official_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("ec84201d-757f-4a7a-b3a2-85ccd5ee81ae"),
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

// Cruel Celebrant — {W}{B} — Creature — Vampire
// Oracle: Whenever this creature or another creature or planeswalker you control dies, each opponent loses 1 life and you gain 1 life.
#[test]
fn cruel_celebrant_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("3ee78cfc-0e9e-4737-a7e2-b42f94228040"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Black, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Black],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (1, 2),
        expected_pt: Some((1, 2)),
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

// Dauntless Survivor — {1}{G} — Creature — Human Warrior
// Oracle: When this creature enters, put a +1/+1 counter on target creature.
#[test]
fn dauntless_survivor_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("8371dee1-5f4a-410a-8b7c-e4110cdbe9bd"),
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
        base_pt: (1, 1),
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

// Dawnhart Disciple — {1}{G} — Creature — Human Warlock
// Oracle: Whenever another Human you control enters, this creature gets +1/+1 until end of turn.
#[test]
fn dawnhart_disciple_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("e86279ba-cb22-4754-90f4-38354657b8fa"),
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

// Dawnhart Geist — {1}{W} — Creature — Spirit Warlock
// Oracle: Whenever you cast an enchantment spell, you gain 2 life.
#[test]
fn dawnhart_geist_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("ed7a2ee0-289d-4e5f-92e0-1539395baf33"),
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
        base_pt: (1, 3),
        expected_pt: Some((1, 3)),
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

// Daybreak Charger — {1}{W} — Creature — Unicorn
// Oracle: When this creature enters, target creature gets +2/+0 until end of turn.
#[test]
fn daybreak_charger_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("d3de6b36-c631-4b52-b16b-0749a347d73d"),
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

// Deathknell Berserker — {1}{B} — Creature — Elf Berserker
// Oracle: When this creature dies, if its power was 3 or greater, create a 2/2 black Zombie Berserker creature token.
#[test]
fn deathknell_berserker_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("4658ae19-d203-4c12-b56e-42cd8f0db91d"),
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

// Deepchannel Duelist — {W}{U} — Creature — Merfolk Soldier
// Oracle: At the beginning of your end step, untap target Merfolk you control. / Other Merfolk you control get +1/+1.
#[test]
fn deepchannel_duelist_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("c892d28c-4a78-4a7f-b8d5-c3663b819c5e"),
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
        expected_pt: None,
        enters_tapped: false,
        etb_changes_tapped: true,
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

// Deeproot Champion — {1}{G} — Creature — Merfolk Shaman
// Oracle: Whenever you cast a noncreature spell, put a +1/+1 counter on this creature.
#[test]
fn deeproot_champion_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("507c2921-7070-482d-8754-dc667e3315bd"),
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
        base_pt: (1, 1),
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

// Deeproot Elite — {1}{G} — Creature — Merfolk Warrior
// Oracle: Whenever another Merfolk you control enters, put a +1/+1 counter on target Merfolk you control.
#[test]
fn deeproot_elite_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("764e8e6e-0ace-4fc8-8ca5-468a10ae3b73"),
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
        base_pt: (1, 1),
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

// Deputy of Acquittals — {W}{U} — Creature — Human Wizard
// Oracle: Flash (You may cast this spell any time you could cast an instant.) / When this creature enters, you may return another target creature you control to its owner's hand.
#[test]
fn deputy_of_acquittals_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("3cbb5045-8566-4279-b7d3-3e599b11ccc5"),
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
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FLASH,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Desperate Bloodseeker — {1}{B} — Creature — Vampire
// Oracle: Lifelink / When this creature enters, target player mills two cards. (They put the top two cards of their library into their graveyard.)
#[test]
fn desperate_bloodseeker_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("de8d7cad-02c7-43f1-896e-eaa73f016ada"),
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
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::LIFELINK,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Dire Fleet Captain — {B}{R} — Creature — Orc Pirate
// Oracle: Whenever this creature attacks, it gets +1/+1 until end of turn for each other attacking Pirate.
#[test]
fn dire_fleet_captain_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("68f1da0d-1143-457a-b709-1da3c060a45d"),
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::Red, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black, Color::Red],
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
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Dire Fleet Hoarder — {1}{B} — Creature — Human Pirate
// Oracle: When this creature dies, create a Treasure token. (It's an artifact with "{T}, Sacrifice this token: Add one mana of any color.")
#[test]
fn dire_fleet_hoarder_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("7d48ad95-d428-4e2d-b697-56907c3ce579"),
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

// Dire Fleet Poisoner — {1}{B} — Creature — Human Pirate
// Oracle: Flash / Deathtouch / When this creature enters, target attacking Pirate you control gets +1/+1 and gains deathtouch until end of turn.
#[test]
fn dire_fleet_poisoner_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("8150cc84-3f9f-470d-a0f9-ee6d9dab13da"),
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
        expected_pt: None,
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FLASH.union(KeywordSet::DEATHTOUCH),
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Discordant Piper — {1}{B} — Creature — Zombie Satyr
// Oracle: When this creature dies, create a 0/1 white Goat creature token.
#[test]
fn discordant_piper_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("bb0107f4-723a-4f2c-8e4b-237d5f668e3c"),
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

// Doomed Dissenter — {1}{B} — Creature — Human
// Oracle: When this creature dies, create a 2/2 black Zombie creature token.
#[test]
fn doomed_dissenter_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("11cb509d-21af-48f1-b355-135ebd3e4bd1"),
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

// Dream Stalker — {1}{U} — Creature — Illusion
// Oracle: When this creature enters, return a permanent you control to its owner's hand.
#[test]
fn dream_stalker_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("6e67551b-1f6e-4ef4-9baf-520607b8cb61"),
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
        base_pt: (1, 5),
        expected_pt: Some((1, 5)),
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

// Drekavac — {1}{B} — Creature — Beast
// Oracle: When this creature enters, sacrifice it unless you discard a noncreature card.
#[test]
fn drekavac_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("f7971f22-2689-4c6f-9fea-1afc4f5afed3"),
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
        base_pt: (3, 3),
        expected_pt: Some((3, 3)),
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

// Dundoolin Weaver — {1}{G} — Creature — Kithkin Druid
// Oracle: When this creature enters, if you control three or more creatures, return target permanent card from your graveyard to your hand.
#[test]
fn dundoolin_weaver_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("28564d10-bfc5-4beb-b0e2-b9aa5c03f740"),
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

// Dwarven Castle Guard — {1}{W} — Creature — Dwarf Soldier
// Oracle: When this creature dies, create a 1/1 colorless Hero creature token.
#[test]
fn dwarven_castle_guard_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("b51e0ae3-126b-4b7e-9dd1-818a5b8e512a"),
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
        base_pt: (2, 1),
        expected_pt: Some((2, 1)),
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

// Elas il-Kor, Sadistic Pilgrim — {W}{B} — Legendary Creature — Phyrexian Kor Cleric
// Oracle: Deathtouch / Whenever another creature you control enters, you gain 1 life. / Whenever another creature you control dies, each opponent loses 1 life.
#[test]
fn elas_il_kor_sadistic_pilgrim_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("a4d33e21-d3fe-4d5b-903c-b75e859d6f7f"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Black, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Black],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: true,
        is_snow: false,
        base_pt: (2, 2),
        expected_pt: Some((2, 2)),
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::DEATHTOUCH,
        has_enters_other: true,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Elderfang Disciple — {1}{B} — Creature — Elf Cleric
// Oracle: When this creature enters, each opponent discards a card.
#[test]
fn elderfang_disciple_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("8db63f1a-0d63-4b11-966d-818d83a082f9"),
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

// Electrostatic Field — {1}{R} — Creature — Wall
// Oracle: Defender / Whenever you cast an instant or sorcery spell, this creature deals 1 damage to each opponent.
#[test]
fn electrostatic_field_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("2fa94a07-c932-4f85-b6e0-a97d2b29eb52"),
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
        base_pt: (0, 4),
        expected_pt: Some((0, 4)),
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::DEFENDER,
        has_enters_other: false,
        has_spell_cast: true,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Electrostatic Infantry — {1}{R} — Creature — Dwarf Wizard
// Oracle: Trample / Whenever you cast an instant or sorcery spell, put a +1/+1 counter on this creature.
#[test]
fn electrostatic_infantry_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("ba8883fc-db24-4c3d-ae36-05085089c9cc"),
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
        base_pt: (1, 2),
        expected_pt: None,
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::TRAMPLE,
        has_enters_other: false,
        has_spell_cast: true,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Elusive Spellfist — {1}{U} — Creature — Human Monk
// Oracle: Whenever you cast a noncreature spell, this creature gets +1/+0 until end of turn and can't be blocked this turn.
#[test]
fn elusive_spellfist_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("b9ba00f2-b11c-45d8-9c38-f9ade5d07d0f"),
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
        base_pt: (1, 3),
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

// Elvish Doomsayer — {1}{B} — Creature — Elf Shaman
// Oracle: When this creature dies, each opponent discards a card.
#[test]
fn elvish_doomsayer_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("5fba553e-81b9-419b-aa20-b2a7cfcd042b"),
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
