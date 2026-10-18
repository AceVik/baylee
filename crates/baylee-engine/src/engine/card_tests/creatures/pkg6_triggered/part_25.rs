//! Package 6 triggered creature behavioral tests — part 25.

#[allow(clippy::wildcard_imports)]
use super::*;

// Underfoot Underdogs — {2}{R} — Creature — Goblin Warrior
// Oracle: When this creature enters, create a 1/1 red Goblin creature token. / {1}, {T}: Target creature you control with power 2 or less can't be blocked this turn.
#[test]
fn underfoot_underdogs_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("3740b86d-b0e6-4837-b408-e7f62b95c787"),
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
        base_pt: (1, 2),
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

// Valakut Predator — {2}{R} — Creature — Elemental
// Oracle: Landfall — Whenever a land you control enters, this creature gets +2/+2 until end of turn.
#[test]
fn valakut_predator_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("b86a5d90-0820-46cb-b43c-c03a423a3a9f"),
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
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: true,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Valor Singer — {2}{R} — Creature — Tiefling Bard
// Oracle: Combat Inspiration — At the beginning of combat on your turn, target creature you control gets +1/+0 until end of turn.
#[test]
fn valor_singer_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("3e0d39e2-d99f-4729-899b-c7bcd2b53615"),
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
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Vampire Envoy — {2}{B} — Creature — Vampire Cleric Ally
// Oracle: Flying / Whenever this creature becomes tapped, you gain 1 life.
#[test]
fn vampire_envoy_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("cfc50aef-b17b-4fc1-af07-fe43b89689b7"),
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
        has_tapped_trigger: true,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Vampire Spawn — {2}{B} — Creature — Vampire
// Oracle: When this creature enters, each opponent loses 2 life and you gain 2 life.
#[test]
fn vampire_spawn_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("33cee4b4-3d80-46f1-9223-49ed785891a1"),
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

// Vault Plunderer — {2}{B} — Creature — Human Rogue
// Oracle: When this creature enters, target player draws a card and loses 1 life.
#[test]
fn vault_plunderer_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("80e5d768-cc1f-48f8-b0fe-ed974e4d1a56"),
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
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Viashino Bladescout — {1}{R}{R} — Creature — Lizard Scout
// Oracle: Flash (You may cast this spell any time you could cast an instant.) / When this creature enters, target creature gains first strike until end of turn.
#[test]
fn viashino_bladescout_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("e23d88c4-8c9c-41cb-b4ff-ebb3f880ab63"),
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
        expected_keywords: KeywordSet::FLASH,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Viashino Firstblade — {1}{R}{W} — Creature — Lizard Soldier
// Oracle: Haste / When this creature enters, it gets +2/+2 until end of turn.
#[test]
fn viashino_firstblade_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("766e277c-983e-488c-a885-c03c3c0d3357"),
        primary_mana: &[
            (ManaColor::White, 1),
            (ManaColor::Red, 1),
            (ManaColor::Colorless, 1),
        ],
        alt_mana: None,
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

// Vicious Clown — {2}{R} — Creature — Human Clown
// Oracle: Whenever another creature you control with power 2 or less enters, this creature gets +2/+0 until end of turn.
#[test]
fn vicious_clown_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("e02abbc0-687b-4aef-95b8-bf18e199725a"),
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

// Village Cannibals — {2}{B} — Creature — Human
// Oracle: Whenever another Human creature dies, put a +1/+1 counter on this creature.
#[test]
fn village_cannibals_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("973504d6-e2e4-49c4-86cb-69f31d66388b"),
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

// Vithian Renegades — {1}{R}{G} — Creature — Human Shaman
// Oracle: When this creature enters, destroy target artifact.
#[test]
fn vithian_renegades_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("78fa248d-7f45-4d43-b3e3-c726db938b0f"),
        primary_mana: &[
            (ManaColor::Red, 1),
            (ManaColor::Green, 1),
            (ManaColor::Colorless, 1),
        ],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Red, Color::Green],
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

