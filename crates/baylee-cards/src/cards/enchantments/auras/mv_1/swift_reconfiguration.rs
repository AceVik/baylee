//! Swift Reconfiguration — {W} — Enchantment — Aura
//! Oracle: Flash
//! Oracle: Enchant creature or Vehicle
//! Oracle: Enchanted permanent is a Vehicle artifact with crew 5 and it loses all other card types. (It's not a creature unless it's crewed.)
//! Set: NEC #10 — Neon Dynasty Commander | Scryfall ID: 975dcfab-0281-4fee-92aa-021ea6c524c7 | Oracle ID: 5d47e820-913f-441a-a6cc-37ab3181d79a
// PARTIAL — a flash Aura that picks a creature or a Vehicle as it is cast,
// arrives attached to it, and then rewrites what that permanent *is* in
// layer 4: an artifact Vehicle that has lost every other card type. On a
// creature that is the whole card — it stops being a creature, so it stops
// attacking, stops blocking, and stops answering anything that wants one.
//
// NOT SUPPORTED: `with crew 5` — "Tap any number of untapped creatures you
// control with total power 5 or greater: This Vehicle becomes an artifact
// creature until end of turn", in the reminder text crew prints. The ability
// shape is there: `Modifier::GrantActivated { cost, effects, mana_ability }`
// grants an activated ability to the enchanted permanent. What is missing is
// a cost it could carry. `CostPart` taps only the source (`TapSelf`); no
// variant chooses a *set* of other creatures while the cost is paid, and none
// reads a total power off the set chosen. A keyword bit is not the way round
// either — `KeywordSet`'s own doc says parameterized keywords like crew N
// are `AbilityDef` data rather than bits, and the 5 is exactly the
// parameter. The card plays as though the clause were not printed, which
// here leaves it a cleaner removal spell than the printed one rather than an
// inert one: the parenthetical's escape hatch is the half that goes missing,
// so the enchanted permanent is never crewed and so never a creature again.
//
// NOT SUPPORTED: the subtypes a card type takes away with it when it goes.
// `Modifier::RemoveType` clears bits in the projected `types` and leaves
// `subtypes` untouched, and there is no remove-a-subtype modifier at all, so
// an enchanted Human Soldier projects as `Artifact — Human Soldier Vehicle`.
// It is not a creature, which is what every rule reached by this card asks
// first, but a creature-type count that does not also ask for creaturehood
// would still see it.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

/// "Enchant creature or Vehicle" (CR 303.4).
///
/// The second half is not a creature clause: a Vehicle is an artifact
/// subtype and an uncrewed one is no creature at all. That is also what
/// keeps this Aura on the battlefield after it resolves — its own third
/// sentence leaves behind a Vehicle, which is still something it may
/// enchant, so CR 704.5m never takes it.
static ENCHANTABLE: Filter = Filter::Or(&[
    Filter::CREATURE,
    Filter::HasSubtype(subtypes::artifact::VEHICLE),
]);

/// "All other card types": every card type a permanent can have except
/// artifact, which is the one the same sentence grants.
///
/// Instant and sorcery are left out because CR 304.4 and CR 307.4 keep such
/// a card off the battlefield in the first place, and Kindred is in because
/// it is a card type like the rest of them.
static OTHER_CARD_TYPES: TypeSet = TypeSet::CREATURE
    .union(TypeSet::ENCHANTMENT)
    .union(TypeSet::LAND)
    .union(TypeSet::PLANESWALKER)
    .union(TypeSet::BATTLE)
    .union(TypeSet::KINDRED);

card!(
    index = index::SWIFT_RECONFIGURATION,
    oracle_id = "5d47e820-913f-441a-a6cc-37ab3181d79a",
    scryfall_id = "975dcfab-0281-4fee-92aa-021ea6c524c7",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Swift Reconfiguration",
        mana_cost = mana!("{W}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    keywords = KeywordSet::FLASH,
    coverage = Coverage::Partial("crew 5 is not written: the Vehicle can never be crewed"),
    abilities = &[
        // "Enchant creature or Vehicle" (CR 303.4): an Aura spell picks the
        // permanent it will enchant as it is cast, and arrives attached to
        // it. `Effect::AttachSelf` reads the resolution's first target, and
        // the resolving spell object *is* the permanent that then goes to
        // the battlefield — `move_object` clears `attached_to` only on the
        // way *off* the battlefield, so the attachment survives the arrival.
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&ENCHANTABLE),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&ENCHANTABLE)))
        ),
        // One printed sentence, one layer: CR 613.1d puts all three of these
        // in layer 4, `Modifier::layer` derives that from the modifier, and
        // the order they sit in says nothing. Here it is safe that it says
        // nothing, which is worth stating because it usually is not:
        // `RemoveType` only clears bits `AddType` does not set, and
        // `AddSubtype` writes the subtype list rather than the type mask, so
        // no two of the three can undo each other whichever way they sort.
        // `Filter::AttachedToBySource` is the enchanted permanent — the same
        // handle the pool's Equipment and the other two Auras grant through.
        static_ability!(
            Filter::AttachedToBySource,
            Modifier::AddType(TypeSet::ARTIFACT)
        ),
        static_ability!(
            Filter::AttachedToBySource,
            Modifier::AddSubtype(subtypes::artifact::VEHICLE)
        ),
        static_ability!(
            Filter::AttachedToBySource,
            Modifier::RemoveType(OTHER_CARD_TYPES)
        ),
    ],
);

// Engine-level coverage belongs in `card_tests`: flash Swift Reconfiguration
// onto an opposing creature and read the enchanted permanent's projected
// types back off the view — artifact and Vehicle in, creature gone — then
// check it can no longer be declared as an attacker, and that the Aura is
// still on the battlefield once it has stopped being a creature, which is
// the one thing about this card that a wrong `Filter` would take away.
