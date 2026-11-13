//! Restricted mana (CR 106.6), as the frozen house reads it: mana that may be
//! spent only on what a filter names — Ancient Ziggurat's on creature spells,
//! Mishra's Workshop's on artifact spells.
//!
//! Every other planner asks the engine what restricted mana may pay for
//! (`LegalActions::restricted_taps`, `restricted_floating`, protocol 43;
//! [`crate::spend`]). The house keeps this reading so `sharp-2026-11` answers
//! as it was recorded: the view says how much restricted mana floats and under
//! which colour, and never what it may pay for, so the restriction is read off
//! the ability that makes the mana, **before** the tap, and a restricted
//! source is tapped only
//!
//! - for a payment its restriction admits, as far as the view can tell: the
//!   kind of payment first (`ManaUse`: a spell, an ability, either), then the
//!   filter. An unreadable filter (a commander's types) is a
//!   no, because mana the payment then refuses is a tap thrown away;
//! - once per spell, since the house plans again after each tap;
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

use baylee_cards_dsl::{Effect, KeywordSet, ManaRestriction, ManaUse, ZoneRef};
use baylee_core::ids::ObjectId;
use baylee_core::types::SubtypeSet;
use baylee_view::{ObjectStatus, PlayerView, PublicObject};

use crate::HeuristicAgent;

/// What the mana `effects` add may be spent on, when it is restricted. A
/// rider alone restricts nothing (#232): that source is an ordinary one.
pub fn only_for(effects: &[Effect]) -> Option<ManaRestriction> {
    effects.iter().find_map(|effect| match effect {
        Effect::AddMana {
            restriction: Some(restriction),
            ..
        } if restriction.restricts => Some(*restriction),
        _ => None,
    })
}

impl HeuristicAgent {
    /// Whether mana `source` makes under `restriction` may pay for casting
    /// `spell`, as far as the view can tell: a restriction that pays only
    /// for abilities pays for no spell (CR 106.6), and the rest must name it.
    pub fn admits(
        &self,
        view: &PlayerView,
        restriction: &ManaRestriction,
        source: Option<ObjectId>,
        spell: ObjectId,
    ) -> bool {
        if restriction.uses == ManaUse::LastLinkedExile && !linked_exile(view, source, spell) {
            return false;
        }
        !matches!(
            restriction.uses,
            ManaUse::Abilities | ManaUse::CumulativeUpkeep
        ) && spell_object(view, spell).is_some_and(|object| {
            self.filter_matches(restriction.filter, view, &object, ZoneRef::Stack, source)
                == Some(true)
        })
    }

    /// Whether the same restricted mana may pay for activating an ability of
    /// `activated`, an Ally source or another permanent whose current
    /// characteristics match.
    ///
    /// The kind of payment is asked before the filter, as the engine asks it
    /// (`mana_provenance::admits`). Mana that may be spent "only to cast" a
    /// spell pays for no ability whatever the ability's source is: Delighted
    /// Halfling's coloured mana ("only to cast a legendary spell") does not
    /// pay Yawgmoth's proliferate, nor Mishra's Workshop's ("only to cast
    /// artifact spells") an Armageddon Clock's `{4}`. The filter alone said
    /// yes to both, so the plan tapped the source and the payment refused
    /// its mana (NEW-05, 3.11.2026).
    pub fn admits_activation(
        &self,
        view: &PlayerView,
        restriction: &ManaRestriction,
        source: Option<ObjectId>,
        activated: ObjectId,
    ) -> bool {
        let filter = match restriction.uses {
            // Powerstone's: any payment but a nonartifact spell's.
            ManaUse::NonSpellPaymentsOrMatchingSpells => return true,
            ManaUse::Spells | ManaUse::CumulativeUpkeep | ManaUse::LastLinkedExile => {
                return false;
            }
            ManaUse::Abilities | ManaUse::SpellsOrAbilities | ManaUse::CostsFromMatchingSource => {
                restriction.ability_filter.unwrap_or(restriction.filter)
            }
        };
        let zone = if view.battlefield.iter().any(|o| o.id == activated) {
            ZoneRef::Battlefield
        } else if view.graveyards.iter().flatten().any(|o| o.id == activated) {
            ZoneRef::Graveyard
        } else if view.command.iter().flatten().any(|o| o.id == activated) {
            ZoneRef::Command
        } else if view.exile.iter().flatten().any(|o| o.id == activated) {
            ZoneRef::Exile
        } else {
            ZoneRef::Hand
        };
        spell_object(view, activated).is_some_and(|object| {
            self.filter_matches(filter, view, &object, zone, source) == Some(true)
        })
    }
}

/// A linked-exile restriction names an object, not a card name or filter.
/// Without the host's exact link, refusing is safer than stranding a tap.
fn linked_exile(view: &PlayerView, source: Option<ObjectId>, spell: ObjectId) -> bool {
    source
        .and_then(|source| view.object(source))
        .and_then(|object| object.board_mana.as_ref())
        .is_some_and(|mana| mana.restricted_to == Some(spell))
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
    let held = baylee_client_core::decision::known_cards(view).find(|c| c.id == id)?;
    let face = crate::policy::face(held.card)?;
    let keywords = baylee_cards::by_index(held.card.index).map_or(KeywordSet::EMPTY, |def| {
        def.keywords_for_face(usize::from(held.card.face))
    });
    let mut subtypes = SubtypeSet::from_slice(face.subtypes);
    // A changeling is every creature type in a hand too (CR 702.73a,
    // `Characteristics::from_face`), so a Cavern's mana pays for it.
    if keywords.contains(KeywordSet::CHANGELING) {
        subtypes = subtypes.union(SubtypeSet::ALL_CREATURE);
    }
    Some(Cow::Owned(PublicObject {
        battle_protector: None,
        id,
        card: Some(held.card),
        rules: None,
        prepared_spell: None,
        name: held.name.as_str().into(),
        controller: baylee_client_core::decision::resource_player(view),
        owner: baylee_client_core::decision::resource_player(view),
        commander: held.commander,
        status: ObjectStatus::default(),
        types: face.types,
        supertypes: face.supertypes,
        subtypes,
        chosen_subtype: None,
        chosen_name: None,
        chosen_opponent: None,
        chosen_color: None,
        chosen_x: None,
        unlocked_doors: None,
        suspended: false,
        token: None,
        colors: held.colors,
        mana_value: held.mana_value,
        keywords: keywords.bits(),
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
        times_kicked: 0,
        bought_back: false,
        summoning_sick: false,
        ability_mana_spending: baylee_core::mana::ManaSpending::EXACT,
        granted_mana: None,
        board_mana: None,
        flashback: None,
        grants: Vec::new(),
        ability_sources: Vec::new(),
        word_changes: Vec::new(),
    }))
}
