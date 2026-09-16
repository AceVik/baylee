//! Witch Enchanter // Witch-Blessed Meadow — {3}{W} — Creature — Human Warlock // Land
//! Oracle: When this creature enters, destroy target artifact or enchantment an opponent controls.
//! Oracle: As this land enters, you may pay 3 life. If you don't, it enters tapped.
//! Oracle: {T}: Add {W}.
//! Set: MH3 #239 — Modern Horizons 3 | Scryfall ID: 62061e7c-cf19-4f03-b8fa-2bdba62d6b0b | Oracle ID: 0355249a-8e4e-41db-9cea-1b901faffbe6
//! Face: Witch Enchanter — {3}{W} — Creature — Human Warlock
//! Face: Witch-Blessed Meadow —  — Land
// IMPLEMENTED — the Warlock front destroys an opponent's artifact or
// enchantment on its own ETB; the modal back is reached by the face-choice
// land play (CR 712.4a), asks for 3 life as it enters or comes down tapped,
// and taps for {W}.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// "Artifact or enchantment an opponent controls."
///
/// Named because the trigger says it twice: what may be pointed at, and what
/// is destroyed. `opponents` is the card's own word — not "you don't control",
/// which would also reach a teammate's permanents.
static OPPONENTS_ARTIFACT_OR_ENCHANTMENT: Filter = f!(opponents ARTIFACT_OR_ENCHANTMENT);

static MEADOW_MANA: &[AbilityDef] = &[mana_ability!(&[Effect::mana(ManaColor::White, 1)])];

card!(
    index = index::WITCH_ENCHANTER,
    oracle_id = "0355249a-8e4e-41db-9cea-1b901faffbe6",
    scryfall_id = "62061e7c-cf19-4f03-b8fa-2bdba62d6b0b",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[
        face!(
            name = "Witch Enchanter",
            mana_cost = mana!("{3}{W}"),
            types = TypeSet::CREATURE,
            subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WARLOCK],
            power = Some(2),
            toughness = Some(2),
        ),
        face!(
            name = "Witch-Blessed Meadow",
            types = TypeSet::LAND,
            abilities = MEADOW_MANA,
            enter_modifiers = &[EnterModifier::TappedOrPayLife(3)],
        ),
    ],
    coverage = Coverage::Implemented,
    abilities = &[triggered!(
        Trigger::ETB,
        &[Effect::destroy(TargetSpec::Object(
            &OPPONENTS_ARTIFACT_OR_ENCHANTMENT
        ))],
        targets = Some(TargetReq::one(TargetSpec::Object(
            &OPPONENTS_ARTIFACT_OR_ENCHANTMENT
        )))
    )],
);

// Engine-level test belongs in baylee-engine (mdfc_tests): the creature front
// asks for an opponent's artifact or enchantment and destroys it on entry,
// while the back face is offered as a second `CastModeKind::PlayLandFace` on a
// land play, puts up the pay-3-life-or-enter-tapped question, and taps for {W}.
