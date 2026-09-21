//! Lose Focus — {1}{U} — Instant
//! Oracle: Replicate {U} (When you cast this spell, copy it for each time you paid its replicate cost. You may choose new targets for the copies.)
//! Oracle: Counter target spell unless its controller pays {2}.
//! Set: MH2 #49 — Modern Horizons 2 | Scryfall ID: 985bdb0c-ce6c-4506-8163-76f3b2fdf5fb | Oracle ID: 1cea6439-7ae5-4887-8c33-7da9fb36e2d4
// PARTIAL — the whole counter clause is built (PlayerMayPayOr refuses the
// {2} for the target spell's controller and CounterTargetSpell runs when
// they decline); replicate has no variant in the DSL.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::LOSE_FOCUS,
    oracle_id = "1cea6439-7ae5-4887-8c33-7da9fb36e2d4",
    scryfall_id = "985bdb0c-ce6c-4506-8163-76f3b2fdf5fb",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Lose Focus",
        mana_cost = mana!("{1}{U}"),
        types = TypeSet::INSTANT,
    ),],
    coverage = Coverage::Partial(
        "replicate — the DSL has no variant for a spell that copies itself for each time its replicate cost was paid"
    ),
    // NOT SUPPORTED: Replicate {U} (When you cast this spell, copy it for each time you paid its replicate cost. You may choose new targets for the copies.)
    abilities = &[spell!(
        &[Effect::PlayerMayPayOr {
            player: PlayerRel::ControllerOfTarget,
            mana: Amount::Fixed(2),
            effect: &Effect::CounterTargetSpell,
        }],
        targets = Some(TargetReq::one(TargetSpec::Spell(&Filter::Any)))
    )],
);
