//! Package 5 non-triggered creature behavioral tests — part 30.

#[allow(clippy::wildcard_imports)]
use super::*;

// Wolfir Avenger — {1}{G}{G} — Creature — Wolf Warrior
// Oracle: Flash (You may cast this spell any time you could cast an instant.) / {1}{G}: Regenerate this creature.
#[test]
fn wolfir_avenger_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("83deda94-66e8-4655-9cd5-6d97d809946d"),
        primary_mana: &[(ManaColor::Green, 2), (ManaColor::Colorless, 1)],
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
        choose_color: false,
        copy_on_enter: false,
        expected_counters: None,
        expected_keywords: KeywordSet::FLASH,
        prowess_bonus: 0,
        has_opponent_enter_tapped_replacement: false,
        has_untap_self: false,
        cycling_mana: None,
        lord_subtype: None,
        lord_pt: None,
        lord_controlled_by_you: false,
    });
}

// Wormwood Dryad — {2}{G} — Creature — Dryad
// Oracle: {G}: This creature gains forestwalk until end of turn and deals 1 damage to you. (It can't be blocked as long as defending player controls a Forest.) / {B}: This creature gains swampwalk until end of turn and deals 1 damage to you. (It can't be blocked as long as defending player controls a Swamp.)
#[test]
fn wormwood_dryad_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("277bd0c7-2400-4c94-ba82-4c9472badb60"),
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
        base_pt: (3, 1),
        expected_pt: Some((3, 1)),
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

// Wu Warship — {2}{U} — Creature — Human Soldier
// Oracle: This creature can't attack unless defending player controls an Island.
#[test]
fn wu_warship_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("f184e860-05c3-43cf-a625-ab53427406c5"),
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

// Yavimaya Gnats — {2}{G} — Creature — Insect
// Oracle: Flying / {G}: Regenerate this creature.
#[test]
fn yavimaya_gnats_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("828c1a94-36e1-484b-afc8-87f85956d60d"),
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

// Zombie Trailblazer — {B}{B}{B} — Creature — Zombie Scout
// Oracle: Tap an untapped Zombie you control: Target land becomes a Swamp until end of turn. / Tap an untapped Zombie you control: Target creature gains swampwalk until end of turn. (It can't be blocked as long as defending player controls a Swamp.)
#[test]
fn zombie_trailblazer_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("5cd96cf2-c1c6-4982-8cf8-70c16704e387"),
        primary_mana: &[(ManaColor::Black, 3)],
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

// Abzan Kin-Guard — {3}{G} — Creature — Human Warrior
// Oracle: This creature has lifelink as long as you control a white or black permanent.
#[test]
fn abzan_kin_guard_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("b292d183-1ba9-40b4-9cba-bd70bf1380ef"),
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

// Accursed Horde — {3}{B} — Creature — Zombie
// Oracle: {1}{B}: Target attacking Zombie gains indestructible until end of turn. (Damage and effects that say "destroy" don't destroy it. If its toughness is 0 or less, it still dies.)
#[test]
fn accursed_horde_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("8f3cba20-e7a4-47f4-8bce-90168ce9cc5c"),
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

// Aerial Engineer — {2}{W}{U} — Creature — Human Artificer
// Oracle: As long as you control an artifact, this creature gets +2/+0 and has flying.
#[test]
fn aerial_engineer_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("27f5969e-d613-4cbc-87af-98899bb5c58a"),
        primary_mana: &[
            (ManaColor::White, 1),
            (ManaColor::Blue, 1),
            (ManaColor::Colorless, 2),
        ],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::White, Color::Blue],
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

// Amphin Pathmage — {3}{U} — Creature — Salamander Wizard
// Oracle: {2}{U}: Target creature can't be blocked this turn.
#[test]
fn amphin_pathmage_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("53e4273c-3292-47a0-90d1-8a0eb0eaf381"),
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Colorless, 3)],
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

