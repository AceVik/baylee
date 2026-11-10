//! Glasswing Grace // Age-Graced Chapel — {3}{W/B}{W/B} — Enchantment — Aura // Land
//! Oracle: Enchant creature
//! Oracle: Enchanted creature gets +2/+2 and has flying and lifelink.
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {W} or {B}.
//! Set: MH3 #254 — Modern Horizons 3 | Scryfall ID: 90630b20-fc83-475f-bcd5-8bcfee0cf241 | Oracle ID: 3a3e8c9b-e458-4661-980d-0a84a4c2452b
//! Face: Glasswing Grace — {3}{W/B}{W/B} — Enchantment — Aura
//! Face: Age-Graced Chapel —  — Land
// IMPLEMENTED — an Aura that picks its creature as it is cast and arrives
// attached to it, then gives that creature +2/+2 (layer 7c) and flying and
// lifelink (layer 6); the modal back is a land that enters tapped and taps
// for {W} or {B}.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// The back face's own ability list: a modal back is a different face of the
/// same card (CR 712.3), so the land taps for mana and carries none of the
/// Aura's statics.
static CHAPEL_MANA: &[AbilityDef] = &[mana_ability!(&[Effect::mana_choice(&[
    ManaColor::White,
    ManaColor::Black
])])];

card!(
    index = index::GLASSWING_GRACE,
    oracle_id = "3a3e8c9b-e458-4661-980d-0a84a4c2452b",
    scryfall_id = "90630b20-fc83-475f-bcd5-8bcfee0cf241",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::White]),
    faces = &[
        face!(
            name = "Glasswing Grace",
            mana_cost = mana!("{3}{W/B}{W/B}"),
            types = TypeSet::ENCHANTMENT,
            subtypes = &[subtypes::enchantment::AURA],
        ),
        face!(
            name = "Age-Graced Chapel",
            types = TypeSet::LAND,
            abilities = CHAPEL_MANA,
            enter_modifiers = &[EnterModifier::Tapped],
        ),
    ],
    coverage = Coverage::Implemented,
    abilities = &[
        // "Enchant creature" (CR 303.4): an Aura spell picks the creature it
        // will enchant as it is cast, and arrives attached to it.
        // `Effect::AttachSelf` reads the resolution's first target, and the
        // resolving spell object *is* the permanent that then goes to the
        // battlefield — `move_object` clears `attached_to` only on the way
        // *off* the battlefield, so the attachment survives the arrival.
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::CREATURE),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
        ),
        // One printed sentence, two layers: CR 613.1 applies 6 before 7c, and
        // `Modifier::layer` derives which is which, so the order these two sit
        // in says nothing. `Filter::AttachedToBySource` is the enchanted
        // creature — the same handle the Equipment in the pool grant through —
        // and CR 704.5m takes the Aura with the creature it was on.
        static_ability!(Filter::AttachedToBySource, Modifier::ModifyPT(2, 2)),
        static_ability!(
            Filter::AttachedToBySource,
            Modifier::AddKeyword(KeywordSet::FLYING.union(KeywordSet::LIFELINK))
        ),
    ],
);

// Engine-level coverage belongs in `card_tests`: cast Glasswing Grace on a
// creature and read the enchanted creature's projected P/T and keywords back
// off the view, then check the Aura goes to the graveyard when that creature
// dies (CR 704.5m) — and play the back face as a land that comes down tapped
// and taps for {W} or {B}.
