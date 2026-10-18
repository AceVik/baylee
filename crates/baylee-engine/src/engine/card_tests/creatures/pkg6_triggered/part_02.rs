//! Package 6 triggered creature behavioral tests — part 02.

#[allow(clippy::wildcard_imports)]
use super::*;

// Champion of the Parish — {W} — Creature — Human Soldier
// Oracle: Whenever another Human you control enters, put a +1/+1 counter on this creature.
#[test]
fn champion_of_the_parish_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("faeaa3eb-6eb8-4621-98c7-0c6f58bbff85"),
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

// Champion of the Perished — {B} — Creature — Zombie
// Oracle: Whenever another Zombie you control enters, put a +1/+1 counter on this creature.
#[test]
fn champion_of_the_perished_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("f3654fbd-16a5-4953-84ac-534e8421032f"),
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
        has_enters_other: true,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Deathgreeter — {B} — Creature — Human Shaman
// Oracle: Whenever another creature dies, you may gain 1 life.
#[test]
fn deathgreeter_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("b7ad11dc-c962-4034-b15a-a080c6fff75f"),
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
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Doomed Traveler — {W} — Creature — Human Soldier
// Oracle: When this creature dies, create a 1/1 white Spirit creature token with flying.
#[test]
fn doomed_traveler_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("a30907c0-fbde-4fd3-a8c7-f304305fcea7"),
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
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Dragonmaster Outcast — {R} — Creature — Human Shaman
// Oracle: At the beginning of your upkeep, if you control six or more lands, create a 5/5 red Dragon creature token with flying.
#[test]
fn dragonmaster_outcast_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("b6fb79c3-cd32-4045-8177-e52841eea65b"),
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

// Drainpipe Vermin — {B} — Creature — Rat
// Oracle: When this creature dies, you may pay {B}. If you do, target player discards a card.
#[test]
fn drainpipe_vermin_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("b6d39905-e074-4517-b806-d081d1069f74"),
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
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Dream Beavers — {B} — Creature — Beaver Nightmare
// Oracle: Flying / When this creature enters, each opponent loses 1 life and you gain 1 life. Scry 1. (Look at the top card of your library. You may put that card on the bottom.)
#[test]
fn dream_beavers_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("a1cec8e8-cf3c-42a4-b625-ca0cd148664e"),
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
        expected_keywords: KeywordSet::FLYING,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Drifter il-Dal — {U} — Creature — Human Wizard
// Oracle: Shadow (This creature can block or be blocked by only creatures with shadow.) / At the beginning of your upkeep, sacrifice this creature unless you pay {U}.
#[test]
fn drifter_il_dal_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("dbe8ad46-d862-4ff6-bf17-238fb8341075"),
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
        expected_keywords: KeywordSet::SHADOW,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Drowner Initiate — {U} — Creature — Merfolk Wizard
// Oracle: Whenever a player casts a blue spell, you may pay {1}. If you do, target player mills two cards.
#[test]
fn drowner_initiate_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("19a4010f-f5bf-48be-a1d7-eacc3bbeeb59"),
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
        has_spell_cast: true,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Elvish Handservant — {G} — Creature — Elf Warrior
// Oracle: Whenever a player casts a Giant spell, you may put a +1/+1 counter on this creature.
#[test]
fn elvish_handservant_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("16fb84e1-0bc7-469c-bd73-40747be56d9e"),
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

// Essence Warden — {G} — Creature — Elf Shaman
// Oracle: Whenever another creature enters, you gain 1 life.
#[test]
fn essence_warden_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("6ca2a89e-7032-4864-b4e9-66f3178f90ab"),
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

// Eye Collector — {B} — Creature — Faerie
// Oracle: Flying / Whenever this creature deals combat damage to a player, each player mills a card. (Each player puts the top card of their library into their graveyard.)
#[test]
fn eye_collector_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("553c2006-e123-406e-ab4a-1d1f67c7468c"),
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
        expected_keywords: KeywordSet::FLYING,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Faerie Seer — {U} — Creature — Faerie Wizard
// Oracle: Flying / When this creature enters, scry 2. (Look at the top two cards of your library, then put any number of them on the bottom and the rest on top in any order.)
#[test]
fn faerie_seer_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("b2e65e8b-5f08-4cc2-ab1d-00f8903dbea2"),
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

// Festerleech — {B} — Creature — Zombie Leech
// Oracle: Whenever this creature deals combat damage to a player, you mill two cards. / {1}{B}: This creature gets +2/+2 until end of turn. Activate only once each turn.
#[test]
fn festerleech_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("d14fdc1d-62bc-4e9c-b79b-8c9c3232f0bc"),
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
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Footlight Fiend — {B/R} — Creature — Devil
// Oracle: When this creature dies, it deals 1 damage to any target.
#[test]
fn footlight_fiend_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("77a5305a-2611-4a64-adb4-da131dcea233"),
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