// Aquus Steed — {3}{U} — Creature — Beast
// Oracle: {2}{U}, {T}: Target creature gets -2/-0 until end of turn.
#[test]
fn aquus_steed_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("3517abe3-eb18-49af-9f1c-b372d2039d99"),
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Colorless, 3)],
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
        base_pt: (1, 3),
        expected_pt: Some((1, 3)),
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

// Armory Guard — {3}{W} — Creature — Giant Soldier
// Oracle: This creature has vigilance as long as you control a Gate.
#[test]
fn armory_guard_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("96c67950-df59-430c-94fd-234086c6c8f2"),
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
        base_pt: (2, 5),
        expected_pt: Some((2, 5)),
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

// Auriok Salvagers — {3}{W} — Creature — Human Soldier
// Oracle: {1}{W}: Return target artifact card with mana value 1 or less from your graveyard to your hand.
#[test]
fn auriok_salvagers_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("db2f3a06-f75e-4689-b147-9820aebb30e2"),
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

// Aurora Griffin — {3}{W} — Creature — Griffin
// Oracle: Flying / {W}: Target permanent becomes white until end of turn.
#[test]
fn aurora_griffin_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("4d74c735-8035-4c42-bfb1-1bc97e5bdd9a"),
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

// Aven Redeemer — {3}{W} — Creature — Bird Cleric
// Oracle: Flying / {T}: Prevent the next 2 damage that would be dealt to any target this turn.
#[test]
fn aven_redeemer_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("faac29f5-10ad-4800-a9d3-2259c4800352"),
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

// Barrenton Cragtreads — {2}{W/U}{W/U} — Creature — Kithkin Scout
// Oracle: This creature can't be blocked by red creatures.
#[test]
fn barrenton_cragtreads_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("969233f8-a644-4297-a2bf-b1ca7a58bb0b"),
        primary_mana: &[(ManaColor::White, 2), (ManaColor::Colorless, 2)],
        alt_mana: Some(&[(ManaColor::Blue, 2), (ManaColor::Colorless, 2)]),
        total_mana: 4,
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

// Ben-Ben, Akki Hermit — {2}{R}{R} — Legendary Creature — Goblin Shaman
// Oracle: {T}: Ben-Ben deals damage to target attacking creature equal to the number of untapped Mountains you control.
#[test]
fn ben_ben_akki_hermit_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("e5b7d0c5-f478-43ef-98fd-7792122ab8f5"),
        primary_mana: &[(ManaColor::Red, 2), (ManaColor::Colorless, 2)],
        alt_mana: None,
        total_mana: 4,
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

// Blade Historian — {R/W}{R/W}{R/W}{R/W} — Creature — Human Cleric
// Oracle: Attacking creatures you control have double strike.
#[test]
fn blade_historian_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("314f0a96-5e91-42a3-9d75-3f641f22a9ee"),
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

// Blazing Hellhound — {2}{B}{R} — Creature — Elemental Dog
// Oracle: {1}, Sacrifice another creature: This creature deals 1 damage to any target.
#[test]
fn blazing_hellhound_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("598a7410-35e8-4d62-9bf1-d44ee7717040"),
        primary_mana: &[
            (ManaColor::Black, 1),
            (ManaColor::Red, 1),
            (ManaColor::Colorless, 2),
        ],
        alt_mana: None,
        total_mana: 4,
        colorless_pips: 0,
        snow_pips: 0,
        has_x: false,
        colors: &[Color::Black, Color::Red],
        is_artifact: false,
        is_enchantment: false,
        is_legendary: false,
        is_snow: false,
        base_pt: (4, 3),
        expected_pt: Some((4, 3)),
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

// Blistering Dieflyn — {3}{R} — Creature — Imp
// Oracle: Flying / {B/R}: This creature gets +1/+0 until end of turn.
#[test]
fn blistering_dieflyn_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("4cd6ad78-7441-4dd7-b486-d9575e8e0947"),
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

// Blockade Runner — {3}{U} — Creature — Merfolk
// Oracle: {U}: This creature can't be blocked this turn.
#[test]
fn blockade_runner_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("f97df8f4-a7ef-45f9-9563-b76b4d2b03f5"),
        primary_mana: &[(ManaColor::Blue, 1), (ManaColor::Colorless, 3)],
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

