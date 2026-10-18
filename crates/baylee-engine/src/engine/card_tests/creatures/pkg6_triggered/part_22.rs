//! Package 6 triggered creature behavioral tests — part 22.

#[allow(clippy::wildcard_imports)]
use super::*;

// Sage's Row Denizen — {2}{U} — Creature — Vedalken Wizard
// Oracle: Whenever another blue creature you control enters, target player mills two cards.
#[test]
fn sage_s_row_denizen_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("fc59aa9a-3abd-4479-a63e-ef0eeb35b5f2"),
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
        base_pt: (2, 3),
        expected_pt: Some((2, 3)),
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

// Sailor of Means — {2}{U} — Creature — Human Pirate
// Oracle: When this creature enters, create a Treasure token. (It's an artifact with "{T}, Sacrifice this token: Add one mana of any color.")
#[test]
fn sailor_of_means_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("c4efae03-99de-4439-925b-504602623bfd"),
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

// Salivating Gremlins — {2}{R} — Creature — Gremlin
// Oracle: Whenever an artifact you control enters, this creature gets +2/+0 and gains trample until end of turn.
#[test]
fn salivating_gremlins_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("4766f232-8275-4256-b756-8dd5c5e825a3"),
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
        base_pt: (2, 3),
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

// Sanity Gnawers — {1}{B}{R} — Creature — Rat
// Oracle: When this creature enters, target player discards a card at random.
#[test]
fn sanity_gnawers_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("e82308d5-29ee-48eb-9d1c-f24897ef191c"),
        primary_mana: &[
            (ManaColor::Black, 1),
            (ManaColor::Red, 1),
            (ManaColor::Colorless, 1),
        ],
        alt_mana: None,
        total_mana: 3,
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
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Savior of the Sleeping — {2}{W} — Creature — Human Knight
// Oracle: Vigilance / Whenever an enchantment you control is put into a graveyard from the battlefield, put a +1/+1 counter on this creature.
#[test]
fn savior_of_the_sleeping_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("412eb5ec-bf13-45fb-94b3-c41e62b19d1d"),
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
        base_pt: (2, 3),
        expected_pt: None,
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::VIGILANCE,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Scion Summoner — {2}{G} — Creature — Eldrazi Drone
// Oracle: Devoid (This card has no color.) / When this creature enters, create a 1/1 colorless Eldrazi Scion creature token. It has "Sacrifice this token: Add {C}." ({C} represents colorless mana.)
#[test]
fn scion_summoner_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("fa60f3a6-5834-4059-adea-dd17366b7885"),
        primary_mana: &[(ManaColor::Green, 1), (ManaColor::Colorless, 2)],
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
        base_pt: (2, 2),
        expected_pt: Some((2, 2)),
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::DEVOID,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Screaming Phantom — {2}{B} — Creature — Spirit
// Oracle: Flying / Whenever this creature attacks, mill a card. (Put the top card of your library into your graveyard.)
#[test]
fn screaming_phantom_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("d32f570a-3738-4a9d-845f-6c7892b69991"),
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
        expected_keywords: KeywordSet::FLYING,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Screeching Soulbreaker — {2}{B} — Creature — Siren Bard
// Oracle: Flying / Whenever this creature attacks, it deals 1 damage to each opponent and you gain 1 life.
#[test]
fn screeching_soulbreaker_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("47e33aa1-2f54-471c-a484-de13bcd2dceb"),
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
        base_pt: (1, 4),
        expected_pt: Some((1, 4)),
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

// Scroll Thief — {2}{U} — Creature — Merfolk Rogue
// Oracle: Whenever this creature deals combat damage to a player, draw a card.
#[test]
fn scroll_thief_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("637c5583-4683-4ae4-8b4e-f5da42a772c7"),
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

