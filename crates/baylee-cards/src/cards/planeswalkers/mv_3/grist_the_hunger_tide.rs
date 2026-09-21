//! Grist, the Hunger Tide — {1}{B}{G} — Legendary Planeswalker — Grist
//! Oracle: As long as Grist isn't on the battlefield, it's a 1/1 Insect creature in addition to its other types.
//! Oracle: +1: Create a 1/1 black and green Insect creature token, then mill a card. If an Insect card was milled this way, put a loyalty counter on Grist and repeat this process.
//! Oracle: −2: You may sacrifice a creature. When you do, destroy target creature or planeswalker.
//! Oracle: −5: Each opponent loses life equal to the number of creature cards in your graveyard.
//! Set: DSC #220 — Duskmourn: House of Horror Commander | Scryfall ID: 1925dc45-4dee-4772-aa16-3b4ca54be6c7 | Oracle ID: 0efb0d7e-dea0-4817-a243-15066e9ef333
// PARTIAL — the −5 loyalty ability is implemented. The static animation, the
// +1 mill loop and the −2 reflexive sacrifice-then-destroy are not written.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::GRIST_THE_HUNGER_TIDE,
    oracle_id = "0efb0d7e-dea0-4817-a243-15066e9ef333",
    scryfall_id = "1925dc45-4dee-4772-aa16-3b4ca54be6c7",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Green]),
    coverage = Coverage::Partial(
        "static animating ability, the +1 mill/repeat loop and the −2 reflexive \
         sacrifice are not expressible"
    ),
    faces = &[face!(
        name = "Grist, the Hunger Tide",
        mana_cost = mana!("{1}{B}{G}"),
        types = TypeSet::PLANESWALKER,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::planeswalker::GRIST],
        loyalty = Some(3),
    ),],
    // NOT SUPPORTED: "As long as Grist isn't on the battlefield, it's a 1/1 Insect
    // creature in addition to its other types." — a `StaticAbility` is registered
    // while its source is on the battlefield and carries no field to condition it
    // on the source's zone; no `Modifier` says "in every zone but this one".
    // NOT SUPPORTED: "+1: … If an Insect card was milled this way, put a loyalty
    // counter on Grist and repeat this process." — no `Effect` reports what `Mill`
    // sent to the graveyard, so the `if` has nothing to read, and there is no
    // "repeat this process" construct in the vocabulary at all.
    // NOT SUPPORTED: "−2: You may sacrifice a creature. When you do, destroy target
    // creature or planeswalker." — the reflexive "when you do" trigger has no shape
    // here: a `TargetReq` is answered when the ability is activated (CR 601.2c),
    // not after a sacrifice made while it resolves, and `Effect::MayDo` offers the
    // whole clause as one yes/no rather than a sacrifice that arms a second one.
    abilities = &[loyalty!(
        -5,
        &[Effect::LoseLife {
            amount: Amount::CountOf {
                filter: &Filter::CREATURE,
                zone: ZoneSel::GraveyardYou,
            },
            target: PlayerRel::EachOpponent,
        }]
    )],
);
