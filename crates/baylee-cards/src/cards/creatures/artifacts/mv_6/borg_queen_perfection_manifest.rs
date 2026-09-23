//! Borg Queen, Perfection Manifest — {4}{B}{B} — Legendary Artifact Creature — Borg Noble
//! Oracle: Artifact creatures you control get +2/+0.
//! Oracle: When Borg Queen enters, assimilate target creature card from an opponent's graveyard. (Put it onto the battlefield under your control with a +1/+1 counter. It's a Borg artifact creature and loses all other creature types.)
//! Set: TRC #197 — Star Trek Commander | Scryfall ID: cb07e5e4-154e-4ba5-85a4-ecc78f1555d2 | Oracle ID: f9b46a1a-474f-4fac-8d71-131c1720e4c0
// PARTIAL — a +2/+0 anthem over your artifact creatures, the Queen included,
// and an ETB that assimilates a creature card out of an opponent's graveyard:
// it arrives under your control with a +1/+1 counter, an artifact and a Borg,
// indefinitely.
//
// NOT SUPPORTED: `and loses all other creature types`. `Modifier` can add one
// subtype (`AddSubtype`) and can make an object *every* creature type
// (`AllCreatureTypes`), but nothing in it subtracts a subtype or sets the
// creature-type set to a single member — `RemoveType` takes a `TypeSet` and so
// reaches card types only, and a `Filter` cannot stand in for a modifier. The
// assimilated creature therefore keeps the tribes it was printed with beside
// Borg, which is a subtraction no other clause of this card reads; the rest of
// the keyword action — the zone change, the counter, the artifact type and the
// Borg type — is written.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// "Artifact creatures you control" — the anthem's subject, which the source
/// matches, so the Queen herself is a 3/4 while she is on the battlefield.
static YOUR_ARTIFACT_CREATURE: Filter =
    Filter::And(&[Filter::ARTIFACT, Filter::CREATURE, Filter::ControlledByYou]);

card!(
    index = index::BORG_QUEEN_PERFECTION_MANIFEST,
    oracle_id = "f9b46a1a-474f-4fac-8d71-131c1720e4c0",
    scryfall_id = "cb07e5e4-154e-4ba5-85a4-ecc78f1555d2",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Borg Queen, Perfection Manifest",
        mana_cost = mana!("{4}{B}{B}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::BORG, subtypes::creature::NOBLE],
        power = Some(1),
        toughness = Some(4),
    ),],
    coverage = Coverage::Partial("assimilate does not strip the other creature types"),
    abilities = &[
        static_ability!(YOUR_ARTIFACT_CREATURE, Modifier::ModifyPT(2, 0)),
        // The assimilated card keeps its `ObjectId` across the move out of
        // the graveyard, so the three effects behind the first one all read
        // the same object: `AddCounter` puts its counter on the permanent
        // that just arrived, and `Filter::This` inside a continuous effect is
        // "the target" and binds to it too. `Duration::Indefinitely` because
        // assimilate outlives the Queen — nothing about the type change is
        // tied to her staying on the battlefield.
        triggered!(
            Trigger::ETB,
            &[
                Effect::reanimate(TargetSpec::CardInGraveyard(
                    &Filter::CREATURE,
                    PlayerRel::Opponent
                )),
                Effect::AddCounter {
                    kind: CounterKind::P1P1,
                    amount: Amount::Fixed(1),
                },
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddType(TypeSet::ARTIFACT),
                    Duration::Indefinitely,
                ),
                Effect::continuous(
                    &Filter::This,
                    Modifier::AddSubtype(subtypes::creature::BORG),
                    Duration::Indefinitely,
                ),
            ],
            targets = Some(TargetReq::one(TargetSpec::CardInGraveyard(
                &Filter::CREATURE,
                PlayerRel::Opponent,
            )))
        ),
    ],
);