// Vodalian Knights — {1}{U}{U} — Creature — Merfolk Knight
// Oracle: First strike / This creature can't attack unless defending player controls an Island. / When you control no Islands, sacrifice this creature. / {U}: This creature gains flying until end of turn.
#[test]
fn vodalian_knights_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("f6daa28f-e5ce-440c-8dc2-b36f59ae0d4f"),
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
        expected_pt: None,
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FIRST_STRIKE,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Void Grafter — {1}{G}{U} — Creature — Eldrazi Drone
// Oracle: Devoid (This card has no color.) / Flash (You may cast this spell any time you could cast an instant.) / When this creature enters, another target creature you control gains hexproof until end of turn.
#[test]
fn void_grafter_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("01d202b9-1ac6-484c-a208-a8bc69bccdb2"),
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
        colors: &[],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (2, 4),
        expected_pt: None,
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::DEVOID.union(KeywordSet::FLASH),
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Voracious Vampire — {2}{B} — Creature — Vampire Knight
// Oracle: Menace / When this creature enters, target Vampire you control gets +1/+1 and gains menace until end of turn.
#[test]
fn voracious_vampire_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("c584bdf2-b328-4e50-81e3-45ca6c2aff7a"),
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
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Vulshok Heartstoker — {2}{R} — Creature — Human Shaman
// Oracle: When this creature enters, target creature gets +2/+0 until end of turn.
#[test]
fn vulshok_heartstoker_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("0d996788-7193-45cd-bc6d-0f470521d1ca"),
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
        expected_keywords: KeywordSet::EMPTY,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Warteye Witch — {2}{B} — Creature — Goblin Shaman
// Oracle: Whenever this creature or another creature you control dies, scry 1.
#[test]
fn warteye_witch_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("594a57f6-7827-41c7-aa37-00b498720b16"),
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
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Weaver of Lightning — {2}{R} — Creature — Human Shaman
// Oracle: Reach (This creature can block creatures with flying.) / Whenever you cast an instant or sorcery spell, this creature deals 1 damage to target creature an opponent controls.
#[test]
fn weaver_of_lightning_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("28274c39-19e2-4b4c-bd53-d31d6063c2ba"),
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
        base_pt: (1, 4),
        expected_pt: Some((1, 4)),
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

// Wee Dragonauts — {1}{U}{R} — Creature — Faerie Wizard
// Oracle: Flying / Whenever you cast an instant or sorcery spell, this creature gets +2/+0 until end of turn.
#[test]
fn wee_dragonauts_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("e9e56b1d-5ad2-4807-8c4c-1a1a6a25ecbf"),
        primary_mana: &[
            (ManaColor::Blue, 1),
            (ManaColor::Red, 1),
            (ManaColor::Colorless, 1),
        ],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue, Color::Red],
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
        expected_keywords: KeywordSet::FLYING,
        has_enters_other: false,
        has_spell_cast: true,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Whisper Agent — {1}{U/B}{U/B} — Creature — Human Rogue
// Oracle: Flash / When this creature enters, surveil 1. (Look at the top card of your library. You may put it into your graveyard.)
#[test]
fn whisper_agent_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("2ed76a99-fe5c-4d68-b572-a30848055c8a"),
        primary_mana: &[(ManaColor::Blue, 2), (ManaColor::Colorless, 1)],
        alt_mana: Some(&[(ManaColor::Black, 2), (ManaColor::Colorless, 1)]),
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Blue, Color::Black],
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
        expected_keywords: KeywordSet::FLASH,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Wistful Selkie — {G/U}{G/U}{G/U} — Creature — Merfolk Wizard
// Oracle: When this creature enters, draw a card.
#[test]
fn wistful_selkie_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("4503d0fb-842d-49f1-9769-8509ff0f2ce1"),
        primary_mana: &[(ManaColor::Green, 3)],
        alt_mana: Some(&[(ManaColor::Blue, 3)]),
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

// Woodland Liege — {2}{G} — Creature — Elf Druid Noble
// Oracle: Whenever a Beast you control enters, draw a card.
#[test]
fn woodland_liege_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("df63e124-1542-48d6-b255-cf45855f1e93"),
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

