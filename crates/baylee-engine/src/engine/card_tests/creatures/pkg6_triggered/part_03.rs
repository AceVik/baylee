//! Package 6 triggered creature behavioral tests — part 03.

#[allow(clippy::wildcard_imports)]
use super::*;

// Jaddi Offshoot — {G} — Creature — Plant
// Oracle: Defender / Landfall — Whenever a land you control enters, you gain 1 life.
#[test]
fn jaddi_offshoot_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("20faefbd-059c-4aae-81e3-31683bf9f7bf"),
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
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::DEFENDER,
        has_enters_other: true,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Knucklebone Witch — {B} — Creature — Goblin Shaman
// Oracle: Whenever a Goblin you control is put into a graveyard from the battlefield, you may put a +1/+1 counter on this creature.
#[test]
fn knucklebone_witch_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("4d165c0f-3e40-45ad-92a1-53b13c07b34d"),
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
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Leonin Vanguard — {W} — Creature — Cat Soldier
// Oracle: At the beginning of combat on your turn, if you control three or more creatures, this creature gets +1/+1 until end of turn and you gain 1 life.
#[test]
fn leonin_vanguard_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("de1e1352-adb2-4ac1-bb2a-ee654d5c727d"),
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

// Llanowar Stalker — {G} — Creature — Elf Warrior
// Oracle: Whenever another creature you control enters, this creature gets +1/+0 until end of turn.
#[test]
fn llanowar_stalker_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("90d375fa-0487-4990-9a98-260f9ee28d2a"),
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

// Mardu Devotee — {W} — Creature — Human Scout
// Oracle: When this creature enters, scry 2. (Look at the top two cards of your library, then put any number of them on the bottom and the rest on top in any order.) / {1}: Add {R}, {W}, or {B}. Activate only once each turn.
#[test]
fn mardu_devotee_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("24ee57b5-a505-4264-97ca-8b8dfb12298e"),
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

// Merrow Witsniper — {U} — Creature — Merfolk Rogue
// Oracle: When this creature enters, target player mills a card.
#[test]
fn merrow_witsniper_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("79f8378a-fb50-4178-be53-b6926bc1cba5"),
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

// Mirkwood Spider — {G} — Creature — Spider
// Oracle: Deathtouch / Whenever this creature attacks, target legendary creature you control gains deathtouch until end of turn.
#[test]
fn mirkwood_spider_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("07fb2903-b239-43d0-864b-c4437b7a5230"),
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
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Mold Adder — {G} — Creature — Fungus Snake
// Oracle: Whenever an opponent casts a blue or black spell, you may put a +1/+1 counter on this creature.
#[test]
fn mold_adder_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("2091eb7e-2357-4ef8-b15e-1e6f82176340"),
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

// Nasty Little Rabbit — {G} — Creature — Rabbit
// Oracle: Ferocious — At the beginning of combat on your turn, if you control a creature with power 4 or greater, put a +1/+1 counter on this creature.
#[test]
fn nasty_little_rabbit_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("ee86cce6-c7c1-40a6-896b-cde9b86bb532"),
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

// Night Market Lookout — {B} — Creature — Human Rogue
// Oracle: Whenever this creature becomes tapped, each opponent loses 1 life and you gain 1 life.
#[test]
fn night_market_lookout_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("5f5c5f59-1cc0-4a72-863c-c26db411f114"),
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

// Novice Inspector — {W} — Creature — Human Detective
// Oracle: When this creature enters, investigate. (Create a Clue token. It's an artifact with "{2}, Sacrifice this token: Draw a card.")
#[test]
fn novice_inspector_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("c206bdcb-d2c4-4bda-a979-6c0627ef8f73"),
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

// Overwhelmed Apprentice — {U} — Creature — Human Wizard
// Oracle: When this creature enters, each opponent mills two cards. Then you scry 2. (Look at the top two cards of your library, then put any number of them on the bottom and the rest on top in any order.)
#[test]
fn overwhelmed_apprentice_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("e68d6459-2f32-4320-a062-6d33f85348f9"),
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
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Pious Wayfarer — {W} — Creature — Human Scout
// Oracle: Constellation — Whenever an enchantment you control enters, target creature gets +1/+1 until end of turn.
#[test]
fn pious_wayfarer_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("8790c7be-c14a-4293-bc4a-3d0efe42ed79"),
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

// Pulse Tracker — {B} — Creature — Vampire Rogue
// Oracle: Whenever this creature attacks, each opponent loses 1 life.
#[test]
fn pulse_tracker_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("f916f689-a314-4952-8107-41c4c2f2e811"),
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

// Ruin Crab — {U} — Creature — Crab
// Oracle: Landfall — Whenever a land you control enters, each opponent mills three cards. (To mill a card, a player puts the top card of their library into their graveyard.)
#[test]
fn ruin_crab_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("8afc00d4-a1c6-4329-af2c-a7f58a0c33e7"),
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
        base_pt: (0, 3),
        expected_pt: Some((0, 3)),
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