// Bloodcrusher of Khorne — {3}{R} — Creature — Demon Knight
// Oracle: Trample / Devastating Charge — Other creatures you control have trample.
#[test]
fn bloodcrusher_of_khorne_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("ae5b9251-2e2d-4e12-b4b4-da051683f970"),
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

// Bloodfire Kavu — {2}{R}{R} — Creature — Kavu
// Oracle: {R}, Sacrifice this creature: It deals 2 damage to each creature.
#[test]
fn bloodfire_kavu_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("a12880c4-1987-42af-8595-0b88981e1091"),
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

// Bloodshot Trainee — {3}{R} — Creature — Goblin Warrior
// Oracle: {T}: This creature deals 4 damage to target creature. Activate only if this creature's power is 4 or greater.
#[test]
fn bloodshot_trainee_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("87ab1c79-79f5-48a9-af1a-7c876684c8d9"),
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

// Boartusk Liege — {1}{R/G}{R/G}{R/G} — Creature — Goblin Knight
// Oracle: Trample / Other red creatures you control get +1/+1. / Other green creatures you control get +1/+1.
#[test]
fn boartusk_liege_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("4fcffb56-5ab6-44e2-804e-8b3c55cc5415"),
        primary_mana: &[(ManaColor::Red, 3), (ManaColor::Colorless, 1)],
        alt_mana: Some(&[(ManaColor::Green, 3), (ManaColor::Colorless, 1)]),
        total_mana: 4,
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

// Bogbrew Witch — {3}{B} — Creature — Human Wizard
// Oracle: {2}, {T}: Search your library for a card named Festering Newt or Bubbling Cauldron, put it onto the battlefield tapped, then shuffle.
#[test]
fn bogbrew_witch_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("ad138184-ab2d-407a-9f9c-94b0675be19d"),
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
        base_pt: (1, 3),
        expected_pt: Some((1, 3)),
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

// Bonescythe Sliver — {3}{W} — Creature — Sliver
// Oracle: Sliver creatures you control have double strike. (They deal both first-strike and regular combat damage.)
#[test]
fn bonescythe_sliver_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("1d95ad32-768f-40a1-a461-e0562326b2b7"),
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

// Bonesplitter Sliver — {3}{R} — Creature — Sliver
// Oracle: All Sliver creatures get +2/+0.
#[test]
fn bonesplitter_sliver_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("a0f6834c-faa9-4d90-adad-847a8b337fe6"),
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
        base_pt: (2, 2),
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
        lord_subtype: Some(subtypes::creature::SLIVER),
        lord_pt: Some((2, 0)),
        lord_controlled_by_you: false,
    });
}

// Canopy Tactician — {3}{G} — Creature — Elf Warrior
// Oracle: Other Elves you control get +1/+1. / {T}: Add {G}{G}{G}.
#[test]
fn canopy_tactician_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("8b20d6f4-6322-4435-92d5-acaae74774f4"),
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
        lord_subtype: Some(subtypes::creature::ELF),
        lord_pt: Some((1, 1)),
        lord_controlled_by_you: true,
    });
}

// Cateran Kidnappers — {2}{B}{B} — Creature — Human Mercenary
// Oracle: {3}, {T}: Search your library for a Mercenary permanent card with mana value 3 or less, put it onto the battlefield, then shuffle.
#[test]
fn cateran_kidnappers_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("938c4f2e-6dda-4ac4-b50c-40dc165b4eaa"),
        primary_mana: &[(ManaColor::Black, 2), (ManaColor::Colorless, 2)],
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
        base_pt: (4, 2),
        expected_pt: Some((4, 2)),
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

// Caterwauling Boggart — {3}{R} — Creature — Goblin Shaman
// Oracle: Goblins you control and Elementals you control have menace. (They can't be blocked except by two or more creatures.)
#[test]
fn caterwauling_boggart_behavioral() {
    verify_creature(&CreatureSpec {
        card: card_index("ab3f4455-7329-4145-b9c4-93eef38bf327"),
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
