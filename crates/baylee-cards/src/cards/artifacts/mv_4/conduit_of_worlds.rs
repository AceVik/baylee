//! Conduit of Worlds — {2}{G}{G} — Artifact
//! Oracle: You may play lands from your graveyard.
//! Oracle: {T}: Choose target nonland permanent card in your graveyard. If you haven't cast a spell this turn, you may cast that card. If you do, you can't cast additional spells this turn. Activate only as a sorcery.
//! Set: MSC #171 — Marvel Super Heroes Commander | Scryfall ID: 8380eb8d-d1c3-4f96-b3b9-54845188c1d1 | Oracle ID: ed14be15-8f8d-4fe3-a147-f5da8ed873bf
// IMPLEMENTED — the land permission; the {T} ability asks, as it resolves,
// whether its controller has cast a spell this turn, offers the target to
// be cast paying its mana cost (a payment window first), and a cast made
// that way locks its caster out of further spells for the turn.

use baylee_cards_dsl::prelude::*;

/// "Nonland permanent card": an artifact, battle, creature, enchantment or
/// planeswalker card (CR 110.4a) that is not also a land (Dryad Arbor).
static NONLAND_PERMANENT_CARD: Filter = Filter::And(&[
    Filter::HasType(
        TypeSet::ARTIFACT
            .union(TypeSet::BATTLE)
            .union(TypeSet::CREATURE)
            .union(TypeSet::ENCHANTMENT)
            .union(TypeSet::PLANESWALKER),
    ),
    Filter::NONLAND,
]);

card!(
    index = index::CONDUIT_OF_WORLDS,
    oracle_id = "ed14be15-8f8d-4fe3-a147-f5da8ed873bf",
    scryfall_id = "8380eb8d-d1c3-4f96-b3b9-54845188c1d1",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Conduit of Worlds",
        mana_cost = mana!("{2}{G}{G}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[
        static_ability!(Filter::Any, Modifier::PlayLandsFromGraveyard),
        // "{T}: Choose target nonland permanent card in your graveyard. If
        // you haven't cast a spell this turn, you may cast that card. If you
        // do, you can't cast additional spells this turn. Activate only as a
        // sorcery."
        activated!(
            Cost::TAP,
            &[Effect::IfCondition {
                condition: Condition::YouCastNoSpellThisTurn,
                then: &[Effect::MayCastTarget {
                    then_no_more_spells: true,
                }],
                otherwise: &[],
            }],
            target = Some(TargetSpec::CardInGraveyard(
                &NONLAND_PERMANENT_CARD,
                PlayerRel::You
            )),
            timing = ActivationTiming::SorcerySpeed,
        ),
    ],
);
