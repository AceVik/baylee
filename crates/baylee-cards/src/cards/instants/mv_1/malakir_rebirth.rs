//! Malakir Rebirth // Malakir Mire — {B} — Instant // Land
//! Oracle: Choose target creature. You lose 2 life. Until end of turn, that creature gains "When this creature dies, return it to the battlefield tapped under its owner's control."
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {B}.
//! Set: ZNR #111 — Zendikar Rising | Scryfall ID: 609d3ecf-f88d-4268-a8d3-4bf2bcf5df60 | Oracle ID: a731e87b-8d99-4b64-8ee3-8e540d652366
//! Face: Malakir Rebirth — {B} — Instant
//! Face: Malakir Mire —  — Land
// PARTIAL — Malakir Mire is complete ({T}: Add {B}, enters tapped).
// Malakir Rebirth keeps its target and its two life; the granted
// dies-trigger is dropped, see the NOT SUPPORTED line beside it.

use baylee_cards_dsl::prelude::*;

/// Malakir Mire's only ability — the land prints no basic land type, so the
/// intrinsic shortcut has nothing to answer and the card supplies the line.
static BACK_MANA: &[AbilityDef] = &[mana_ability!(&[Effect::mana(ManaColor::Black, 1)])];

card!(
    index = index::MALAKIR_REBIRTH,
    oracle_id = "a731e87b-8d99-4b64-8ee3-8e540d652366",
    scryfall_id = "609d3ecf-f88d-4268-a8d3-4bf2bcf5df60",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[
        face!(
            name = "Malakir Rebirth",
            mana_cost = mana!("{B}"),
            types = TypeSet::INSTANT,
        ),
        face!(
            name = "Malakir Mire",
            types = TypeSet::LAND,
            enter_modifiers = &[EnterModifier::Tapped],
            abilities = BACK_MANA,
        ),
    ],
    coverage = Coverage::Partial(
        "the granted dies-trigger is dropped: no effect returns a card from a \
         graveyard to the battlefield tapped under its owner's control"
    ),
    abilities = &[
        // NOT SUPPORTED: "Until end of turn, that creature gains \"When this
        // creature dies, return it to the battlefield tapped under its
        // owner's control.\"" — `Modifier::GrantTriggered` inside
        // `Effect::continuous` could grant the trigger, but the clause it
        // would carry is not sayable: the nearest variant,
        // `Effect::GraveyardToBattlefield`, puts the card onto the
        // battlefield untapped and under the effect's controller, where the
        // printing says *tapped* and under its *owner's* control.
        spell!(
            &[Effect::LoseLife {
                amount: Amount::Fixed(2),
                target: PlayerRel::You,
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
        ),
    ],
);
