//! The per-seat game view: the wire contract between a game host and a client.
//!
//! # Why this crate exists
//!
//! A client cannot recompute rules. Power/toughness after anthems, the name of
//! a copied permanent, the types of an animated land — all of that is the
//! result of the engine's layer projection (CR 613) and is *not* derivable from
//! the printed card. The view therefore carries **projected characteristics**,
//! not just a card reference.
//!
//! # Hidden information (CR 400.2)
//!
//! The view is built per seat and is the only thing that reaches a client.
//! Anything a seat may not know must be *unrepresentable* here, not merely
//! omitted by a caller:
//!
//! - Library contents are never present, only counts — except the ones a
//!   pending choice is holding in front of this seat, which arrive in
//!   [`PlayerView::looking_at`] and leave again with the question.
//! - Another seat's hand is only a count, with the same exception.
//! - A face-down permanent reveals its identity only to controllers who are
//!   entitled to look ([`PublicObject::card`] is `None` otherwise).
//!
//! # Layering
//!
//! This crate is deliberately free of the rules kernel: it depends only on
//! `baylee-core` id types plus `serde`. A client that renders a duel needs the
//! engine's choice taxonomy as well, but an application that only *displays*
//! game state (a spectator overlay, an MMO world showing a duel in progress)
//! needs nothing but this crate.

#![warn(missing_docs)]

pub use baylee_core::mana::ManaPayment;

mod combat;
mod counters;
mod game_static;
mod log;
mod objects;
mod player_view;
mod seats;
mod seen;
mod shown_hands;
mod status;
mod turn;
mod wire;

pub use combat::{AttackerView, BlockerView, CombatView};
pub use counters::{CounterEntry, CounterKind};
pub use game_static::{Finish, GameStatic, PrintEntry, SeatIdentity};
pub use log::{
    ClockAnswer, LOG_TAIL_CAP, LogAbility, LogEntry, LogEvent, LogFrom, LogObject, LogPlace,
    LogTail, LogTarget, LogZone, PolicyAct, PolicyAnswer,
};
pub use objects::{
    BoardMana, CardIdentity, DamageSourceView, GrantSource, GrantedMana, HandObject, NamedFace,
    ObjectSummaryKey, PublicObject, RulesFace, StackItem, StackText, TokenAbility,
};
pub use player_view::{PlayerView, SeatClock, TargetingContext, WordChange};
pub use seats::{CommanderDamage, CommanderView, HouseAnswer, LossCause, ManaPoolView, SeatView};
pub use seen::{Seen, SeenIn};
pub use shown_hands::{SeatSetting, SharedHand};
pub use status::ObjectStatus;
pub use turn::{DayNight, Phase, Step};

