//! Package 6 triggered creature behavioral tests — part 38.

#[allow(clippy::wildcard_imports)]
use super::*;

// Heavy Infantry — {4}{W} — Creature — Human Soldier
// Oracle: When this creature enters, tap target creature an opponent controls.
#[test]
fn heavy_infantry_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("d07707ef-fa4e-423f-8a1f-095d19100321"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (3, 4),
        expected_pt: Some((3, 4)),
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

// Herald of Faith — {3}{W}{W} — Creature — Angel
// Oracle: Flying / Whenever this creature attacks, you gain 2 life.
#[test]
fn herald_of_faith_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("cb97da84-1d13-4795-b68a-2bf111a50067"),
        primary_mana: &[(ManaColor::White, 2), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (4, 3),
        expected_pt: Some((4, 3)),
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

// Hero of the Dunes — {3}{W}{B} — Creature — Human Soldier
// Oracle: When this creature enters, return target artifact or creature card with mana value 3 or less from your graveyard to the battlefield. / Creatures you control with mana value 3 or less get +1/+0.
#[test]
fn hero_of_the_dunes_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("eb6f936e-8dba-49da-84fc-de240b11b8e4"),
        primary_mana: &[
            (ManaColor::White, 1),
            (ManaColor::Black, 1),
            (ManaColor::Colorless, 3),
        ],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Black],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (3, 2),
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

// Horizon Seed — {4}{W} — Creature — Spirit
// Oracle: Whenever you cast a Spirit or Arcane spell, regenerate target creature.
#[test]
fn horizon_seed_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("3970e021-209d-41d3-9466-8b3ef9ab1a4c"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 5,
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
        has_spell_cast: true,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Iceridge Serpent — {4}{U} — Creature — Serpent
// Oracle: When this creature enters, return target creature an opponent controls to its owner's hand.
#[test]
fn iceridge_serpent_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("53f07529-c212-43e4-b5b6-086efeadda39"),
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 5,
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

// Icewind Elemental — {4}{U} — Creature — Elemental
// Oracle: Flying / When this creature enters, draw a card, then discard a card.
#[test]
fn icewind_elemental_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("5001bf21-9dbb-4229-aeef-25805e71b832"),
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue],
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
        expected_keywords: KeywordSet::FLYING,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Imperial Ceratops — {4}{W} — Creature — Dinosaur
// Oracle: Enrage — Whenever this creature is dealt damage, you gain 2 life.
#[test]
fn imperial_ceratops_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("86cee7f0-0aac-46f1-8e4e-062fc398477e"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (3, 5),
        expected_pt: Some((3, 5)),
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

// Infested Roothold — {4}{G} — Creature — Wall
// Oracle: Defender (This creature can't attack.) / Protection from artifacts / Whenever an opponent casts an artifact spell, you may create a 1/1 green Insect creature token.
#[test]
fn infested_roothold_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("86ebaaf1-82eb-439e-bfca-68521ca199eb"),
        primary_mana: &[(ManaColor::Green, 1), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 5,
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
        has_enters_other: false,
        has_spell_cast: true,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Inkfathom Divers — {3}{U}{U} — Creature — Merfolk Soldier
// Oracle: Islandwalk (This creature can't be blocked as long as defending player controls an Island.) / When this creature enters, look at the top four cards of your library, then put them back in any order.
#[test]
fn inkfathom_divers_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("dbdf3c82-b0fc-479c-acce-f7e60be3a2b1"),
        primary_mana: &[(ManaColor::Blue, 2), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 5,
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
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::ISLANDWALK,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Izzet Chronarch — {3}{U}{R} — Creature — Human Wizard
// Oracle: When this creature enters, return target instant or sorcery card from your graveyard to your hand.
#[test]
fn izzet_chronarch_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("1da438f3-db1c-4713-a60c-e078f31d809c"),
        primary_mana: &[
            (ManaColor::Blue, 1),
            (ManaColor::Red, 1),
            (ManaColor::Colorless, 3),
        ],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue, Color::Red],
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

// Jeskai Shrinekeeper — {2}{U}{R}{W} — Creature — Dragon
// Oracle: Flying, haste / Whenever this creature deals combat damage to a player, you gain 1 life and draw a card.
#[test]
fn jeskai_shrinekeeper_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("bb50b4f4-f0da-4283-821b-6d74bd20752f"),
        primary_mana: &[
            (ManaColor::White, 1),
            (ManaColor::Blue, 1),
            (ManaColor::Red, 1),
            (ManaColor::Colorless, 2),
        ],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Blue, Color::Red],
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
        expected_keywords: KeywordSet::FLYING.union(KeywordSet::HASTE),
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Klingon Strike Force — {3}{R}{R} — Creature — Klingon Warrior
// Oracle: Trample (This creature can deal excess combat damage to the player it's attacking.) / Haste (This creature can attack and {T} as soon as it comes under your control.) / When this creature dies, it deals 2 damage to each opponent.
#[test]
fn klingon_strike_force_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("847efb61-5933-43c1-93ff-e8375f9469a9"),
        primary_mana: &[(ManaColor::Red, 2), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (5, 5),
        expected_pt: Some((5, 5)),
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::TRAMPLE.union(KeywordSet::HASTE),
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Knight-Captain of Eos — {4}{W} — Creature — Human Knight
// Oracle: When this creature enters, create two 1/1 white Soldier creature tokens. / {W}, Sacrifice a Soldier: Prevent all combat damage that would be dealt this turn.
#[test]
fn knight_captain_of_eos_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("611f1714-a8bb-4ce4-a810-4d26fe64e358"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 5,
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

// Knightfisher — {3}{U}{U} — Creature — Bird Knight
// Oracle: Flying / Whenever another nontoken Bird you control enters, create a 1/1 blue Fish creature token.
#[test]
fn knightfisher_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("cb2d222c-df42-41a7-a517-0a41e47a4280"),
        primary_mana: &[(ManaColor::Blue, 2), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (4, 5),
        expected_pt: Some((4, 5)),
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

// Kor Entanglers — {4}{W} — Creature — Kor Soldier Ally
// Oracle: Rally — Whenever this creature or another Ally you control enters, tap target creature an opponent controls.
#[test]
fn kor_entanglers_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("6fedd98c-9c58-4ca5-b77b-dcb4cd88d238"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (3, 4),
        expected_pt: Some((3, 4)),
        enters_tapped: false,
        etb_changes_tapped: true,
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

// Kurgadon — {4}{G} — Creature — Beast
// Oracle: Whenever you cast a creature spell with mana value 6 or greater, put three +1/+1 counters on this creature.
#[test]
fn kurgadon_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("6ce6aed2-b959-4572-96fd-fee340d5a5ee"),
        primary_mana: &[(ManaColor::Green, 1), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 5,
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
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: false,
        has_spell_cast: true,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Lightning Reaver — {3}{B}{R} — Creature — Zombie Beast
// Oracle: Haste; fear (This creature can't be blocked except by artifact creatures and/or black creatures.) / Whenever this creature deals combat damage to a player, put a charge counter on it. / At the beginning of your end step, this creature deals damage equal to the number of charge counters on it to each opponent.
#[test]
fn lightning_reaver_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("687abe1c-b2fc-49e0-bd36-3db3dc3a5039"),
        primary_mana: &[
            (ManaColor::Black, 1),
            (ManaColor::Red, 1),
            (ManaColor::Colorless, 3),
        ],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black, Color::Red],
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
        expected_keywords: KeywordSet::HASTE.union(KeywordSet::FEAR),
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Locthwain Lancer — {4}{B} — Creature — Human Knight
// Oracle: Menace / Whenever a nontoken Knight you control dies, each opponent loses 1 life and you draw a card.
#[test]
fn locthwain_lancer_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("a9baa117-ab06-4af5-9a09-4be423d7cbbd"),
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (5, 5),
        expected_pt: Some((5, 5)),
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

// Loxodon Battle Priest — {4}{W} — Creature — Elephant Cleric
// Oracle: At the beginning of combat on your turn, put a +1/+1 counter on another target creature you control.
#[test]
fn loxodon_battle_priest_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("8cf4ddb0-2a83-4d8f-b636-7ba3c81ea4ca"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (3, 5),
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

// Maja, Bretagard Protector — {2}{G}{W}{W} — Legendary Creature — Human Warrior
// Oracle: Other creatures you control get +1/+1. / Landfall — Whenever a land you control enters, create a 1/1 white Human Warrior creature token.
#[test]
fn maja_bretagard_protector_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("0a2075b5-9609-433d-bcd2-e0a637456cf8"),
        primary_mana: &[
            (ManaColor::White, 2),
            (ManaColor::Green, 1),
            (ManaColor::Colorless, 2),
        ],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Green],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: true,
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

// Makindi Ox — {4}{W} — Creature — Ox
// Oracle: Landfall — Whenever a land you control enters, tap target creature an opponent controls.
#[test]
fn makindi_ox_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("e4f0fa12-542b-485f-a792-3a8aba933522"),
        primary_mana: &[(ManaColor::White, 1), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (4, 4),
        expected_pt: Some((4, 4)),
        enters_tapped: false,
        etb_changes_tapped: true,
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

// Marshdrinker Giant — {3}{G}{G} — Creature — Giant Warrior
// Oracle: When this creature enters, destroy target Island or Swamp an opponent controls.
#[test]
fn marshdrinker_giant_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("be109ed5-9e3e-40b3-9bf0-7caf7cdbb958"),
        primary_mana: &[(ManaColor::Green, 2), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Green],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (4, 3),
        expected_pt: Some((4, 3)),
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

// Messenger Drake — {3}{U}{U} — Creature — Drake
// Oracle: Flying / When this creature dies, draw a card.
#[test]
fn messenger_drake_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("4c62dce6-dc1f-4d2d-95dd-295810ada6a6"),
        primary_mana: &[(ManaColor::Blue, 2), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 5,
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

// Mindeye Drake — {4}{U} — Creature — Drake
// Oracle: Flying / When this creature dies, target player mills five cards.
#[test]
fn mindeye_drake_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("6e33ec9a-e148-4aa6-bff8-b356c45fb369"),
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (2, 5),
        expected_pt: Some((2, 5)),
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

// Mnemonic Wall — {4}{U} — Creature — Wall
// Oracle: Defender / When this creature enters, you may return target instant or sorcery card from your graveyard to your hand.
#[test]
fn mnemonic_wall_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("f2933616-211b-4d3f-8064-2be61c26079a"),
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue],
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
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Molder Beast — {4}{G} — Creature — Beast
// Oracle: Trample / Whenever an artifact is put into a graveyard from the battlefield, this creature gets +2/+0 until end of turn.
#[test]
fn molder_beast_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("2b732c6a-f09c-4ac0-ae3a-1d8820b69aba"),
        primary_mana: &[(ManaColor::Green, 1), (ManaColor::Colorless, 4)],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Green],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (5, 3),
        expected_pt: None,
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::TRAMPLE,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Necromaster Dragon — {3}{U}{B} — Creature — Dragon
// Oracle: Flying / Whenever this creature deals combat damage to a player, you may pay {2}. If you do, create a 2/2 black Zombie creature token and each opponent mills two cards.
#[test]
fn necromaster_dragon_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("b03fc155-2a70-4106-853c-33bdc88d548c"),
        primary_mana: &[
            (ManaColor::Blue, 1),
            (ManaColor::Black, 1),
            (ManaColor::Colorless, 3),
        ],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue, Color::Black],
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
        expected_keywords: KeywordSet::FLYING,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Ob Nixilis, the Fallen — {3}{B}{B} — Legendary Creature — Demon
// Oracle: Landfall — Whenever a land you control enters, you may have target player lose 3 life. If you do, put three +1/+1 counters on Ob Nixilis.
#[test]
fn ob_nixilis_the_fallen_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("154104b7-4125-4276-8bc7-9a6edfe48cb9"),
        primary_mana: &[(ManaColor::Black, 2), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: true,
        is_snow: false,
        base_pt: (3, 3),
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

// Ogre Slumlord — {3}{B}{B} — Creature — Ogre Rogue
// Oracle: Whenever another nontoken creature dies, you may create a 1/1 black Rat creature token. / Rats you control have deathtouch.
#[test]
fn ogre_slumlord_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("0a5e3748-2e58-4e53-9653-8af4e21cf223"),
        primary_mana: &[(ManaColor::Black, 2), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 5,
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
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Ohran Frostfang — {3}{G}{G} — Snow Creature — Snake
// Oracle: Attacking creatures you control have deathtouch. / Whenever a creature you control deals combat damage to a player, draw a card.
#[test]
fn ohran_frostfang_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("b99ada26-9a61-4175-9fb8-15a106960220"),
        primary_mana: &[(ManaColor::Green, 2), (ManaColor::Colorless, 3)],
        alt_mana: None,
        total_mana: 5,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Green],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: true,
        base_pt: (2, 6),
        expected_pt: Some((2, 6)),
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
