//! Package 6 triggered creature behavioral tests — part 08.

#[allow(clippy::wildcard_imports)]
use super::*;

// Horned Kavu — {R}{G} — Creature — Kavu
// Oracle: When this creature enters, return a red or green creature you control to its owner's hand.
#[test]
fn horned_kavu_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("14817050-9f2d-453f-b36b-e6429fa29165"),
        primary_mana: &[(ManaColor::Red, 1), (ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red, Color::Green],
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
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Iceberg Cancrix — {1}{U} — Snow Creature — Crab
// Oracle: Whenever another snow permanent you control enters, you may have target player mill two cards.
#[test]
fn iceberg_cancrix_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("9e00188a-0743-4a39-9229-977a0a115659"),
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
        is_snow: true,
        base_pt: (0, 4),
        expected_pt: Some((0, 4)),
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

// Il Mheg Pixie — {1}{U} — Creature — Faerie
// Oracle: Flying / Whenever this creature attacks, surveil 1. (Look at the top card of your library. You may put it into your graveyard.)
#[test]
fn il_mheg_pixie_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("c2c47e13-b581-41a8-a4af-542557a6249a"),
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

// Impassioned Orator — {1}{W} — Creature — Human Cleric
// Oracle: Whenever another creature you control enters, you gain 1 life.
#[test]
fn impassioned_orator_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("c7107a2d-2dcf-42e9-9ea9-d0fc0d6d2ec6"),
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
        has_enters_other: true,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Initiate's Companion — {1}{G} — Creature — Cat
// Oracle: Whenever this creature deals combat damage to a player, untap target creature or land.
#[test]
fn initiate_s_companion_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("f3264224-43a4-4c6b-b363-2d147687dfc1"),
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
        base_pt: (3, 1),
        expected_pt: Some((3, 1)),
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

// Invasion Reinforcements — {1}{W} — Creature — Human Warrior Ally
// Oracle: Flash / When this creature enters, create a 1/1 white Ally creature token.
#[test]
fn invasion_reinforcements_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("185316ff-6fb7-4b5c-8a60-3238674562a6"),
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

// Ironpaw Aspirant — {1}{W} — Creature — Cat Warrior
// Oracle: When this creature enters, put a +1/+1 counter on target creature.
#[test]
fn ironpaw_aspirant_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("fab5f694-4a9c-4e72-b164-f236e8212ff7"),
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

// Ishi-Ishi, Akki Crackshot — {1}{R} — Legendary Creature — Goblin Warrior
// Oracle: Whenever an opponent casts a Spirit or Arcane spell, Ishi-Ishi deals 2 damage to that player.
#[test]
fn ishi_ishi_akki_crackshot_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("30265fce-28e6-4994-83f7-0e4c4b22df35"),
        primary_mana: &[(ManaColor::Red, 1), (ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: true,
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

// Jeong Jeong's Deserters — {1}{W} — Creature — Human Rebel Ally
// Oracle: When this creature enters, put a +1/+1 counter on target creature.
#[test]
fn jeong_jeong_s_deserters_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("6a06d281-3ca2-498f-91d0-e13e7b0869f9"),
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

// Jeskai Barricade — {1}{W} — Creature — Wall
// Oracle: Flash (You may cast this spell any time you could cast an instant.) / Defender / When this creature enters, you may return another target creature you control to its owner's hand.
#[test]
fn jeskai_barricade_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("7a2fa0a6-1324-4714-8896-6bb97640578a"),
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
        base_pt: (0, 4),
        expected_pt: Some((0, 4)),
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FLASH.union(KeywordSet::DEFENDER),
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Jeskai Sage — {1}{U} — Creature — Human Monk
// Oracle: Prowess (Whenever you cast a noncreature spell, this creature gets +1/+1 until end of turn.) / When this creature dies, draw a card.
#[test]
fn jeskai_sage_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("b79d4b17-482e-415b-a63b-1f97a55e7583"),
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
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::PROWESS,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Judge Magister Gabranth — {W}{B} — Legendary Creature — Human Advisor Knight
// Oracle: Menace (This creature can't be blocked except by two or more creatures.) / Whenever another creature or artifact you control dies, put a +1/+1 counter on Judge Magister Gabranth.
#[test]
fn judge_magister_gabranth_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("1a846ad6-15c9-4bf9-9127-612018a557b9"),
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
        expected_pt: None,
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::MENACE,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Judge of Currents — {1}{W} — Creature — Merfolk Wizard
// Oracle: Whenever a Merfolk you control becomes tapped, you may gain 1 life.
#[test]
fn judge_of_currents_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("b4eafd12-5f3e-43e6-aa0a-6bb5f4f5782b"),
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

// Kalastria Healer — {1}{B} — Creature — Vampire Cleric Ally
// Oracle: Rally — Whenever this creature or another Ally you control enters, each opponent loses 1 life and you gain 1 life.
#[test]
fn kalastria_healer_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("b70718c3-a959-402a-a715-7a3c9724a70a"),
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
        base_pt: (1, 2),
        expected_pt: Some((1, 2)),
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

// Kalastria Highborn — {B}{B} — Creature — Vampire Shaman
// Oracle: Whenever this creature or another Vampire you control dies, you may pay {B}. If you do, target player loses 2 life and you gain 2 life.
#[test]
fn kalastria_highborn_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("77e319da-d782-4258-bf0e-626755e5adc9"),
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

// Kami of the Crescent Moon — {U}{U} — Legendary Creature — Spirit
// Oracle: At the beginning of each player's draw step, that player draws an additional card.
#[test]
fn kami_of_the_crescent_moon_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("38cfe4a0-6ead-435b-9f86-928205c0a52c"),
        primary_mana: &[(ManaColor::Blue, 2)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: true,
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

// Kazandu Nectarpot — {1}{G} — Creature — Insect
// Oracle: Landfall — Whenever a land you control enters, you gain 1 life.
#[test]
fn kazandu_nectarpot_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("263b526c-8b81-44ee-a7c4-5c24bf9b42a4"),
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
        base_pt: (1, 3),
        expected_pt: Some((1, 3)),
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

// Kessig Flamebreather — {1}{R} — Creature — Human Shaman
// Oracle: Whenever you cast a noncreature spell, this creature deals 1 damage to each opponent.
#[test]
fn kessig_flamebreather_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("eadd9559-6dcb-4b96-8c95-58abddd0e120"),
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

// Kiln Fiend — {1}{R} — Creature — Elemental Beast
// Oracle: Whenever you cast an instant or sorcery spell, this creature gets +3/+0 until end of turn.
#[test]
fn kiln_fiend_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("eac8c196-8477-4b79-9875-21afa1e61708"),
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
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: false,
        has_spell_cast: true,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Kindly Customer — {1}{W} — Creature — Human Citizen
// Oracle: When this creature enters, draw a card.
#[test]
fn kindly_customer_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("3faab84f-7055-4554-b948-181928d8d4e2"),
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

// Kinsbaile Skirmisher — {1}{W} — Creature — Kithkin Soldier
// Oracle: When this creature enters, target creature gets +1/+1 until end of turn.
#[test]
fn kinsbaile_skirmisher_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("6b142a4a-769b-4e1b-aae6-ba6e11f35836"),
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

// Knotvine Paladin — {G}{W} — Creature — Human Knight
// Oracle: Whenever this creature attacks, it gets +1/+1 until end of turn for each untapped creature you control.
#[test]
fn knotvine_paladin_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("75efff64-ea9b-4e3e-a729-fc003ee3db1e"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Green],
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

// Kor Firewalker — {W}{W} — Creature — Kor Soldier
// Oracle: Protection from red / Whenever a player casts a red spell, you may gain 1 life.
#[test]
fn kor_firewalker_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("fb37ff1c-7461-4a16-b27f-6eef0db937fe"),
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
        base_pt: (2, 2),
        expected_pt: Some((2, 2)),
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

// Kor Skyfisher — {1}{W} — Creature — Kor Soldier
// Oracle: Flying / When this creature enters, return a permanent you control to its owner's hand.
#[test]
fn kor_skyfisher_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("054a0193-861e-4644-ab0a-8aa33407abc8"),
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
        base_pt: (2, 3),
        expected_pt: Some((2, 3)),
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

// Kozilek's Sentinel — {1}{R} — Creature — Eldrazi Drone
// Oracle: Devoid (This card has no color.) / Whenever you cast a colorless spell, this creature gets +1/+0 until end of turn.
#[test]
fn kozilek_s_sentinel_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("86ae05d6-f1ba-456d-bbc0-f35cd0d0537d"),
        primary_mana: &[(ManaColor::Red, 1), (ManaColor::Colorless, 1)],
        alt_mana: None,
        total_mana: 2,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[],
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
        expected_keywords: KeywordSet::DEVOID,
        has_enters_other: false,
        has_spell_cast: true,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Kraul Whipcracker — {B}{G} — Creature — Insect Assassin
// Oracle: Reach / When this creature enters, destroy target token an opponent controls.
#[test]
fn kraul_whipcracker_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("b563ae31-338e-4bcc-a05f-d7ed7c3fe46e"),
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::Green, 1)],
        alt_mana: None,
        total_mana: 2,
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
        expected_keywords: KeywordSet::REACH,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Kronch Wrangler — {1}{G} — Creature — Human Warrior
// Oracle: Trample / Whenever a creature you control with power 4 or greater enters, put a +1/+1 counter on this creature.
#[test]
fn kronch_wrangler_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("635dda8c-3761-4c87-8723-8cd09878516b"),
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

// Kruin Striker — {1}{R} — Creature — Human Warrior
// Oracle: Whenever another creature you control enters, this creature gets +1/+0 and gains trample until end of turn. (It can deal excess combat damage to the player or planeswalker it's attacking.)
#[test]
fn kruin_striker_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("3e261fa8-1256-4525-8115-5880758338fe"),
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

// Kujar Seedsculptor — {1}{G} — Creature — Elf Druid
// Oracle: When this creature enters, put a +1/+1 counter on target creature you control.
#[test]
fn kujar_seedsculptor_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("93f817c7-2a3d-4346-94c1-3ba8bafb485d"),
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

// Leaf-Crowned Visionary — {G}{G} — Creature — Elf Druid
// Oracle: Other Elves you control get +1/+1. / Whenever you cast an Elf spell, you may pay {G}. If you do, draw a card.
#[test]
fn leaf_crowned_visionary_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("ef693b9f-11e3-49bf-8387-b8f480b9007a"),
        primary_mana: &[(ManaColor::Green, 2)],
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
