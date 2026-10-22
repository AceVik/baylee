//! Sapphire Charm — {U} — Instant
//! Oracle: Choose one —
//! Oracle: • Target player draws a card at the beginning of the next turn's upkeep.
//! Oracle: • Target creature gains flying until end of turn.
//! Oracle: • Target creature an opponent controls phases out. (While it's phased out, it's treated as though it doesn't exist. It phases in before its controller untaps during their next untap step.)
//! Set: MIR #89 — Mirage | Scryfall ID: 2944e5df-eef9-42be-a591-5ac15e306ad8 | Oracle ID: 2ea2bf6c-a427-4d0c-b447-159959327697
// PARTIAL: Effect::AtNextTurnsUpkeep does not carry a targeted player into its delayed trigger, so mode 1 targets a player and has the spell's controller draw a card at the next turn's upkeep; modes 2 and 3 work as printed.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SAPPHIRE_CHARM,
    oracle_id = "2ea2bf6c-a427-4d0c-b447-159959327697",
    scryfall_id = "2944e5df-eef9-42be-a591-5ac15e306ad8",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    coverage = Coverage::Partial(
        "Effect::AtNextTurnsUpkeep does not carry a targeted player into its delayed trigger, so mode 1 targets a player and has the spell's controller draw a card at the next turn's upkeep; modes 2 and 3 work as printed",
    ),
    faces = &[face!(
        name = "Sapphire Charm",
        mana_cost = mana!("{U}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[AbilityDef::ModalSpell {
        choose: ModeCount::ONE,
        modes: &[
            mode!(
                &[Effect::AtNextTurnsUpkeep {
                    effects: &[Effect::draw(1)],
                }],
                targets = Some(TargetReq::one(TargetSpec::AnyPlayer)),
            ),
            mode!(
                &[Effect::ModifyTargets {
                    modifier: Modifier::AddKeyword(KeywordSet::FLYING),
                    duration: Duration::UntilEndOfTurn,
                }],
                targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
            ),
            mode!(
                &[Effect::PhaseOut {
                    target: Some(TargetSpec::Object(&Filter::OPPONENT_CREATURE)),
                }],
                targets = Some(TargetReq::one(TargetSpec::Object(
                    &Filter::OPPONENT_CREATURE,
                ))),
            ),
        ],
    }],
);
