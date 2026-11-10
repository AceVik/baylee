//! Provoke — {1}{G} — Instant
//! Oracle: Untap target creature you don't control. That creature blocks this turn if able.
//! Oracle: Draw a card.
//! Set: TPR #186 — Tempest Remastered | Scryfall ID: 9d0df301-737c-417b-b924-0a6ae42c77a0 | Oracle ID: 731821b1-89fd-4166-94b8-35a1364e4d9d
// IMPLEMENTED

use baylee_cards_dsl::prelude::*;

static NOT_YOUR_CREATURE: Filter =
    Filter::And(&[Filter::CREATURE, Filter::Not(&Filter::ControlledByYou)]);

card!(
    index = index::PROVOKE,
    oracle_id = "731821b1-89fd-4166-94b8-35a1364e4d9d",
    scryfall_id = "9d0df301-737c-417b-b924-0a6ae42c77a0",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Provoke",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[
            Effect::UntapTarget,
            Effect::ModifyTargets {
                modifier: Modifier::BlocksEachAttackerIfAble,
                duration: Duration::UntilEndOfTurn,
            },
            Effect::draw(1),
        ],
        targets = Some(TargetReq::one(TargetSpec::Object(&NOT_YOUR_CREATURE))),
    ),],
);
