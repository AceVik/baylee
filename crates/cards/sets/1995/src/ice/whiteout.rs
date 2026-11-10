//! Whiteout — {1}{G} — Instant
//! Oracle: All creatures lose flying until end of turn.
//! Oracle: Sacrifice a snow land: Return this card from your graveyard to your hand.
//! Set: ME2 #185 — Masters Edition II | Scryfall ID: ee7bb29a-daf4-42d3-9404-c9f25d72a648 | Oracle ID: 9e8de3e1-1159-460f-99b7-1043e5814597

use baylee_cards_dsl::prelude::*;

static SNOW_LAND: Filter = Filter::And(&[Filter::LAND, Filter::HasSupertype(SupertypeSet::SNOW)]);

card!(
    index = index::WHITEOUT,
    oracle_id = "9e8de3e1-1159-460f-99b7-1043e5814597",
    scryfall_id = "ee7bb29a-daf4-42d3-9404-c9f25d72a648",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Whiteout",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[
        spell!(&[Effect::continuous(
            &Filter::CREATURE,
            Modifier::RemoveKeyword(KeywordSet::FLYING),
            Duration::UntilEndOfTurn,
        )]),
        activated!(
            cost!(Sacrifice(&SNOW_LAND)),
            &[Effect::ReturnToHand {
                target: TargetSpec::ThisObject,
            }],
            zone = ActivationZone::Graveyard
        ),
    ],
);
