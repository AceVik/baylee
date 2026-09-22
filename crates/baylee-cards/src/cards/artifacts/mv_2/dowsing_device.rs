//! Dowsing Device // Geode Grotto — {1}{R} — Artifact // Land — Cave
//! Oracle: Whenever this artifact or another artifact you control enters, up to one target creature you control gets +1/+0 and gains haste until end of turn. Then transform this artifact if you control four or more artifacts.
//! Oracle: (Transforms from Dowsing Device.)
//! Oracle: {T}: Add {R}.
//! Oracle: {2}{R}, {T}: Until end of turn, target creature gains haste and gets +X/+0, where X is the number of artifacts you control. Activate only as a sorcery.
//! Set: LCI #146 — The Lost Caverns of Ixalan | Scryfall ID: 3d715e9f-223d-462e-8ce3-eebbaf1cd021 | Oracle ID: 2f4374f6-c695-4a5d-a6d6-0e41eaa587ca
//! Face: Dowsing Device — {1}{R} — Artifact
//! Face: Geode Grotto —  — Land — Cave
// PARTIAL — the artifact-enters pump trigger and both Geode Grotto abilities
// are built; the "Then transform …" clause is not (see the NOT SUPPORTED line
// beside the ability).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// Geode Grotto's abilities. A transforming card's back face carries its own
/// (CR 712.2), so they hang off the face and not off the card.
static BACK_FACE_ABILITIES: &[AbilityDef] = &[
    mana_ability!(&[Effect::mana(ManaColor::Red, 1)]),
    activated!(
        cost!("{2}{R}", TapSelf),
        &[Effect::PumpTarget {
            power: Amount::CountOf {
                filter: &Filter::YOUR_ARTIFACT,
                zone: ZoneSel::Battlefield,
            },
            toughness: Amount::Fixed(0),
            keywords: KeywordSet::HASTE,
            duration: Duration::UntilEndOfTurn,
        }],
        target = Some(TargetSpec::Object(&Filter::CREATURE)),
        timing = ActivationTiming::SorcerySpeed,
    ),
];

card!(
    index = index::DOWSING_DEVICE,
    oracle_id = "2f4374f6-c695-4a5d-a6d6-0e41eaa587ca",
    scryfall_id = "3d715e9f-223d-462e-8ce3-eebbaf1cd021",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    faces = &[
        face!(
            name = "Dowsing Device",
            mana_cost = mana!("{1}{R}"),
            types = TypeSet::ARTIFACT,
        ),
        face!(
            name = "Geode Grotto",
            // CR 712.8c: a nonmodal double-faced card is cast as its front
            // face and reaches this one only by transforming.
            castable_from_hand = false,
            types = TypeSet::LAND,
            subtypes = &[subtypes::land::CAVE],
            abilities = BACK_FACE_ABILITIES,
        ),
    ],
    coverage = Coverage::Partial(
        "the transform clause — no Effect branches on how many permanents you \
         control, and ExileSelfReturnAsFace is unconditional"
    ),
    abilities = &[triggered!(
        Trigger::EntersBattlefield(&Filter::Or(&[
            Filter::This,
            Filter::And(&[Filter::ARTIFACT, Filter::ControlledByYou, Filter::Another]),
        ])),
        &[Effect::PumpTarget {
            power: Amount::Fixed(1),
            toughness: Amount::Fixed(0),
            keywords: KeywordSet::HASTE,
            duration: Duration::UntilEndOfTurn,
        }],
        targets = Some(TargetReq::up_to_one(TargetSpec::Object(
            &Filter::YOUR_CREATURE
        ))),
    )],
);

// NOT SUPPORTED: "Then transform this artifact if you control four or more
// artifacts." No Effect runs a branch on a count of permanents a player
// controls — IfControlGreatestCmc compares mana values, and
// Condition::ControlCount is an activation / intervening-`if` condition that
// would gate the whole trigger, including the pump that prints no condition.
// ExileSelfReturnAsFace is the transform shape, but it is unconditional and
// exiles the permanent, which the printed transform does not.
