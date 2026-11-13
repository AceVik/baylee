//! Revitalizing Repast // Old-Growth Grove — {B/G} — Instant // Land
//! Oracle: Put a +1/+1 counter on target creature. It gains indestructible until end of turn.
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {B} or {G}.
//! Set: MH3 #256 — Modern Horizons 3 | Scryfall ID: 03522b6b-31ec-4126-8885-5dbb2248688b | Oracle ID: 8dd6d060-d023-48a6-85cb-7a5521b6257b
//! Face: Revitalizing Repast — {B/G} — Instant
//! Face: Old-Growth Grove —  — Land
// IMPLEMENTED — the front face puts a +1/+1 counter on the targeted creature
// and grants it indestructible until end of turn; the back face enters tapped
// and taps for {B} or {G}.

use baylee_cards_dsl::prelude::*;

/// `{T}: Add {B} or {G}.` — the back face's whole text, beside the
/// `EnterModifier::Tapped` on that face.
static GROVE_MANA: &[AbilityDef] = &[mana_ability!(&[Effect::mana_choice(&[
    ManaColor::Black,
    ManaColor::Green,
])])];

card!(
    index = index::REVITALIZING_REPAST,
    oracle_id = "8dd6d060-d023-48a6-85cb-7a5521b6257b",
    scryfall_id = "03522b6b-31ec-4126-8885-5dbb2248688b",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Green]),
    faces = &[
        face!(
            name = "Revitalizing Repast",
            mana_cost = mana!("{B/G}"),
            types = TypeSet::INSTANT,
        ),
        face!(
            name = "Old-Growth Grove",
            types = TypeSet::LAND,
            enter_modifiers = &[EnterModifier::Tapped],
            abilities = GROVE_MANA,
        ),
    ],
    coverage = Coverage::Implemented,
    abilities = &[spell!(
        &[
            Effect::AddCounter {
                kind: CounterKind::P1P1,
                amount: Amount::Fixed(1),
            },
            Effect::continuous(
                &Filter::This,
                Modifier::AddKeyword(KeywordSet::INDESTRUCTIBLE),
                Duration::UntilEndOfTurn,
            ),
        ],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
    )],
);
