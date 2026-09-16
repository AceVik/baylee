//! Strength of the Harvest // Haven of the Harvest — {2}{G/W} — Enchantment — Aura // Land
//! Oracle: Enchant creature
//! Oracle: Enchanted creature gets +1/+1 for each creature and/or enchantment you control.
//! Oracle: This land enters tapped.
//! Oracle: {T}: Add {G} or {W}.
//! Set: MH3 #258 — Modern Horizons 3 | Scryfall ID: a7143aa7-b16d-4e63-910c-6ceec55483f3 | Oracle ID: 1a8c996d-ca93-4c17-ace5-66ecd6b99317
//! Face: Strength of the Harvest — {2}{G/W} — Enchantment — Aura
//! Face: Haven of the Harvest —  — Land
// IMPLEMENTED — an Aura that picks its creature as it is cast and arrives
// attached to it, then swells that creature by +1/+1 for every creature and
// enchantment its own controller has on the board; the modal back is a land
// that enters tapped and taps for {G} or {W}.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// "for each creature and/or enchantment you control". `ModifyPTPerCount`
/// already counts only permanents the *effect's* controller controls, so this
/// states the two types and nothing else — and the Aura is itself an
/// enchantment you control, so it counts itself, while a permanent that is
/// both a creature and an enchantment is one object and counts once.
static CREATURE_OR_ENCHANTMENT: Filter = Filter::Or(&[Filter::CREATURE, Filter::ENCHANTMENT]);

/// The back face's own ability list: a modal back is a different face of the
/// same card (CR 712.8f), so the land taps for mana and carries none of the
/// Aura's statics.
static HAVEN_MANA: &[AbilityDef] = &[mana_ability!(&[Effect::mana_choice(&[
    ManaColor::Green,
    ManaColor::White,
])])];

card!(
    index = index::STRENGTH_OF_THE_HARVEST,
    oracle_id = "1a8c996d-ca93-4c17-ace5-66ecd6b99317",
    scryfall_id = "a7143aa7-b16d-4e63-910c-6ceec55483f3",
    color_identity = ColorSet::from_slice(&[Color::Green, Color::White]),
    faces = &[
        face!(
            name = "Strength of the Harvest",
            mana_cost = mana!("{2}{G/W}"),
            types = TypeSet::ENCHANTMENT,
            subtypes = &[subtypes::enchantment::AURA],
        ),
        face!(
            name = "Haven of the Harvest",
            types = TypeSet::LAND,
            abilities = HAVEN_MANA,
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
        // `Filter::AttachedToBySource` is the enchanted creature — the same
        // handle the Equipment in the pool grant through — and CR 704.5m
        // takes the Aura with the creature it was on. The count is read on
        // every projection rather than fixed as the Aura enters, which is
        // what "for each" means: layer 7c, derived by `Modifier::layer`.
        static_ability!(
            Filter::AttachedToBySource,
            Modifier::ModifyPTPerCount {
                filter: &CREATURE_OR_ENCHANTMENT,
                p: 1,
                t: 1,
            }
        ),
    ],
);

// Engine-level coverage belongs in `card_tests`: cast Strength of the Harvest
// on a creature, read the enchanted creature's projected P/T back off the view
// with two and then three creatures/enchantments on the board, and check the
// Aura goes to the graveyard when that creature dies (CR 704.5m) — and play
// the back face as a land that comes down tapped and taps for {G} or {W}.