// Sedraxis Alchemist — {2}{B} — Creature — Zombie Wizard
// Oracle: When this creature enters, if you control a blue permanent, return target nonland permanent to its owner's hand.
#[test]
fn sedraxis_alchemist_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("56f3f1b4-c36a-41f3-95a1-45e9d7e74cf9"),
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

// Selhoff Occultist — {2}{U} — Creature — Human Rogue
// Oracle: Whenever this creature or another creature dies, target player mills a card.
#[test]
fn selhoff_occultist_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("d72031e1-c8cf-4e57-b15b-6a5eaacff1e2"),
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
        base_pt: (2, 3),
        expected_pt: Some((2, 3)),
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

// Selkie Hedge-Mage — {2}{G/U} — Creature — Merfolk Wizard
// Oracle: When this creature enters, if you control two or more Forests, you may gain 3 life. / When this creature enters, if you control two or more Islands, you may return target tapped creature to its owner's hand.
#[test]
fn selkie_hedge_mage_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("1f1c369a-3209-405b-9880-0e10b86e5698"),
        primary_mana: &[(ManaColor::Green, 1), (ManaColor::Colorless, 2)],
        alt_mana: Some(&[(ManaColor::Blue, 1), (ManaColor::Colorless, 2)]),
        total_mana: 3,
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

// Seller of Songbirds — {2}{W} — Creature — Human
// Oracle: When this creature enters, create a 1/1 white Bird creature token with flying.
#[test]
fn seller_of_songbirds_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("87e4c6ff-f83f-412b-9d23-aac7a57ef6db"),
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

// Serum Visionary — {2}{U} — Creature — Vedalken Wizard
// Oracle: When this creature enters, draw a card, then scry 2.
#[test]
fn serum_visionary_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("922d9d79-8038-4185-9dfb-2d2d52fd8460"),
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

// Setessan Champion — {2}{G} — Creature — Human Warrior
// Oracle: Constellation — Whenever an enchantment you control enters, put a +1/+1 counter on this creature and draw a card.
#[test]
fn setessan_champion_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("38513660-260f-4663-b199-6473fa004a3b"),
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
        base_pt: (1, 3),
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

// Settlement Blacksmith — {2}{W} — Creature — Human Artificer
// Oracle: When this creature enters, if you control an Equipment, draw a card.
#[test]
fn settlement_blacksmith_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("f98d0f8a-c8b5-4fb8-b598-e70ffab3c0ff"),
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

// Shadewing Laureate — {W}{W/B}{B} — Creature — Human Warlock
// Oracle: Flying / Whenever another creature you control with flying dies, put a +1/+1 counter on target creature you control.
#[test]
fn shadewing_laureate_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("53a17d4f-cdbc-4669-a30e-5d5a334cc3c0"),
        primary_mana: &[(ManaColor::White, 2), (ManaColor::Black, 1)],
        alt_mana: Some(&[(ManaColor::White, 1), (ManaColor::Black, 2)]),
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
        expected_pt: None,
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FLYING,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Shadow Stinger — {2}{B} — Creature — Vampire Rogue
// Oracle: Tap another untapped Rogue you control: This creature gains deathtouch until end of turn. / Whenever this creature deals combat damage to a player, that player mills three cards. (They put the top three cards of their library into their graveyard.)
#[test]
fn shadow_stinger_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("214562b1-fdf0-4d5d-93f0-594b31b80bb3"),
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
        base_pt: (1, 4),
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

