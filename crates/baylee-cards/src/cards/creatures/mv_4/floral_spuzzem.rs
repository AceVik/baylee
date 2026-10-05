//! Floral Spuzzem — {3}{G} — Creature — Elemental
//! Oracle: Whenever this creature attacks and isn't blocked, you may destroy target artifact defending player controls. If you do, this creature assigns no combat damage this turn.
//! Set: LEG #187 — Legends | Scryfall ID: d141b9e3-7129-41e5-8b44-d3867e1c7e1d | Oracle ID: 994de451-14f9-466f-a56e-da052b4666e5
// PARTIAL — the whole ability is off the card, see the NOT SUPPORTED line below.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::FLORAL_SPUZZEM,
    oracle_id = "994de451-14f9-466f-a56e-da052b4666e5",
    scryfall_id = "d141b9e3-7129-41e5-8b44-d3867e1c7e1d",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    coverage = Coverage::Partial(
        "no Trigger hears \"attacks and isn't blocked\" (Trigger::Attacks \
         fires before blockers are declared) and no Effect or Modifier makes \
         a creature assign no combat damage this turn"
    ),
    faces = &[face!(
        name = "Floral Spuzzem",
        mana_cost = mana!("{3}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::ELEMENTAL],
        power = Some(2),
        toughness = Some(2),
    ),],
);

// NOT SUPPORTED: "Whenever this creature attacks and isn't blocked, you may
// destroy target artifact defending player controls. If you do, this
// creature assigns no combat damage this turn." — the destroy half is
// expressible (`Effect::MayDo` around `Effect::destroy` with
// `TargetSpec::Object(&Filter::And(&[Filter::ARTIFACT,
// Filter::ControlledByDefendingPlayer]))`), but the trigger head is not:
// `Trigger::Attacks` fires at declare attackers (CR 508.1), where
// `Filter::Unblocked` is not yet true (CR 509.1h), and no trigger or delayed
// trigger hears "attacks and isn't blocked". The rider has no vocabulary
// either: nothing makes a creature assign no combat damage. The ability
// comes off the card rather than firing at the wrong moment or paying the
// destroy without the drawback.