// Wylie Duke, Atiin Hero — {1}{G}{W} — Legendary Creature — Human Ranger
// Oracle: Vigilance / Whenever Wylie Duke becomes tapped, you gain 1 life and draw a card.
#[test]
fn wylie_duke_atiin_hero_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("b3ac27d6-7e18-404d-9af2-b03897a5f1d9"),
        primary_mana: &[
            (ManaColor::White, 1),
            (ManaColor::Green, 1),
            (ManaColor::Colorless, 1),
        ],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Green],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: true,
        is_snow: false,
        base_pt: (4, 2),
        expected_pt: Some((4, 2)),
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::VIGILANCE,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: true,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Xindi Surveyors — {2}{G} — Creature — Xindi Scientist
// Oracle: Landfall — Whenever a land you control enters, put a +1/+1 counter on this creature.
#[test]
fn xindi_surveyors_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("5fdff691-686d-4ce0-bd12-eea676a648b4"),
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
        base_pt: (3, 2),
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

// Yahenni, Undying Partisan — {2}{B} — Legendary Creature — Aetherborn Vampire
// Oracle: Haste / Whenever a creature an opponent controls dies, put a +1/+1 counter on Yahenni. / Sacrifice another creature: Yahenni gains indestructible until end of turn.
#[test]
fn yahenni_undying_partisan_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("fdba89eb-1cf5-46e6-9d09-1adb9bc40fcd"),
        primary_mana: &[(ManaColor::Black, 1), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 3,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black],
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
        expected_keywords: KeywordSet::HASTE,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: true,
        cycling_mana: None,
    });
}

// Yavimaya Sapherd — {2}{G} — Creature — Fungus
// Oracle: When this creature enters, create a 1/1 green Saproling creature token.
#[test]
fn yavimaya_sapherd_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("42a1954f-774e-4d80-98b3-a736f23e2060"),
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

// Yeva's Forcemage — {2}{G} — Creature — Elf Shaman
// Oracle: When this creature enters, target creature gets +2/+2 until end of turn.
#[test]
fn yeva_s_forcemage_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("785e451d-87b6-4772-97e5-d2744818e0fd"),
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

// Zhalfirin Lancer — {2}{W} — Creature — Human Knight
// Oracle: Whenever another Knight you control enters, this creature gets +1/+1 and gains vigilance until end of turn.
#[test]
fn zhalfirin_lancer_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("5adcd0cb-9305-4e77-a117-31e9dca5c96d"),
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

// Zo-Zu the Punisher — {1}{R}{R} — Legendary Creature — Goblin Warrior
// Oracle: Whenever a land enters, Zo-Zu deals 2 damage to that land's controller.
#[test]
fn zo_zu_the_punisher_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("0e412b68-9179-4094-960a-95692428855b"),
        primary_mana: &[(ManaColor::Red, 2), (ManaColor::Colorless, 1)],
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

// Alela, Artful Provocateur — {1}{W}{U}{B} — Legendary Creature — Faerie Warlock
// Oracle: Flying, deathtouch, lifelink / Other creatures you control with flying get +1/+0. / Whenever you cast an artifact or enchantment spell, create a 1/1 blue Faerie creature token with flying.
#[test]
fn alela_artful_provocateur_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("936ae5dc-9838-47f3-ba1f-66523a7f5b76"),
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
        base_pt: (2, 3),
        expected_pt: None,
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FLYING
            .union(KeywordSet::DEATHTOUCH)
            .union(KeywordSet::LIFELINK),
        has_enters_other: false,
        has_spell_cast: true,
        has_tapped_trigger: false,
        has_dies_trigger: false,
        cycling_mana: None,
    });
}

// Algae Gharial — {3}{G} — Creature — Crocodile
// Oracle: Shroud (This creature can't be the target of spells or abilities.) / Whenever another creature dies, you may put a +1/+1 counter on this creature.
#[test]
fn algae_gharial_behavioral() {
    verify_triggered_creature(&TriggeredCreatureSpec {
        card: card_index("7020dc51-8271-40a3-9c4c-ebd40c0b08b2"),
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
        base_pt: (1, 1),
        expected_pt: None,
        enters_tapped: false,
        etb_changes_tapped: false,
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::SHROUD,
        has_enters_other: false,
        has_spell_cast: false,
        has_tapped_trigger: false,
        has_dies_trigger: true,
        cycling_mana: None,
    });
}