// Foundry Street Denizen — {R} — Creature — Goblin Warrior
// Oracle: Whenever another red creature you control enters, this creature gets +1/+0 until end of turn.
#[test]
fn foundry_street_denizen_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("e8148616-9f0b-4d4e-8d6d-18251a84e5e8"),
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

// Fourth Bridge Prowler — {B} — Creature — Human Rogue
// Oracle: When this creature enters, you may have target creature get -1/-1 until end of turn.
#[test]
fn fourth_bridge_prowler_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("da18d97c-85f3-4395-9af5-bca72e33ebfc"),
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
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Generous Visitor — {G} — Creature — Spirit
// Oracle: Whenever you cast an enchantment spell, put a +1/+1 counter on target creature.
#[test]
fn generous_visitor_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("0629747e-8dd8-4150-95ee-9d860264ccb0"),
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

// Gilded Goose — {G} — Creature — Bird
// Oracle: Flying / When this creature enters, create a Food token. (It's an artifact with "{2}, {T}, Sacrifice this token: You gain 3 life.") / {1}{G}, {T}: Create a Food token. / {T}, Sacrifice a Food: Add one mana of any color.
#[test]
fn gilded_goose_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("f2f09757-1931-47c0-a5f0-39280445489d"),
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
        base_pt: (0, 2),
        expected_pt: Some((0, 2)),
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

// Goblin Arsonist — {R} — Creature — Goblin Shaman
// Oracle: When this creature dies, you may have it deal 1 damage to any target.
#[test]
fn goblin_arsonist_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("c1177f22-a1cf-4da3-a68d-ff954e878403"),
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

// Goblins of the Flarg — {R} — Creature — Goblin Warrior
// Oracle: Mountainwalk (This creature can't be blocked as long as defending player controls a Mountain.) / When you control a Dwarf, sacrifice this creature.
#[test]
fn goblins_of_the_flarg_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("3a620d92-ec92-4733-8c05-7fa1b956bab6"),
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
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::MOUNTAINWALK,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// God-Pharaoh's Faithful — {W} — Creature — Human Wizard
// Oracle: Whenever you cast a blue, black, or red spell, you gain 1 life.
#[test]
fn god_pharaoh_s_faithful_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("3f86e6c6-fca5-43e9-88cc-4546cdd5f0ec"),
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
        base_pt: (0, 4),
        expected_pt: Some((0, 4)),
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

// Greedy Freebooter — {B} — Creature — Human Pirate
// Oracle: When this creature dies, scry 1 and create a Treasure token. (To scry 1, look at the top card of your library. You may put that card on the bottom. A Treasure token is an artifact with "{T}, Sacrifice this token: Add one mana of any color.")
#[test]
fn greedy_freebooter_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("d4a9d63a-5a8a-4f51-9e49-43d6dc3234b1"),
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
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Grim Physician — {B} — Creature — Zombie
// Oracle: When this creature dies, target creature an opponent controls gets -1/-1 until end of turn.
#[test]
fn grim_physician_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("381539ce-fc05-4855-80da-b0807a7c07c2"),
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

// Hada Freeblade — {W} — Creature — Human Soldier Ally
// Oracle: Whenever this creature or another Ally you control enters, you may put a +1/+1 counter on this creature.
#[test]
fn hada_freeblade_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("e9ad9df4-2fd6-4a22-b861-0a8d28f2fe10"),
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

// Harrier Strix — {U} — Creature — Bird
// Oracle: Flying / When this creature enters, tap target permanent. / {2}{U}: Draw a card, then discard a card.
#[test]
fn harrier_strix_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("eb154ebb-d02f-405a-a557-3d5ec7648c42"),
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

// Hedron Crab — {U} — Creature — Crab
// Oracle: Landfall — Whenever a land you control enters, target player mills three cards. (They put the top three cards of their library into their graveyard.)
#[test]
fn hedron_crab_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("7216f974-3c84-40ef-b904-82019900c204"),
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

// Humbling Elder — {U} — Creature — Human Monk
// Oracle: Flash / When this creature enters, target creature an opponent controls gets -2/-0 until end of turn.
#[test]
fn humbling_elder_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("013bcab8-e527-4700-82c9-4cb9cb8a4807"),
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
        expected_pt: None,
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

// Hunted Witness — {W} — Creature — Human
// Oracle: When this creature dies, create a 1/1 white Soldier creature token with lifelink.
#[test]
fn hunted_witness_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("1fde570c-aa96-45ca-a6be-f59c8e4f4b6a"),
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
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Infestation Sage — {B} — Creature — Elf Warlock
// Oracle: When this creature dies, create a 1/1 black and green Insect creature token with flying.
#[test]
fn infestation_sage_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("d398dca3-f3a5-44c9-998f-b4fce9e5c2bf"),
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
        has_dies_trigger: true,
        cycling_mana: None,
    });
}
