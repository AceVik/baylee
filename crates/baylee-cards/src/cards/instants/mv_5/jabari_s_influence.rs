//! Jabari's Influence — {3}{W}{W} — Instant
//! Oracle: Cast this spell only after combat.
//! Oracle: Gain control of target nonartifact, nonblack creature that attacked you this turn and put a -1/-0 counter on it.
//! Set: MIR #24 — Mirage | Scryfall ID: c9f764d7-4c18-40ef-8373-ef1e2a88007e | Oracle ID: 8930b1ce-80ac-48b6-a368-ccff9fd01d38
// NOT SUPPORTED: restricting the target to a creature that specifically attacked you this turn; as written it is any opponent's creature that attacked this turn (a creature that attacked someone else in a game of three or more players, or a planeswalker, is also a target).

use baylee_cards_dsl::prelude::*;

static NONARTIFACT_NONBLACK_ATTACKED_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Not(&Filter::ARTIFACT),
    Filter::Not(&Filter::HasColor(ColorSet::from_slice(&[Color::Black]))),
    Filter::ControlledByOpponent,
    Filter::AttackedThisTurn,
]);

card!(
    index = index::JABARI_S_INFLUENCE,
    oracle_id = "8930b1ce-80ac-48b6-a368-ccff9fd01d38",
    scryfall_id = "c9f764d7-4c18-40ef-8373-ef1e2a88007e",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "can be cast only after combat, gains control of target nonartifact, nonblack creature an opponent controls that attacked this turn, and puts a -1/-0 counter on it; no Filter says \"attacked you this turn\", so in a game of three or more players it can also take a creature that attacked another player, and a creature that attacked a planeswalker",
    ),
    faces = &[face!(
        name = "Jabari's Influence",
        mana_cost = mana!("{3}{W}{W}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[spell!(
        &[
            Effect::ModifyTargets {
                modifier: Modifier::GainControl,
                duration: Duration::Indefinitely,
            },
            Effect::AddCounter {
                kind: CounterKind::Minus {
                    power: 1,
                    toughness: 0,
                },
                amount: Amount::Fixed(1),
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::Object(
            &NONARTIFACT_NONBLACK_ATTACKED_CREATURE,
        ))),
        condition = Some(Condition::AfterCombat),
    )],
);
