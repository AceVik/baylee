//! Nettlecyst — {3} — Artifact — Equipment
//! Oracle: Living weapon (When this Equipment enters, create a 0/0 black Phyrexian Germ creature token, then attach this to it.)
//! Oracle: Equipped creature gets +1/+1 for each artifact and/or enchantment you control.
//! Oracle: Equip {2}
//! Set: MKC #233 — Murders at Karlov Manor Commander | Scryfall ID: 0a7cb0f8-2946-4b00-a192-0b31c8e1ec5c | Oracle ID: 04c7f4fe-2098-4311-866d-6733c08d5178
// PARTIAL — an Equipment that grows with the board: the creature holding it
// gets +1/+1 for each artifact and/or enchantment you control, and equip {2}
// moves it at sorcery speed.
//
// NOT SUPPORTED: `Living weapon` — "create a 0/0 black Phyrexian Germ
// creature token, then attach this to it". The trigger is not what is
// missing. `Effect::AttachSelf` attaches the source to a *chosen target* —
// `resolve` reads `res.targets.first()` — and targets are chosen when the
// ability goes on the stack, so nothing in the effect vocabulary can name a
// token an earlier effect in the same resolution created; the nearest thing
// that reads an object off the resolution rather than off a choice is
// `TargetSpec::EventObject`, and it is about the *triggering* event. There
// is no Germ in `tokens::ALL` either, and a `TokenDef` written here instead
// would be a permanent the client's token-art registry has never heard of.
// The card plays exactly as though the line were not printed: a {3}
// Equipment that needs a creature of its own to hold it.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::NETTLECYST,
    oracle_id = "04c7f4fe-2098-4311-866d-6733c08d5178",
    scryfall_id = "0a7cb0f8-2946-4b00-a192-0b31c8e1ec5c",
    faces = &[face!(
        name = "Nettlecyst",
        mana_cost = mana!("{3}"),
        types = TypeSet::ARTIFACT,
        subtypes = &[subtypes::artifact::EQUIPMENT],
    ),],
    coverage = Coverage::Partial("living weapon is not written: no Germ token is created"),
    abilities = &[
        // Nettlecyst is itself an artifact you control, so it counts itself
        // (the filter says nothing about `Another`), and a permanent that is
        // both an artifact and an enchantment is one object and counts once.
        // `ModifyPTPerCount` counts over the battlefield under the *effect's*
        // controller, which is this Equipment's controller — "you control" —
        // and not the equipped creature's, which a control-change would part
        // company with.
        static_ability!(
            Filter::AttachedToBySource,
            Modifier::ModifyPTPerCount {
                filter: &Filter::ARTIFACT_OR_ENCHANTMENT,
                p: 1,
                t: 1,
            }
        ),
        equip!("{2}"),
    ],
);
