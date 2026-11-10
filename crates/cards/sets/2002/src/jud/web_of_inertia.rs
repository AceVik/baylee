//! Web of Inertia — {2}{U} — Enchantment
//! Oracle: At the beginning of combat on each opponent's turn, that player may exile a card from their graveyard. If the player doesn't, creatures they control can't attack you this turn.
//! Set: JUD #53 — Judgment | Scryfall ID: 0e6d2721-1dfc-4f4f-a914-9352ca6981c0 | Oracle ID: a96d24f1-7670-4cb2-9971-5dd1b4c67957
use baylee_cards_dsl::prelude::*;

card!(
    index = index::WEB_OF_INERTIA,
    oracle_id = "a96d24f1-7670-4cb2-9971-5dd1b4c67957",
    scryfall_id = "0e6d2721-1dfc-4f4f-a914-9352ca6981c0",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Web of Inertia",
        mana_cost = mana!("{2}{U}"),
        types = TypeSet::ENCHANTMENT,
    ),],
    abilities = &[triggered!(
        Trigger::StepBegin {
            step: StepKind::CombatBegin,
            whose: PlayerRel::Opponent,
        },
        &[Effect::PlayerMayPayCostOr {
            player: PlayerRel::ActivePlayer,
            cost: &CostPart::ExileFromGraveyard(&Filter::Any),
            effect: &Effect::continuous(
                &Filter::And(&[Filter::CREATURE, Filter::ControlledByActivePlayer]),
                Modifier::CantAttackYou,
                Duration::UntilEndOfTurn,
            ),
        }]
    )],
);
