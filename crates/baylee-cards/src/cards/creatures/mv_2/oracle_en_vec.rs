//! Oracle en-Vec — {1}{W} — Creature — Human Wizard
//! Oracle: {T}: Target opponent chooses any number of creatures they control. During that player's next turn, the chosen creatures attack if able, and other creatures can't attack. At the beginning of that turn's end step, destroy each of the chosen creatures that didn't attack this turn. Activate only during your turn.
//! Set: TMP #31 — Tempest | Scryfall ID: cc538730-c46c-4e5f-bc1f-0efb7765086d | Oracle ID: 367faa8c-617a-41a0-901c-f65106e86d56
// PARTIAL: taps during your turn targeting an opponent to force opponents' creatures to attack until your next turn; having target opponent choose a subset of their creatures to attack while others can't and destroying chosen creatures that didn't attack at the end step is not supported.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::ORACLE_EN_VEC,
    oracle_id = "367faa8c-617a-41a0-901c-f65106e86d56",
    scryfall_id = "cc538730-c46c-4e5f-bc1f-0efb7765086d",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "taps during your turn targeting an opponent to force opponents' creatures to attack until your next turn; having target opponent choose a subset of their creatures to attack while others can't and destroying chosen creatures that didn't attack at the end step is not supported",
    ),
    faces = &[face!(
        name = "Oracle en-Vec",
        mana_cost = mana!("{1}{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WIZARD],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[activated!(
        cost!(TapSelf),
        &[Effect::continuous(
            &Filter::OPPONENT_CREATURE,
            Modifier::AttacksEachCombat,
            Duration::UntilYourNextTurn,
        )],
        target = Some(TargetSpec::AnyOpponent),
        condition = Some(Condition::YourTurn),
    )],
);
