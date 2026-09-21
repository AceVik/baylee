//! Sink into Stupor // Soporific Springs — {1}{U}{U} — Instant // Land
//! Oracle: Return target spell or nonland permanent an opponent controls to its owner's hand.
//! Oracle: As this land enters, you may pay 3 life. If you don't, it enters tapped.
//! Oracle: {T}: Add {U}.
//! Set: MH3 #241 — Modern Horizons 3 | Scryfall ID: 5358b87a-1a29-426d-b165-40c97da2c14d | Oracle ID: bcc6eece-75ea-494c-b33a-d4477d504e0b
//! Face: Sink into Stupor — {1}{U}{U} — Instant
//! Face: Soporific Springs —  — Land
// IMPLEMENTED — front face is the printed return (one StackOrBattlefield
// target: a spell or a nonland permanent an opponent controls); back face is
// a shock land — TappedOrPayLife(3) as it enters, and {T}: Add {U}.

use baylee_cards_dsl::prelude::*;

/// "A spell or nonland permanent an opponent controls" — one filter, because
/// the two halves are the same clause: no spell on the stack is a land, and a
/// spell's controller is the player who cast it.
static NONLAND_OPPONENT: Filter = f!(opponents NONLAND);

static SPRINGS_MANA: &[AbilityDef] = &[mana_ability!(&[Effect::mana(ManaColor::Blue, 1)])];

card!(
    index = index::SINK_INTO_STUPOR,
    oracle_id = "bcc6eece-75ea-494c-b33a-d4477d504e0b",
    scryfall_id = "5358b87a-1a29-426d-b165-40c97da2c14d",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[
        face!(
            name = "Sink into Stupor",
            mana_cost = mana!("{1}{U}{U}"),
            types = TypeSet::INSTANT,
        ),
        face!(
            name = "Soporific Springs",
            types = TypeSet::LAND,
            enter_modifiers = &[EnterModifier::TappedOrPayLife(3)],
            abilities = SPRINGS_MANA,
        ),
    ],
    coverage = Coverage::Implemented,
    abilities = &[spell!(
        &[Effect::ReturnToHand {
            target: TargetSpec::StackOrBattlefield(&NONLAND_OPPONENT),
        }],
        targets = Some(TargetReq::one(TargetSpec::StackOrBattlefield(
            &NONLAND_OPPONENT
        )))
    )],
);