/// Protocol version of the view payload. Bumped on any breaking change so a
/// client can refuse a host it cannot render rather than mis-rendering it.
///
/// 26 added [`GameStatic::decision_secs`] and [`GameStatic::reconnect_secs`]
/// (#98) — the two limits the table plays at. Since #85 a room picks its own
/// clock, so both are per table, and a client was told neither.
///
/// They are on this payload rather than on a view because they are known at
/// join and constant for the game, and because the reconnect window is the
/// one number a client cannot be sent when it needs it: it is disconnected
/// for exactly the window it would be counting. Three ways into a game have
/// no lobby row to read them from at all — `dev-table`, a rematch, and
/// taking over an AI chair — which is why the lobby listing is not enough.
///
/// 25 added [`PlayerView::decision_remaining_ms`] (#68). The decision clock
/// has run since the engine moved into a process of its own and reached no
/// seat at all, so a player saw nothing for ten minutes and then lost a
/// decision in silence. Nothing else moved: the number is computed by
/// whoever owns the wall clock and handed in, because the host that builds
/// this view is forbidden one.
///
/// 24 added [`PlayerView::owed`] (#92): what the seat named by
/// [`PlayerView::awaiting`] still has to pay, when the engine has opened a
/// CR 605.3a mana window for it. The window is an ordinary priority round by
/// design, which is what made it invisible — a seat that had just agreed to
/// pay ward's tax was handed priority over two untapped Plains with nothing
/// castable and no stated reason to tap them, and passed.
///
/// 23 renamed `PlayerView::priority` to [`PlayerView::awaiting`] and fed it
/// from the pending question rather than from priority, and deleted
/// `is_my_priority`, which had no caller in the tree (#90). The three readers
/// of the old field all already meant "who are we waiting for", so the rename
/// is what makes them right — no logic moved with it.
///
/// 22 widened [`SubtypeSet`] from 512 bits to 1024 (#43). The widening is the
/// half this constant can defend: the array on the wire grows from eight
/// numbers to sixteen, and a client reading the old shape is turned away.
/// What it cannot defend is the other half of that ticket — a *renumbered*
/// table leaves the struct exactly as it was, so two builds agree on the shape
/// and disagree on what a number means. That is why subtype ids became
/// append-only in the same commit rather than trusting this number to carry
/// it.
/// 27 adds cosmetic holographic, glitter and galaxy print finishes.
/// 28 adds [`PublicObject::rules`] and `StackItem::Ability::rules`, the card
/// an object's abilities are printed on — the copied card's for a copy — and
/// makes `StackItem::Ability::text` an index into *that* card's sentences.
/// 29 replaces `SeatView::has_lost` with [`SeatView::loss`], which says why
/// (the bool is now the method [`SeatView::has_lost`]), and adds
/// [`SeatView::house_answered`], so a game lost while the house was answering
/// for a seat is not drawn as one the player played and lost (#83).
/// 30 adds [`PublicObject::flashback`], what the viewing seat may pay to cast
/// a card from its graveyard: the engine lists that cast only once its price
/// is floating, so a planner that could not see it never tapped for it
/// (#242).
/// 31 adds [`PublicObject::grants`], who granted each granted ability and in
/// which sentence, so a client draws the grantor's text on that row (#212).
/// 32 adds [`PlayerView::deciding`], the seats still deciding their opening
/// mulligan, which every seat now answers at once (#257). While it is not
/// empty, [`PlayerView::awaiting`] is per seat: this seat while it has a
/// question open, `None` once it has kept. So a seat that has kept is told no
/// remainder.
/// 33 adds the game log (#262): [`LogTail`] and what it carries. It is not a
/// field of [`PlayerView`], which stays a snapshot; it travels beside the
/// view in the same envelope, and an agent answering from a view never sees
/// it.
/// 34 adds teammates' hands shown on request (#265):
/// [`PlayerView::shared_hands`] and the three sets beside it, and
/// [`SeatSetting`], what a seat sends to show, withdraw, ask or decline.
/// 35 adds [`PlayerView::policy_acts`], what this seat's own per-ability
/// policies answered for it since it last answered by hand (#234).
/// 36 makes the game log read like a chat (#300): [`LogEntry::at`], when
/// the host wrote a line; where a land was played or a spell cast from
/// ([`LogFrom`]); where in a library a card went ([`LogPlace`]); and which
/// registry token a line names.
/// 37: face-up log events and authoritative targeting context.
/// 38 adds the public creature type named for a permanent.
/// 39 adds the public suspend state of exiled cards.
/// 40 adds explicitly revealed library tops, never library contents.
/// 41 adds [`PublicObject::chosen_name`], the card name chosen for a
/// permanent as it entered (Pithing Needle), as the card and face it names.
/// 42 adds [`PublicObject::unlocked_doors`], which halves of a Room are
/// unlocked (CR 709.5c).
/// 43 writes every mana cost in a view ([`PublicObject::flashback`],
/// [`PlayerView::owed`]) as its notation, `"{2}{U}{U}"`, and no longer as
/// the sixteen-slot list a replicated cost overflowed.
/// 44 adds the attacking bands (CR 702.22c): [`CombatView::bands`], and
/// the log line an attacker joining a band writes, [`LogEvent::Banded`].
/// Compatible addition: `SeatView::no_max_hand_size` defaults to false when
/// absent; older readers ignore the additional JSON field.
/// Compatible addition: the public chosen opponent defaults to absent.
/// Version 45 adds public permanent choices (`LogEvent::CardsKept`).
/// Compatible addition: [`ManaPoolView::spending`] defaults to exact colors;
/// older readers ignore this extra field and retain their conservative
/// planning. Existing mana counts, enum variants and field types are unchanged.
/// Version 46 widens the [`LogEvent::Damage`] amount from `u16` to `u32` so
/// large paid X values retain their full damage amount in public logs.
/// Version 47 distinguishes fixed mana debts from optional payments of any
/// amount in [`PlayerView::owed`].
/// Version 48 accompanies explicit damage replacement ordering and
/// simultaneous prevention allocation decisions in the choice protocol.
/// Version 49 adds exact, entitled descriptions of damage-source incarnations.
/// Version 50 makes stack and retarget targets versioned and adds historical
/// target descriptions in `PlayerView::target_objects`.
/// Version 51 adds finite damage redirection to the damage-effect choices.
/// Version 52 accompanies temporary special-action offers and answers.
/// Version 53 distinguishes the answering actor from the resource player,
/// projects hands inspected through control, and widens mana counters.
/// Version 54 leaves a [`PublicObject`]'s fields out of the JSON while they
/// are at their default (`None`, empty, `false`, zero) and reads a missing
/// one as that default (`wire.rs`): a reader of 53 requires some of them.
/// Version 55 adds the game log's [`LogEvent::BecameMonarch`], a variant a
/// reader of 54 cannot parse.
/// Compatible addition: [`PlayerView::casting`], the card this seat may take
/// back with `PlayerAction::CancelCast`, absent when `None`; a reader of 55
/// without it ignores it.
/// Compatible addition: [`PlayerView::clocks`], every seat's running
/// decision clock (owner, 08.10.2026), absent while empty; a reader of 55
/// without it ignores it.
///
/// [`SubtypeSet`]: baylee_core::types::SubtypeSet
pub const VIEW_VERSION: u32 = 55;

// ------------------------------------------------------------------- targets

/// Exact target identities shared with engine choices.
pub use baylee_core::ids::TargetRef;

#[cfg(test)]
mod tests;
