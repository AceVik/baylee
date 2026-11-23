//! Objects an instruction names without targeting them anew, and what an
//! Aura, Equipment or Fortification entering the battlefield attached is
//! attached to (CR 303.4f–g, CR 301.5c, CR 301.6).

use super::{Filter, PlayerRel, TargetSpec, ZoneSel};

/// The objects an instruction acts on, read as it resolves.
///
/// Nothing here targets: [`Self::Spec`] reads the targets the ability
/// already chose (or the source, or the event object), [`Self::Found`] the
/// cards an earlier instruction of the same resolution searched for, chose
/// or created, and [`Self::AllInZone`] every matching object in a zone.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum DefinedObjects {
    /// Every object `TargetSpec` names: all chosen targets of the active
    /// target group, `ThisObject` (the source, or its public-zone successor
    /// for a moving instruction, CR 400.7e), or `EventObject`.
    Spec(TargetSpec),
    /// Every object recorded in `SubjectContext::found` earlier in this
    /// resolution whose recorded incarnation still exists (CR 400.7): the
    /// cards a search found, a choice chose, or a token instruction made.
    Found,
    /// Every object matching `filter` in `zone`, in canonical zone order.
    AllInZone {
        /// Zone(s) scanned.
        zone: ZoneSel,
        /// Filter each candidate must match there.
        filter: &'static Filter,
    },
}

/// What an Aura, Equipment or Fortification entering the battlefield
/// attached is attached to.
///
/// CR 303.4f: an Aura put onto the battlefield other than by resolving as an
/// Aura spell enters attached to what the instruction names, or, when the
/// instruction leaves it open, to what the player putting it there chooses
/// as it enters. Neither is targeting (CR 115.1), so hexproof and shroud do
/// not stop it; the Aura's own enchant ability and protection do. CR 303.4g:
/// an Aura with nothing legal to enchant stays in the zone it was in (a token
/// is not created). Equipment and Fortifications enter and are then attached
/// to a legal permanent (CR 301.5c, CR 301.6); with none, they stay
/// unattached on the battlefield.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum AttachHost {
    /// The object or player `TargetSpec` names: `ThisObject` ("attached to
    /// this creature"), `EventObject` ("attached to it"), a chosen object
    /// target ("attached to target creature"), or a chosen player target
    /// ("attached to target opponent", a Curse).
    Spec(TargetSpec),
    /// A permanent matching the filter, chosen as the attachment enters by
    /// the player putting it onto the battlefield (CR 303.4f).
    ChosenPermanent(&'static Filter),
    /// A player the relation names, chosen as the Aura enters by the player
    /// putting it onto the battlefield (an enchant-player Aura, CR 303.4f).
    ChosenPlayer(PlayerRel),
}