// Shadowmage Infiltrator — {1}{U}{B} — Creature — Human Wizard
// Oracle: Fear (This creature can't be blocked except by artifact creatures and/or black creatures.) / Whenever this creature deals combat damage to a player, you may draw a card.
#[test]
fn shadowmage_infiltrator_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("3a6f886b-2043-47e9-9c0f-f7913a6fa67d"),
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
        base_pt: (1, 3),
        expected_pt: Some((1, 3)),
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FEAR,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Shaman of the Pack — {1}{B}{G} — Creature — Elf Shaman
// Oracle: When this creature enters, target opponent loses life equal to the number of Elves you control.
#[test]
fn shaman_of_the_pack_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("39c7d2cc-4fd1-4f1a-be1a-a6ae56d2356d"),
        primary_mana: &[
            (ManaColor::Black, 1),
            (ManaColor::Green, 1),
            (ManaColor::Colorless, 1),
        ],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black, Color::Green],
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
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Shinra Reinforcements — {2}{B} — Creature — Human Soldier
// Oracle: When this creature enters, mill three cards and you gain 3 life. (To mill three cards, put the top three cards of your library into your graveyard.)
#[test]
fn shinra_reinforcements_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("6cfe63ac-3581-461b-8ed3-3e231851e3f8"),
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
        base_pt: (2, 3),
        expected_pt: Some((2, 3)),
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

// Shopkeeper's Bane — {2}{G} — Creature — Badger Pest
// Oracle: Trample / Whenever this creature attacks, you gain 2 life.
#[test]
fn shopkeeper_s_bane_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("e7572317-6bec-4fb0-81ac-76aec04f14ad"),
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

// Sibsig Icebreakers — {2}{B} — Creature — Zombie
// Oracle: When this creature enters, each player discards a card.
#[test]
fn sibsig_icebreakers_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("4880c976-b417-48e4-98a2-9e5a8da0d758"),
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
        base_pt: (2, 3),
        expected_pt: Some((2, 3)),
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

// Siege Veteran — {2}{W} — Creature — Human Soldier
// Oracle: At the beginning of combat on your turn, put a +1/+1 counter on target creature you control. / Whenever another nontoken Soldier you control dies, create a 1/1 colorless Soldier artifact creature token.
#[test]
fn siege_veteran_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("4d4fc571-a859-4460-a3c7-3f76a671ab1b"),
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

// Silt Crawler — {2}{G} — Creature — Beast
// Oracle: When this creature enters, tap all lands you control.
#[test]
fn silt_crawler_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("5ba1595a-6578-459d-96fe-fdfb84ac34d5"),
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
        expected_pt: Some((3, 3)),
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

// Silver Drake — {1}{W}{U} — Creature — Drake
// Oracle: Flying / When this creature enters, return a white or blue creature you control to its owner's hand.
#[test]
fn silver_drake_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("ec7eb7d8-92b2-4442-856e-a9790301c3b3"),
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

// Silvergill Peddler — {2}{U} — Creature — Merfolk Citizen
// Oracle: Whenever this creature becomes tapped, draw a card, then discard a card.
#[test]
fn silvergill_peddler_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("08688903-d9cc-4d57-ba91-14a8523551b9"),
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
        base_pt: (2, 3),
        expected_pt: Some((2, 3)),
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

// Skirmish Rhino — {W}{B}{G} — Creature — Rhino
// Oracle: Trample / When this creature enters, each opponent loses 2 life and you gain 2 life.
#[test]
fn skirmish_rhino_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("40eb7f43-7cee-41a1-a4d1-f34ee125ccd1"),
        primary_mana: &[
            (ManaColor::White, 1),
            (ManaColor::Black, 1),
            (ManaColor::Green, 1),
        ],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Black, Color::Green],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (3, 4),
        expected_pt: Some((3, 4)),
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

// Skull Collector — {1}{B}{B} — Creature — Ogre Warrior
// Oracle: At the beginning of your upkeep, return a black creature you control to its owner's hand. / {1}{B}: Regenerate this creature.
#[test]
fn skull_collector_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("42ec62b1-c899-4ebd-b786-28edf76ce8e0"),
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

// Skyclave Geopede — {2}{R} — Creature — Insect
// Oracle: Trample / Landfall — Whenever a land you control enters, this creature gets +2/+2 until end of turn.
#[test]
fn skyclave_geopede_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("e00d601f-ac73-47a6-8920-779d71b482c9"),
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
        base_pt: (3, 1),
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