// Sage of Epityr — {U} — Creature — Human Wizard
// Oracle: When this creature enters, look at the top four cards of your library, then put them back in any order.
#[test]
fn sage_of_epityr_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("ef893585-246c-4028-bd2b-14ac51ce29be"),
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

// Sage of Mysteries — {U} — Creature — Human Wizard
// Oracle: Constellation — Whenever an enchantment you control enters, target player mills two cards.
#[test]
fn sage_of_mysteries_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("62c949c8-0011-4ab7-8388-e643be79df83"),
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
        base_pt: (0, 2),
        expected_pt: Some((0, 2)),
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

// Sawblade Scamp — {R} — Creature — Phyrexian Beast
// Oracle: Haste / Whenever you cast a noncreature spell, put an oil counter on this creature. / {T}, Remove an oil counter from this creature: It deals 1 damage to each opponent.
#[test]
fn sawblade_scamp_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("8cac286c-1a17-4ee1-9858-d7a2f8c09037"),
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
        expected_pt: None,
        enters_tapped: false,
        etb_changes_tapped: true,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::HASTE,
        has_enters_other: false,
        has_spell_cast: true,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Sazh's Chocobo — {G} — Creature — Bird
// Oracle: Landfall — Whenever a land you control enters, put a +1/+1 counter on this creature.
#[test]
fn sazh_s_chocobo_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("c2b082b4-5d9c-4125-b353-d1cf4f334720"),
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
        base_pt: (0, 1),
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

// Scute Mob — {G} — Creature — Insect
// Oracle: At the beginning of your upkeep, if you control five or more lands, put four +1/+1 counters on this creature.
#[test]
fn scute_mob_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("f1e0eda0-4a5e-4f1f-8131-ba58a58887cf"),
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

// Scythe Leopard — {G} — Creature — Cat
// Oracle: Landfall — Whenever a land you control enters, this creature gets +1/+1 until end of turn.
#[test]
fn scythe_leopard_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("ab539b16-d2a0-404d-8ad3-7ff33f2f1abe"),
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

// Scythe Tiger — {G} — Creature — Cat
// Oracle: Shroud (This creature can't be the target of spells or abilities.) / When this creature enters, sacrifice it unless you sacrifice a land.
#[test]
fn scythe_tiger_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("4d0fa514-87fc-46ff-bf49-4cd332396978"),
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
        base_pt: (3, 2),
        expected_pt: Some((3, 2)),
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::SHROUD,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Serrated Scorpion — {B} — Creature — Scorpion
// Oracle: When this creature dies, it deals 2 damage to each opponent and you gain 2 life.
#[test]
fn serrated_scorpion_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("abbdc656-02e9-43a0-8e2c-263d6c37c273"),
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

// Shambling Goblin — {B} — Creature — Zombie Goblin
// Oracle: When this creature dies, target creature an opponent controls gets -1/-1 until end of turn.
#[test]
fn shambling_goblin_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("9d8c39f2-ddb5-4b79-861a-e1e25f447c29"),
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
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Shrieking Drake — {U} — Creature — Drake
// Oracle: Flying / When this creature enters, return a creature you control to its owner's hand.
#[test]
fn shrieking_drake_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("2a8e14be-6f93-428d-a204-e2399dc93797"),
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

// Slumbering Keepguard — {W} — Creature — Human Knight
// Oracle: Whenever an enchantment you control enters, scry 1. / {2}{W}: This creature gets +1/+1 until end of turn for each enchantment you control.
#[test]
fn slumbering_keepguard_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("89078485-c438-4035-95dc-dbe37fb13e50"),
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

// Snarling Gorehound — {B} — Creature — Dog
// Oracle: Menace / Whenever another creature you control with power 2 or less enters, surveil 1. (Look at the top card of your library. You may put it into your graveyard.)
#[test]
fn snarling_gorehound_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("d2b1c5eb-06f0-4ec4-9446-791faac099a7"),
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
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::MENACE,
        has_enters_other: true,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Soul's Attendant — {W} — Creature — Human Cleric
// Oracle: Whenever another creature enters, you may gain 1 life.
#[test]
fn soul_s_attendant_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("045a9d3d-c20d-427f-a77b-0bffc6f55526"),
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

// Spindrift Drake — {U} — Creature — Drake
// Oracle: Flying / At the beginning of your upkeep, sacrifice this creature unless you pay {U}.
#[test]
fn spindrift_drake_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("53599f89-1598-4d3d-8aa7-75b397b258eb"),
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

// Steppe Lynx — {W} — Creature — Cat
// Oracle: Landfall — Whenever a land you control enters, this creature gets +2/+2 until end of turn.
#[test]
fn steppe_lynx_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("b16cef02-dcf8-4866-ac42-6ee929ddbea2"),
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
        base_pt: (0, 1),
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
