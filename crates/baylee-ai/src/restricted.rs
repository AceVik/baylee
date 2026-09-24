//! Restricted mana (CR 106.6): mana that may be spent only on what a filter
//! names — Ancient Ziggurat's on creature spells, Mishra's Workshop's on
//! artifact spells.
//!
//! The view says how much restricted mana floats and under which colour, and
//! never what it may pay for; `manaplan` counts none of it for that reason
//! (its rule 2). So the restriction is read off the ability that makes the
//! mana, **before** the tap, and a restricted source is tapped only
//!
//! - for a spell its filter admits, as far as the view can tell — an
//!   unreadable filter (a chosen type, a commander's types) is a no, because
//!   mana the payment then refuses is a tap thrown away;
//! - once per spell;
//! - as the last tap of the plan. After it the engine's own merge
//!   (`casting::spendable_pool`) makes the spell castable, so the planner
//!   never has to see the restricted half of the pool. A restricted tap sent
//!   earlier would float mana the next plan cannot count, and the spell
//!   would be abandoned with a land already tapped for it.
//!
//! A spell whose price is floated before the cast (an X, a kicker) is paid
//! with unrestricted sources only, for the same reason: the check that the
//! pool covers the price reads the pool the planner reads.

use std::borrow::Cow;

use baylee_cards_dsl::{Effect, Filter, ZoneRef};
use baylee_core::ids::ObjectId;
use baylee_core::types::SubtypeSet;
use baylee_view::{ObjectStatus, PlayerView, PublicObject};

use crate::HeuristicAgent;

/// What the mana `effects` add may be spent on, when it is restricted.
pub(crate) fn only_for(effects: &[Effect]) -> Option<&'static Filter> {
    effects.iter().find_map(|effect| match effect {
        Effect::AddMana {
            restriction: Some(restriction),
            ..
        } => Some(restriction.filter),
        _ => None,
    })
}

impl HeuristicAgent {
    /// Whether mana `source` makes under `filter` may pay for the spell
    /// `spell`, as far as the view can tell.
    pub(crate) fn admits(
        &self,
        view: &PlayerView,
        filter: &Filter,
        source: Option<ObjectId>,
        spell: ObjectId,
    ) -> bool {
        spell_object(view, spell).is_some_and(|object| {
            self.filter_matches(filter, view, &object, ZoneRef::Stack, source) == Some(true)
        })
    }
}

/// The spell `id` as a restriction reads it: the card about to be cast.
///
/// A card outside the hand is already a [`PublicObject`] in the view. One in
/// hand is a `HandObject`, which carries no subtypes or supertypes, so those
/// come from the printed face — what the engine's filter reads of a card in
/// hand as well.
fn spell_object(view: &PlayerView, id: ObjectId) -> Option<Cow<'_, PublicObject>> {
    if let Some(object) = view.object(id) {
        return Some(Cow::Borrowed(object));
    }
    let held = view.hand.iter().find(|c| c.id == id)?;
    let face = crate::policy::face(held.card)?;
    let mut subtypes = SubtypeSet::EMPTY;
    for &subtype in face.subtypes {
        subtypes.insert(subtype);
    }
    Some(Cow::Owned(PublicObject {
        id,
        card: Some(held.card),
        rules: None,
        name: held.name.clone(),
        controller: view.seat,
        owner: view.seat,
        commander: held.commander,
        status: ObjectStatus::default(),
        types: face.types,
        supertypes: face.supertypes,
        subtypes,
        token: None,
        colors: held.colors,
        mana_value: held.mana_value,
        keywords: baylee_cards::by_index(held.card.index).map_or(0, |def| {
            def.keywords_for_face(usize::from(held.card.face)).bits()
        }),
        power: face.power,
        toughness: face.toughness,
        base_power: face.power,
        base_toughness: face.toughness,
        loyalty: face.loyalty,
        damage: 0,
        counters: Vec::new(),
        attached_to: None,
        targets: Vec::new(),
        stack_item: None,
        summoning_sick: false,
        granted_mana: None,
        board_mana: None,
    }))
}
