//! Casting and resolution.
//!
//! S2 scope: no modes/targets/X, auto-payment (CR 601.2h compressed).
//! The full stepwise `CastPlan` wizard (modes, alternative/additional
//! costs, targets, X, payment plans) lands with the ability runtime (M1.S3
//! / M2).

use crate::event::{Cause, GameEvent};
use crate::mana_pay;
use crate::object::ObjectKind;
use crate::state::{GameState, StateError};
use crate::turn::Phase;
use crate::zone::{Zone, ZoneLocation, ZonePosition};
use baylee_core::generated::subtypes::land;
use baylee_core::ids::{ObjectId, PlayerId};
use baylee_core::mana::{ManaColor, ManaCost, ManaPool};
use baylee_core::types::TypeSet;

/// Why a card cannot be cast right now (validated before the action).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum CastError {
    /// The card is not in the caster's hand.
    #[error("card is not in your hand")]
    NotInHand,
    /// Timing rules forbid it (main phase, own turn, empty stack).
    #[error("sorcery-speed timing not met")]
    BadTiming,
    /// The mana pool cannot pay the cost.
    #[error("not enough mana")]
    NotEnoughMana,
    /// Costs with {X}/{Y}/{Z} need the full wizard (M1.S3).
    #[error("variable costs not supported yet")]
    VariableCost,
    /// Every way of casting the card fails on its own terms: each one is
    /// either unpayable or has nothing to point at.
    ///
    /// The variant the note beside [`can_cast`]'s face probe has asked for
    /// since it was written — "a `CastError` variant that does not call a
    /// target problem *not enough mana*". A modal spell is where the
    /// difference stopped being cosmetic: Damn on three Swamps could pay for
    /// "destroy target creature" and had nothing to destroy, and could point
    /// the overload at everything and not pay `{2}{W}{W}` for it. Neither
    /// half is a mana problem, and answering with one sent a player looking
    /// for a fourth land.
    #[error("no way to cast this spell")]
    NoWayToCast,
}

/// Whether any effect lets mana be spent as though it were mana of any
/// color (Mycosynth Lattice).
///
/// The payment step has always honoured this; the *affordability* checks
/// did not, so the Lattice's third line was unreachable — the spell it made
/// payable was never offered as castable in the first place.
#[must_use]
pub fn mana_is_wild(state: &GameState) -> bool {
    state
        .effects
        .iter()
        .any(|fx| matches!(fx.modifier, baylee_cards_dsl::Modifier::ManaIsAnyColor))
}

/// The permanents convoke may be paid with (CR 702.51): untapped creatures
/// and artifacts the caster controls, one `{1}` each.
///
/// One function because the offer and the payment must not disagree. The
/// wizard enumerated these to ask which to tap, and `can_cast` did not
/// count them at all — so a convoke spell was offered as castable exactly
/// when its printed cost was already payable, which is the one case convoke
/// is not for. Clever Concealment and Spirit Water Revival were
/// `Coverage::Implemented` and could never actually be convoked.
#[must_use]
pub fn convoke_sources(state: &GameState, player: PlayerId) -> Vec<ObjectId> {
    state
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            state.object(*id).is_some_and(|o| {
                o.controller == player
                    && (o.characteristics().types.contains(TypeSet::CREATURE)
                        || o.characteristics().types.contains(TypeSet::ARTIFACT))
                    && !o.status.contains(crate::object::Status::TAPPED)
            })
        })
        .collect()
}

/// The generic mana this face's keywords can pay for right now: convoke's
/// untapped permanents (CR 702.51) and delve's graveyard (CR 702.66a), one
/// point each.
///
/// One function because two probes ask it and an offer is the difference
/// between their answers. [`can_cast`] decides whether the card appears in
/// `LegalActions` at all; `Engine::cast_options` decides which ways of casting
/// it appear once it has been pressed. Every time those two have counted
/// differently the result has been the same defect in one of its two
/// directions — a card offered and then refused as "no way to cast this
/// spell", or a card never offered that the wizard would have paid for
/// happily. Convoke was fixed by making the count one function and delve was
/// left beside it: Dig Through Time is the pool's only delve card, is
/// `Coverage::Implemented`, and was offered at eight mana or not at all, with
/// a full graveyard doing nothing.
///
/// The *printed* face, because neither keyword can be granted: a copy of a
/// delve spell is not a delve spell.
#[must_use]
pub fn keyword_reduction(
    state: &GameState,
    face: &baylee_cards_dsl::FaceDef,
    player: PlayerId,
) -> u32 {
    let convoke = if face.convoke {
        u32::try_from(convoke_sources(state, player).len()).unwrap_or(u32::MAX)
    } else {
        0
    };
    let delve = if face.delve {
        u32::try_from(state.zones.list(ZoneLocation::Graveyard(player)).len()).unwrap_or(u32::MAX)
    } else {
        0
    };
    convoke.saturating_add(delve)
}

/// The generic mana a cost reduction printed on the card itself takes off
/// right now — Surgical Metamorph's "costs {1} less if you weren't the
/// starting player".
///
/// The other half of the pair above, and it went the same way: the wizard
/// applied it when it built the normal-cost option and the offer did not, so
/// the seat the reduction is *for* was never shown the card at the price it
/// would have paid.
#[must_use]
pub fn printed_reduction(
    state: &GameState,
    face: &baylee_cards_dsl::FaceDef,
    player: PlayerId,
) -> u32 {
    match face.cost_reduction {
        Some(baylee_cards_dsl::CostReduction::NotStartingPlayer(n))
            if player != state.starting_player =>
        {
            n
        }
        _ => 0,
    }
}

/// The non-front faces that are a way of *casting* the card, in face order.
///
/// Two readers ask this — [`can_cast`], deciding whether the card belongs in
/// `LegalActions`, and the wizard's `cast_options`, listing the ways of
/// paying once it has been pressed — and every divergence between them is one
/// of two defects: a card offered and then refused with "no way to cast this
/// spell", or a card never offered that the player would have paid for.
///
/// A land back is *played* rather than cast (CR 305.1) and a disturb back is
/// cast from the graveyard on its own branch (CR 702.146a), so what is left is
/// an MDFC's back (CR 712.11b) and an adventure (CR 715). Whether the card may
/// be cast from where it lies at all is settled before this is asked, so the
/// only zone question left is the one CR 715.3d asks: a card exiled *on its
/// adventure* may be cast as the creature and not as the adventure again —
/// "It can't be cast as an Adventure this way", the rule adds, "although
/// other effects that allow a player to cast it may allow a player to cast it
/// as an Adventure". So the gate is the `Adventure` rider the resolution put
/// on the card *and* the exile it put it in, which is not the same as asking
/// the zone: an Opposition Agent exile and a commander in the command zone
/// are effects of exactly that other kind, and a plain "is it in the hand"
/// test would refuse both the back face the rule grants them. The exile half
/// is there because nothing takes the rider off again — `move_object` clears
/// a copy's characteristics (CR 400.7) and leaves the rider list alone — so
/// a Twining Twins that was cast off its adventure and then bounced would
/// arrive in the hand still wearing it.
pub fn castable_back_faces(
    def: &baylee_cards_dsl::CardDef,
    on_adventure: bool,
) -> impl Iterator<Item = (usize, &baylee_cards_dsl::FaceDef)> {
    def.faces.iter().enumerate().skip(1).filter(move |(_, f)| {
        f.castable_from_hand && !f.types.contains(TypeSet::LAND) && !(on_adventure && f.adventure)
    })
}

/// Whether one face of `card` has enough legal targets to be cast at all.
///
/// A spell whose only target requirement cannot be met is a cast that ends
/// at the targeting step with nothing paid and nothing on the stack, so a
/// card offered on that footing is a button that only ever errors and an
/// agent picks it again on every pass, the state being unchanged.
///
/// Deliberately conservative, and answers `true` whenever it cannot be sure:
/// a modal spell chooses targets per mode, an X-counted requirement depends
/// on a number nobody has picked yet, and a player is not an object. All
/// three stay the wizard's problem.
///
/// The `face` parameter is the whole reason this is not a method on the
/// engine any more. It used to ask face 0 and nothing else, which was the
/// same answer for every way of casting a card — and an adventure
/// (CR 715) is a *different spell* with its own target line on the back of a
/// creature that has none. Swift Spiral has to find a nontoken creature;
/// Twining Twins in front of it has nothing to target, so the front face
/// answered "no requirement, therefore yes" for a mode that would then be
/// refused.
#[must_use]
pub fn face_has_a_legal_target(
    state: &GameState,
    lookup: &impl crate::state::CardLookup,
    player: PlayerId,
    card: ObjectId,
    face: usize,
) -> bool {
    let Some(def) = state
        .object(card)
        .and_then(|o| o.card)
        .and_then(|c| lookup.card(c.index))
    else {
        return true;
    };
    let abilities = def.abilities_for_face(face);
    // A modal spell is answered mode by mode (CR 700.2): the card is castable
    // when *any* one of its modes can be pointed at something, and which one
    // is `cast_options`' question, not this one's.
    let mut modal = abilities.iter().filter_map(|a| match a {
        baylee_cards_dsl::AbilityDef::ModalSpell { modes } => Some(modes),
        _ => None,
    });
    if let Some(modes) = modal.next() {
        return modes
            .iter()
            .any(|mode| requirement_is_reachable(mode.targets, state, player, card));
    }
    let req = abilities.iter().find_map(|a| match a {
        baylee_cards_dsl::AbilityDef::Spell { targets, .. } => *targets,
        _ => None,
    });
    requirement_is_reachable(req, state, player, card)
}

/// Whether one mode of a modal spell (CR 700.2) can be pointed at anything.
///
/// The half of [`face_has_a_legal_target`] a *mode* needs, and the reason it
/// is separate: the card being castable and this particular mode being
/// takeable are two questions, and the wizard has to ask the second one about
/// every button it offers. It did not, which was invisible for as long as a
/// modal spell was also offered a mode-less `Normal` cast that resolved to
/// nothing — Cyclonic Rift on an empty board offered "return target nonland
/// permanent" and refused it with "not enough legal targets" the moment it
/// was pressed.
#[must_use]
pub fn mode_has_a_legal_target(
    state: &GameState,
    lookup: &impl crate::state::CardLookup,
    player: PlayerId,
    card: ObjectId,
    mode: usize,
) -> bool {
    let Some(def) = state
        .object(card)
        .and_then(|o| o.card)
        .and_then(|c| lookup.card(c.index))
    else {
        return true;
    };
    let req = def.abilities.iter().find_map(|a| match a {
        baylee_cards_dsl::AbilityDef::ModalSpell { modes } => modes.get(mode).map(|m| m.targets),
        _ => None,
    });
    match req {
        Some(req) => requirement_is_reachable(req, state, player, card),
        None => true,
    }
}

/// Whether choosing a mode is the **only** way to cast this face (CR 700.2).
///
/// The cast wizard's own guard, lifted out of it so the offer can ask the
/// same question with the same words. It refuses a mode-less `Normal` option
/// for a card whose every effect sits under a mode — such a spell would go
/// hand → stack → graveyard and do nothing — and the offer has to know that,
/// because a card the wizard will only ever price *per mode* is castable
/// exactly when one of its modes is.
///
/// The three clauses hold it to that sentence and no further: a permanent
/// spell arrives on the battlefield whether a mode was picked or not, and a
/// plain [`baylee_cards_dsl::AbilityDef::Spell`] printed beside the modes is
/// what resolution finds first.
#[must_use]
pub fn modes_are_the_only_way(def: &baylee_cards_dsl::CardDef, face: usize) -> bool {
    let Some(f) = def.faces.get(face) else {
        return false;
    };
    let abilities = def.abilities_for_face(face);
    abilities
        .iter()
        .any(|a| matches!(a, baylee_cards_dsl::AbilityDef::ModalSpell { .. }))
        && !abilities
            .iter()
            .any(|a| matches!(a, baylee_cards_dsl::AbilityDef::Spell { .. }))
        && !f.types.is_permanent()
}

/// Whether a target requirement can be met on this board.
///
/// Deliberately conservative, and answers `true` whenever it cannot be sure:
/// no requirement at all, a minimum of zero, an X-counted requirement whose
/// number nobody has picked yet, and anything naming a *player*, who is not
/// an object and is never absent. All of those stay the wizard's problem.
fn requirement_is_reachable(
    req: Option<baylee_cards_dsl::TargetReq>,
    state: &GameState,
    player: PlayerId,
    card: ObjectId,
) -> bool {
    let Some(req) = req else {
        return true;
    };
    if req.min == 0
        || req.count_is_x
        || matches!(
            req.spec,
            baylee_cards_dsl::TargetSpec::AnyPlayer
                | baylee_cards_dsl::TargetSpec::AnyOpponent
                | baylee_cards_dsl::TargetSpec::Player(_)
                | baylee_cards_dsl::TargetSpec::ThisObject
        )
    {
        return true;
    }
    crate::eval::target_options(&req.spec, state, player, card).len() >= req.min as usize
}

/// Whether `pool` covers `cost`, honouring a mana-conversion effect.
pub(crate) fn affordable(state: &GameState, pool: &ManaPool, cost: &ManaCost) -> bool {
    wild_or_not(mana_is_wild(state), pool, cost)
}

/// The pool a *particular spell* may be paid from, or `None` when that is
/// simply the player's pool.
///
/// Restricted mana — Cavern of Souls, Path of Ancestry — does not live in the
/// pool's plain counters, and [`mana_pay::can_pay`] reads nothing else. So a
/// player whose only mana came off a Cavern was offered **nothing to cast**,
/// while `Engine::spend_restricted` on the far side of the wizard would have
/// paid the cost with it without complaint. That is the whole of the fault:
/// the offer and the payment were answering different questions about the
/// same pool.
///
/// They ask the same one here. An entry counts exactly when its own filter
/// matches this spell — the test `spend_restricted` applies, against the same
/// `restriction_info` — so a Cavern naming Ally pays for an Ally and stays
/// invisible to everything else at the table.
///
/// The card is still in a hand rather than on the stack, which is the one
/// difference from the payment site and does not reach these filters: they
/// read characteristics and a chosen subtype, neither of which the stack
/// confers. A rider (uncounterable) is not consulted at all — a rider changes
/// what the spell *becomes*, never whether it can be cast.
/// It is `pub(crate)` because it has **two** callers and they must not
/// disagree: the wizard enumerates the ways to cast a spell with the same
/// probe `can_cast` used to offer it, or a spell is offered in
/// `LegalActions` and then refused as "no way to cast this spell" — which is
/// exactly what the convoke count above this one was written to stop
/// happening.
pub(crate) fn spendable_pool(
    state: &GameState,
    player: PlayerId,
    card: ObjectId,
) -> Option<ManaPool> {
    let pool = &state.players[player.get() as usize].mana_pool;
    if pool.restricted().is_empty() {
        return None;
    }
    let spell = state.object(card)?;
    let mut probe = pool.clone();
    for mana in pool.restricted() {
        let Some(&(source, filter, _)) = state.restriction_info.get(&mana.restriction.0) else {
            continue;
        };
        if crate::eval::matches(filter, state, spell, player, source) {
            if mana.flags.contains(baylee_core::mana::ManaFlags::SNOW) {
                probe.add_snow(mana.color, mana.amount);
            } else {
                probe.add(mana.color, mana.amount);
            }
        }
    }
    Some(probe)
}

/// [`affordable`] with the conversion flag already read.
///
/// Payment sites need it in this shape: `pool` is borrowed mutably there,
/// so the state cannot be read at the same time.
pub(crate) fn wild_or_not(wild: bool, pool: &ManaPool, cost: &ManaCost) -> bool {
    if wild {
        mana_pay::can_pay_wild(pool, cost)
    } else {
        mana_pay::can_pay(pool, cost)
    }
}

/// Pays `cost` from `pool`, honouring a mana-conversion effect.
///
/// `wild` comes from [`mana_is_wild`], read before the pool is borrowed.
/// Whether a face prints a mana cost at all (CR 202.1b).
///
/// A card with **no** mana cost cannot be cast unless something else gives it
/// a cost or lets it be cast without paying one — and that is a different
/// thing from a cost of `{0}`, which is paid by paying nothing and is a
/// perfectly ordinary spell. `ManaCost` keeps the two apart and always has:
/// Ornithopter's `{0}` is one `Generic(0)` symbol, Ancestral Vision's blank
/// is no symbols at all, and `to_string` writes them as `"{0}"` and `""`.
///
/// Nothing read that difference. Every probe here asks only whether the pool
/// covers the cost, and a pool covers a blank cost trivially, so Ancestral
/// Vision — a card whose entire text is a suspend ability and three drawn
/// cards — sat in `legal.castable` from the hand of anybody who reached their
/// main phase, castable for nothing.
///
/// Ask it of a **printed** cost and never of one that has been through the
/// cost arithmetic: `with_less_generic` rebuilds a cost symbol by symbol and
/// does not write back a `Generic(0)`, so `{0}` reduced by nothing comes out
/// blank. Both callers ask before any reduction, which is also what the rule
/// means — a discount that takes `{1}` down to nothing leaves a spell that is
/// cast for free and was never in question here.
pub(crate) fn has_a_printed_cost(cost: &ManaCost) -> bool {
    cost.symbols().next().is_some()
}

pub(crate) fn pay_with(wild: bool, pool: &mut ManaPool, cost: &ManaCost) -> bool {
    if wild {
        mana_pay::pay_wild(pool, cost)
    } else {
        mana_pay::pay(pool, cost)
    }
}

/// Whether `player` may begin casting a spell with these characteristics
/// right now.
///
/// CR 117.1a is the permission: an instant any time its controller has
/// priority, a noninstant during their own main phase with the stack empty
/// (CR 307.1 says the same of a sorcery in particular). Flash (CR 702.8a)
/// moves a card into the first group whatever its types say, Teferi's +1
/// does the same for that player's sorceries, and Teferi's static pulls
/// every opponent's spell back into the second.
///
/// Characteristics rather than an object, because two callers ask this
/// about two different things. [`can_cast`] passes the *projected*
/// characteristics of a card that is sitting in a zone, so a continuous
/// effect that granted it flash is read. The prepared cast passes the
/// **printed** face of a card that has no object at all — the copy does not
/// exist until it is cast, so nothing could have granted it anything — and
/// still wants the two player-scoped effects above, which is the whole
/// reason this is one function and not two.
pub(crate) fn timing_allows(
    state: &GameState,
    player: PlayerId,
    types: TypeSet,
    keywords: baylee_cards_dsl::KeywordSet,
) -> bool {
    // Teferi's restriction forces sorcery-speed timing on everything for
    // opponents.
    let teferi_lock = state.effects.iter().any(|fx| {
        matches!(
            fx.modifier,
            baylee_cards_dsl::Modifier::OpponentsCastAsSorcery
        ) && state.is_opponent(fx.controller, player)
    });
    // Flash (CR 702.8a) makes a card castable whenever an instant could be
    // — Snapcaster Mage, Restoration Angel, the whole free-spell cycle.
    let is_instant =
        types.contains(TypeSet::INSTANT) || keywords.contains(baylee_cards_dsl::KeywordSet::FLASH);
    // Teferi +1: your sorceries have flash until your next turn.
    let sorcery_flash = types.contains(TypeSet::SORCERY)
        && state.effects.iter().any(|fx| {
            matches!(fx.modifier, baylee_cards_dsl::Modifier::SorceriesHaveFlash)
                && fx.controller == player
        });
    if teferi_lock || (!is_instant && !sorcery_flash) {
        let main_phase = matches!(state.turn.phase, Phase::FirstMain | Phase::SecondMain);
        return main_phase && state.turn.active == player && state.zones.stack_is_empty();
    }
    true
}

/// Whether a continuous effect grants `card` flashback (CR 702.34) right now.
///
/// One reader, because a grant arrives in either of two shapes and a caller
/// that knew only one of them offered nothing. `Effect::GrantFlashback`
/// names its target and registers `EffectFilter::ObjectIs`; a card whose
/// sentence is about a *set* — "each instant and sorcery card in your
/// graveyard" — resolves through `bound_now`, which cannot enumerate a
/// filter reaching past the battlefield and so registers
/// `EffectFilter::Dsl`. Both readers of the grant matched `ObjectIs` alone,
/// so the second shape granted flashback to nobody: the effect was in the
/// table, `legal.castable` held no graveyard card, and nothing said why.
///
/// [`crate::effects::applies_to`] is the function that already answers both,
/// and is what `granted_activated`, `eval::protected_from` and the granted-
/// trigger walk ask. A third hand-rolled `matches!` beside them was the
/// defect waiting to happen, and it happened.
#[must_use]
pub fn flashback_granted(state: &GameState, card: ObjectId) -> bool {
    let Some(obj) = state.object(card) else {
        return false;
    };
    state.effects.iter().any(|fx| {
        matches!(fx.modifier, baylee_cards_dsl::Modifier::GrantsFlashback)
            && crate::effects::applies_to(state, fx, obj)
    })
}

/// Whether `card` can be cast by `player` right now (printed cost or any
/// alternative/mode).
///
/// # Errors
/// [`CastError`] describing the first legality violation.
#[allow(clippy::too_many_lines)] // one gate per rule; splitting hides the list
pub fn can_cast(
    state: &GameState,
    lookup: &impl crate::state::CardLookup,
    player: PlayerId,
    card: ObjectId,
) -> Result<(), CastError> {
    let obj = state.object(card).ok_or(CastError::NotInHand)?;
    let in_hand = obj.zone == Zone::Hand && obj.zone_owner == Some(player);
    let in_own_graveyard = obj.zone == Zone::Graveyard && obj.zone_owner == Some(player);
    // Flashback (CR 702.34): a granted card may be cast from its owner's
    // graveyard.
    let flashback_ok = !in_hand && in_own_graveyard && flashback_granted(state, card);
    // Disturb (CR 702.146): a face with disturb is castable from the
    // owner's graveyard.
    let disturb_ok = !in_hand
        && in_own_graveyard
        && obj
            .card
            .and_then(|c| lookup.card(c.index))
            .is_some_and(|def| def.faces.iter().any(|f| f.disturb));
    // Adventure (CR 715): a card on an adventure may be cast from exile.
    let on_adventure =
        obj.zone == Zone::Exile && obj.riders.contains(&crate::object::Rider::Adventure);
    let adventure_ok = !in_hand && on_adventure;
    // Opposition Agent: cards exiled by the takeover are playable by the
    // agent from exile.
    let takeover_ok = !in_hand
        && obj.zone == Zone::Exile
        && obj
            .riders
            .iter()
            .any(|r| matches!(r, crate::object::Rider::PlayableFromExileFor(p) if *p == player));
    // Commander (CR 903.8): a commander in the command zone may be cast
    // from there by its owner, at the same timing it would have from a
    // hand. The marker list is what makes it a commander — an emblem is in
    // the same zone and is not castable by anybody.
    let commander_ok = !in_hand
        && obj.zone == Zone::Command
        && obj.zone_owner == Some(player)
        && state
            .commanders
            .get(player.get() as usize)
            .is_some_and(|cs| cs.iter().any(|c| c.object == card));
    if !in_hand && !flashback_ok && !disturb_ok && !adventure_ok && !takeover_ok && !commander_ok {
        return Err(CastError::NotInHand);
    }
    let c = obj.characteristics();
    // Lands can never be cast as spells (CR 305.1).
    if c.types.contains(TypeSet::LAND) {
        return Err(CastError::BadTiming);
    }
    // Timing (CR 601.3). Read off the projected characteristics, so a
    // granted flash counts.
    if !timing_allows(state, player, c.types, c.keywords) {
        return Err(CastError::BadTiming);
    }
    // Restricted mana this spell may be paid with counts towards it; see
    // [`spendable_pool`].
    let with_restricted = spendable_pool(state, player, card);
    let pool = with_restricted
        .as_ref()
        .unwrap_or(&state.players[player.get() as usize].mana_pool);
    // Commander tax (CR 903.8). A cost *increase*, so it lands on every way
    // of casting the card — printed cost, alternative cost and mode alike
    // (CR 601.2f) — which is why it is folded into each probe below rather
    // than into the first one.
    let tax = commander_tax(state, player, card);
    // Convoke and delve are *reductions* of the generic part, so they go on
    // the same probes the tax does and in the other direction. Read off the
    // printed face: a granted convoke does not exist.
    let printed = obj.card.and_then(|c| lookup.card(c.index));
    let printed_face = printed.map(|def| &def.faces[0]);
    let reduction = printed_face.map_or(0, |face| keyword_reduction(state, face, player));
    let probe = |cost: &ManaCost| {
        affordable(
            state,
            pool,
            &cost.with_more_generic(tax).with_less_generic(reduction),
        )
    };
    // A disturb cast is not the front face at any price. CR 702.146 casts the
    // card *transformed*, for the back's disturb cost, and `cast_options` has
    // always known it — its disturb branch returns the backs and nothing
    // else, so there is no front-cost option here for this probe to be
    // agreeing with. It asked about the front's mana cost anyway: Mirrorhall
    // Mimic is `{3}{U}` in front of a `{3}{U}{U}` disturb, so four mana put
    // it in `legal.castable` and the wizard then refused it with "no way to
    // cast this spell".
    if disturb_ok {
        let affordable_disturb = printed.is_some_and(|def| {
            def.faces.iter().enumerate().skip(1).any(|(i, f)| {
                f.disturb
                    && probe(&f.mana_cost.with_x(0))
                    && face_has_a_legal_target(state, lookup, player, card, i)
            })
        });
        return if affordable_disturb {
            Ok(())
        } else {
            Err(CastError::NotEnoughMana)
        };
    }
    // Printed cost probed with X = 0, and after a reduction printed on the
    // card itself; the full payment is validated when the wizard finishes.
    // A face with no printed cost has no normal way to be cast at all
    // (CR 202.1b) and falls straight through to the alternatives.
    let normal_cost = c
        .mana_cost
        .with_less_generic(printed_face.map_or(0, |face| printed_reduction(state, face, player)));
    // `c.mana_cost` and not `normal_cost`: the question is what the card
    // *prints*, and cost arithmetic does not preserve the answer —
    // `with_less_generic` rebuilds a cost symbol by symbol and drops a
    // `Generic(0)`, so `{0}` reduced by nothing is indistinguishable from a
    // blank. Asking before the arithmetic is also the right rule, because a
    // reduction that takes `{1}` down to nothing leaves a spell that is cast
    // for free and was always castable.
    // A card whose only spell ability is modal has no normal way to be cast
    // at all — `cast_options` offers none — so its printed price is not an
    // answer about the card, and taking it as one is how Damn was offered on
    // a board that could take neither of its modes. The price `{B}{B}`
    // belongs to "destroy target creature", which had nothing to destroy; the
    // mode that needed no target was `{2}{W}{W}` on three Swamps. Both
    // questions were asked and both were answered yes, about different modes.
    let modal_only = printed.is_some_and(|def| modes_are_the_only_way(def, 0));
    if modal_only || !has_a_printed_cost(&c.mana_cost) || !probe(&normal_cost.with_x(0)) {
        // Alternative costs may still make it castable (pitch/evoke). The
        // wizard computes the exact options; this decides only whether there
        // is one, and asks about the whole cost to do it.
        let Some(card_ref) = obj.card else {
            return Err(CastError::NotEnoughMana);
        };
        let Some(def) = lookup.card(card_ref.index) else {
            return Err(CastError::NotEnoughMana);
        };
        let face = &def.faces[0];
        // Mana is not the whole of an alternative cost, and this probe used
        // to behave as though it were: a Force of Will with no other blue
        // card in hand has a zero mana cost, so it went into
        // `legal.castable` and the wizard then reversed the cast at the pitch
        // stage. Every part is asked about here, the way every condition is
        // asked about below — the two halves of the same offer.
        let any_alt = face.alternative_costs.iter().any(|alt| {
            // Every condition, not merely the one that was written first.
            // `CommanderControlled` fell through the old `!matches!` as
            // "true", so Flawless Maneuver and Fierce Guardianship were
            // offered as castable with no commander anywhere — and the
            // wizard, which does check the condition, then found no way to
            // cast them at all.
            let condition_ok = match alt.condition {
                baylee_cards_dsl::AltCondition::Always => true,
                baylee_cards_dsl::AltCondition::NotYourTurn => state.turn.active != player,
                baylee_cards_dsl::AltCondition::CommanderControlled => {
                    controls_a_commander(state, player)
                }
            };
            condition_ok
                && probe(&alt.cost.mana)
                && alternative_parts_payable(state, player, card, alt.cost.parts)
        });
        // Affordable *and* pointable, of the **same** mode. A mode's price
        // and a mode's target line are the two halves of one way to cast the
        // card, and this probe used to ask only the first while
        // `has_a_legal_target` asked the second of whichever mode happened to
        // answer yes — so a board where one mode was payable and a *different*
        // one was targetable offered a card the wizard then refused. It is the
        // same intersection `cast_options` makes two files away, which is the
        // half that was already right.
        let any_mode = def.abilities.iter().any(|a| match a {
            baylee_cards_dsl::AbilityDef::ModalSpell { modes } => {
                modes.iter().enumerate().any(|(i, m)| {
                    probe(&m.cost_override.unwrap_or(face.mana_cost).with_x(0))
                        && mode_has_a_legal_target(state, lookup, player, card, i)
                })
            }
            _ => false,
        });
        // And every other face the wizard would offer, which this probe knew
        // nothing about. Twining Twins is a `{2}{U}{U}` creature in front of
        // a `{1}{W}` instant, so the adventure was unreachable on exactly the
        // boards it is for: the ones where the creature cannot be paid for.
        //
        // Timing is still the front face's alone, here and in the wizard, so
        // a `{1}{W}` instant on the back of a creature is a sorcery-speed
        // instant. The two agree about that, which is what this probe is for;
        // that they agree on something wrong is a separate fault.
        //
        // The target line is asked per face and beside the price, because
        // both have to hold of the *same* face for it to be a way of casting
        // the card. What is still coarse is the other direction:
        // `compute_legal` asks `has_a_legal_target` about face 0 before it
        // gets here, so a front that cannot be targeted keeps a targetable
        // back off the offer. No card in the pool is that shape, and closing
        // it means folding the face-0 question into the probes below and
        // giving `CastError` a variant that does not call a target problem
        // "not enough mana".
        let any_face = castable_back_faces(def, on_adventure).any(|(i, f)| {
            probe(&f.mana_cost.with_x(0)) && face_has_a_legal_target(state, lookup, player, card, i)
        });
        if !any_alt && !any_mode && !any_face {
            // Which of the two refused matters to whoever reads it. A mode
            // that was affordable and had nothing to point at is not a
            // player one land short, and telling them it is sends them
            // looking for the land.
            let a_mode_was_affordable = def.abilities.iter().any(|a| match a {
                baylee_cards_dsl::AbilityDef::ModalSpell { modes } => modes
                    .iter()
                    .any(|m| probe(&m.cost_override.unwrap_or(face.mana_cost).with_x(0))),
                _ => false,
            });
            return Err(if a_mode_was_affordable {
                CastError::NoWayToCast
            } else {
                CastError::NotEnoughMana
            });
        }
    }
    Ok(())
}

/// How many lands `player` may play this turn (CR 305.2).
///
/// One, "however, continuous effects may increase this number" — which the
/// rule says in those words, so the number is read off the effect table
/// rather than being a constant with an exception bolted to it. Two such
/// effects add up: nothing in CR 305.2 makes them redundant with each other
/// the way two copies of a keyword are.
///
/// Saturating, because the sum is a `u8` and what it feeds is "may I play
/// one more": a table holding 255 extra land drops is a player who may play
/// lands all day either way, and wrapping to nought is the one answer that
/// would be wrong.
#[must_use]
pub fn land_drops_allowed(state: &GameState, player: PlayerId) -> u8 {
    state
        .effects
        .iter()
        .filter(|fx| fx.controller == player)
        .fold(1u8, |total, fx| match fx.modifier {
            baylee_cards_dsl::Modifier::ExtraLandDrops(n) => total.saturating_add(n),
            _ => total,
        })
}

/// Whether `player` has a land drop left this turn (CR 305.2a).
///
/// One predicate with two ends asking it: the offer in
/// `abilities::compute_legal`, which decides whether a land is in
/// `legal.lands` at all, and [`play_land`], which refuses an answer nobody
/// offered. Written out they were `== 0` and `>= 1` — the same sentence
/// exactly once, so the moment the limit stopped being one, one of the two
/// would have gone on reading the old rule and the difference would show as
/// a land the engine offers and then refuses.
#[must_use]
pub fn has_a_land_drop_left(state: &GameState, player: PlayerId) -> bool {
    state.players[player.get() as usize].lands_played_this_turn < land_drops_allowed(state, player)
}

/// Whether a land sitting in `zone` is one `player` may play (CR 305.1, and
/// the permissions that widen it).
///
/// The hand is the rules' own answer and needs no effect. The graveyard is
/// Crucible of Worlds and Ramunap Excavator, and it is a **permission**
/// rather than a second way of casting: `Modifier::GrantsFlashback` is the
/// neighbouring sentence about a graveyard and says nothing at all here,
/// because playing a land is not casting a spell (CR 305.1).
#[must_use]
pub fn land_zone_open(state: &GameState, player: PlayerId, zone: Zone) -> bool {
    match zone {
        Zone::Hand => true,
        Zone::Graveyard => state.effects.iter().any(|fx| {
            fx.controller == player
                && matches!(
                    fx.modifier,
                    baylee_cards_dsl::Modifier::PlayLandsFromGraveyard
                )
        }),
        _ => false,
    }
}

/// Plays a land (special action, no stack).
///
/// # Errors
/// [`CastFailure`] when the action is illegal.
///
/// # Panics
/// Internal invariant violations (existence validated first).
pub fn play_land(
    state: &mut GameState,
    player: PlayerId,
    card: ObjectId,
) -> Result<(), CastFailure> {
    let obj = state.object(card).ok_or(CastFailure::NoSuchObject)?;
    if !land_zone_open(state, player, obj.zone) || obj.zone_owner != Some(player) {
        return Err(CastFailure::Legality(CastError::NotInHand));
    }
    if !obj.characteristics().types.contains(TypeSet::LAND) {
        return Err(CastFailure::Legality(CastError::BadTiming));
    }
    let main_phase = matches!(state.turn.phase, Phase::FirstMain | Phase::SecondMain);
    if !main_phase || state.turn.active != player || !state.zones.stack_is_empty() {
        return Err(CastFailure::Legality(CastError::BadTiming));
    }
    if !has_a_land_drop_left(state, player) {
        return Err(CastFailure::Legality(CastError::BadTiming));
    }
    state.players[player.get() as usize].lands_played_this_turn += 1;
    {
        let obj = state.object_mut(card).expect("validated");
        obj.kind = ObjectKind::Permanent;
        obj.set_controller(player);
    }
    state
        .move_object(
            card,
            ZoneLocation::Battlefield,
            ZonePosition::Top,
            Cause::Effect,
        )
        .map_err(CastFailure::State)?;
    state.journal.record(GameEvent::LandPlayed {
        object: card,
        player,
    });
    Ok(())
}

/// Mana color produced by a land with exactly one basic land subtype
/// (CR 305.6).
///
/// `None` for a land with no basic subtype and for one with several: both
/// are served by the `AddManaChoice` ability printed on the card, which can
/// offer the choice this shortcut cannot.
#[must_use]
pub fn intrinsic_mana(state: &GameState, source: ObjectId) -> Option<ManaColor> {
    let obj = state.object(source)?;
    if !obj.characteristics().types.contains(TypeSet::LAND) {
        return None;
    }
    let s = &obj.characteristics().subtypes;
    let mut only = None;
    for (subtype, color) in [
        (land::PLAINS, ManaColor::White),
        (land::ISLAND, ManaColor::Blue),
        (land::SWAMP, ManaColor::Black),
        (land::MOUNTAIN, ManaColor::Red),
        (land::FOREST, ManaColor::Green),
    ] {
        if s.contains(subtype) {
            // CR 305.6 gives a land one mana ability *per* basic type, so a
            // dual has two of them and its controller picks which to
            // activate. This shortcut cannot ask, and answering on the
            // player's behalf is worse than not answering at all: Godless
            // Shrine used to tap for white and never for black, whatever the
            // player needed. A land with more than one basic type is left to
            // the printed `AddManaChoice` ability on its card, which does ask.
            if only.is_some() {
                return None;
            }
            only = Some(color);
        }
    }
    only
}

/// CR 903.8's tax on casting `card` from the command zone, in generic mana:
/// `{2}` for each previous cast of *this* commander from there.
///
/// Zero unless the card is in the command zone right now — a commander cast
/// from a hand it was bounced to pays nothing. And per commander rather than
/// per seat: a partner deck taxes its two independently, which is why the
/// count sits on [`crate::state::Commander`] and not beside
/// `GameState::commander_casts`, whose job is Commander's Insight.
///
/// It lives beside [`controls_a_commander`] for the same reason that one
/// does, and it moved here from the wizard after making the same mistake in
/// the other direction: the wizard charged the tax and the legality probe
/// did not, so a commander that had been cast once was *offered* for its
/// printed cost and then refused mid-wizard. A tax the counter records and
/// nothing charges is not a tax, and one only half the engine charges is
/// worse than none.
#[must_use]
pub fn commander_tax(state: &GameState, player: PlayerId, card: ObjectId) -> u32 {
    if state.object(card).map(|o| o.zone) != Some(Zone::Command) {
        return 0;
    }
    state
        .commanders
        .get(player.get() as usize)
        .and_then(|cs| cs.iter().find(|c| c.object == card))
        .map_or(0, |c| c.casts.saturating_mul(2))
}

/// Does `player` control one of their own commanders right now?
///
/// This is card text, not a rule — "if you control a commander" is what
/// Fierce Guardianship and Flawless Maneuver print — but it lives here
/// because two readers ask it: the legality probe below and the wizard's
/// list of cast options. A probe that answered differently from the wizard
/// offers a spell the wizard then refuses to cast, which is exactly the bug
/// this replaces. It reads the marker list rather than the command zone,
/// since a commander on the battlefield has left that zone by definition.
#[must_use]
pub fn controls_a_commander(state: &GameState, player: PlayerId) -> bool {
    state
        .commanders
        .get(player.get() as usize)
        .is_some_and(|cs| {
            cs.iter().any(|c| {
                state
                    .object(c.object)
                    .is_some_and(|o| o.zone == Zone::Battlefield && o.controller == player)
            })
        })
}

/// The cards that could be exiled from `player`'s hand to pay an
/// `ExileFromHand` cost on `card` — Force of Will's blue card, Solitude's
/// white one.
///
/// One reader, three callers, and that is the whole point of it being a
/// function. `cast_wizard`'s `PitchChoice` stage builds its prompt from this
/// list; [`can_cast`] asks whether the list is empty before calling the card
/// castable; `Engine::can_afford` asks the same before offering the
/// alternative cost as a mode. The two askers used to answer "yes" without
/// looking, so a Force of Will with no other blue card was lit up as castable
/// and then reversed itself on reaching the pitch stage — the shape a dead
/// offer always has, and the one a second predicate written to *agree* with
/// the stage would have kept, because it would have agreed with itself.
///
/// The card being cast is not a candidate: it is what is being paid for, and
/// it is what `eval::matches` is handed as `this`, so a filter saying
/// "another" reads the same word here as anywhere else.
#[must_use]
pub fn pitchable(
    state: &GameState,
    player: PlayerId,
    card: ObjectId,
    filter: &baylee_cards_dsl::Filter,
) -> Vec<ObjectId> {
    state
        .zones
        .list(ZoneLocation::Hand(player))
        .iter()
        .copied()
        .filter(|id| {
            *id != card
                && state
                    .object(*id)
                    .is_some_and(|o| crate::eval::matches(filter, state, o, player, card))
        })
        .collect()
}

/// Whether the non-mana parts of an alternative cost could be paid right now.
///
/// [`can_cast`] reaches this only when the printed cost is unaffordable and
/// the face prints an alternative cost, which in this pool is four cards, so
/// the hand scan costs nothing anybody can measure. `zones.list` is ordered,
/// so it costs no determinism either.
///
/// The nine parts that answer `true` are named rather than swept into a
/// wildcard: `cast_wizard::paid_as_an_alternative_cost` says which two of
/// them the payment ever pays, and `offer_tests` holds the rest off this list
/// entirely — so a twelfth `CostPart` has to be looked at here too rather
/// than being quietly declared payable.
fn alternative_parts_payable(
    state: &GameState,
    player: PlayerId,
    card: ObjectId,
    parts: &[baylee_cards_dsl::CostPart],
) -> bool {
    use baylee_cards_dsl::CostPart;
    parts.iter().all(|part| match part {
        CostPart::PayLife(n) => state.can_pay_life(player, i32::from(*n)),
        CostPart::ExileFromHand(filter) => !pitchable(state, player, card, filter).is_empty(),
        // The same question `can_afford` asks of an activation, asked of the
        // card about to be cast. It answers `false` for every card in hand,
        // because a card in a hand carries no counters — and that is the
        // truthful answer rather than a special case: an alternative cost
        // this pool does not print is not silently declared payable.
        CostPart::RemoveCounterSelf { kind, n } => state
            .object(card)
            .is_some_and(|o| o.counters.get(*kind) >= *n),
        // The rest are payable, most of them because they are paid off the
        // card or the board rather than out of a count. `RemoveCounterSelfX`
        // joins them for its own reason: zero is a legal number to announce,
        // so there is nothing here an alternative cost could fail to pay.
        CostPart::TapSelf
        | CostPart::UntapSelf
        | CostPart::SacrificeSelf
        | CostPart::Sacrifice(_)
        | CostPart::Discard(_)
        | CostPart::TapOther(_)
        | CostPart::ReturnToHand(_)
        | CostPart::DiscardSelf
        | CostPart::ExileSelf
        | CostPart::ReturnSelfToHand
        | CostPart::RemoveCounterSelfX { .. }
        | CostPart::PutCounterSelf { .. }
        | CostPart::PayLifeX => true,
    })
}

/// Whether the intrinsic mana ability of `source` can be activated now.
#[must_use]
pub fn can_activate_mana(state: &GameState, player: PlayerId, source: ObjectId) -> bool {
    let Some(obj) = state.object(source) else {
        return false;
    };
    obj.zone == Zone::Battlefield
        && obj.controller == player
        && !obj.status.contains(crate::object::Status::TAPPED)
        // A land is never summoning sick, so this costs an ordinary land
        // nothing; it is here for the land that is also a creature (Dryad
        // Arbor, an animated manland), whose intrinsic {T} is an activated
        // ability of a creature like any other (CR 302.6).
        && !crate::combat::summoning_sick(state, obj)
        && intrinsic_mana(state, source).is_some()
}

/// Taps a basic land for its intrinsic mana (CR 305.6).
///
/// # Errors
/// [`CastFailure::NoSuchObject`] for stale handles.
///
/// # Panics
/// Internal invariant violations (legality checked first).
pub fn activate_mana(
    state: &mut GameState,
    player: PlayerId,
    source: ObjectId,
) -> Result<(), CastFailure> {
    if !can_activate_mana(state, player, source) {
        return Err(CastFailure::Legality(CastError::BadTiming));
    }
    let color = intrinsic_mana(state, source).expect("checked above");
    {
        let obj = state.object_mut(source).expect("checked above");
        obj.status.insert(crate::object::Status::TAPPED);
    }
    state.journal.record(GameEvent::ObjectTapped {
        object: source,
        cause: Cause::Cost,
    });
    let snow = state.object(source).is_some_and(|o| {
        o.characteristics()
            .supertypes
            .contains(baylee_core::types::SupertypeSet::SNOW)
    });
    if snow {
        state.players[player.get() as usize]
            .mana_pool
            .add_snow(color, 1);
    } else {
        state.players[player.get() as usize].mana_pool.add(color, 1);
    }
    state.journal.record(GameEvent::ManaProduced {
        player,
        color,
        amount: 1,
        source: Some(source),
    });
    Ok(())
}

/// Casting/playing failure.
#[derive(Debug, thiserror::Error)]
pub enum CastFailure {
    /// Legality violation.
    #[error("illegal action: {0}")]
    Legality(#[from] CastError),
    /// Stale handle.
    #[error("object vanished")]
    NoSuchObject,
    /// Zone machinery error.
    #[error("state error: {0}")]
    State(#[from] StateError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::{ContinuousEffect, EffectFilter};
    use crate::state::{CardLookup, Commander};
    use baylee_cards_dsl::{Duration, Filter, Modifier};
    use baylee_core::ids::{CardIndex, EffectId, ObjectId, SubtypeId};
    use baylee_core::preset::{
        AIProfile, DeckEntry, FormatId, GamePreset, HouseRules, PrintInfo, SeatCapabilities,
        SeatController, SeatSpec,
    };

    struct RegistryLookup;
    impl CardLookup for RegistryLookup {
        fn card(&self, index: CardIndex) -> Option<&'static baylee_cards_dsl::CardDef> {
            baylee_cards::by_index(index)
        }
    }

    fn me() -> PlayerId {
        PlayerId::new(0)
    }
    fn them() -> PlayerId {
        PlayerId::new(1)
    }

    fn state() -> GameState {
        let forest = baylee_cards::by_oracle_id("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
            .expect("registry contains Forest")
            .index;
        let entry = DeckEntry {
            card: forest,
            print: baylee_core::ids::PrintRef::new(0),
        };
        let seat = || SeatSpec {
            controller: SeatController::Ai(AIProfile::default()),
            capabilities: SeatCapabilities::default(),
            deck: (0..60).map(|_| entry).collect(),
            sideboard: vec![],
            commanders: vec![],
            starting_life: None,
            starting_hand: None,
            starting_battlefield: vec![],
            emblems: vec![],
            team: None,
        };
        let preset = GamePreset {
            format: FormatId::Freeform,
            seed: 13,
            house_rules: HouseRules::default(),
            modifiers: vec![],
            prints: vec![PrintInfo {
                scryfall_id: uuid::Uuid::nil(),
                lang: "EN".into(),
                finish: baylee_core::preset::Finish::Normal,
            }],
            seats: vec![seat(), seat()],
        };
        GameState::from_preset(&preset, &RegistryLookup).expect("game starts")
    }

    fn permanent(state: &mut GameState, owner: PlayerId, name: &str) -> ObjectId {
        let name = state.names.intern(name);
        state.create_bare(
            owner,
            ObjectKind::Permanent,
            name,
            ZoneLocation::Battlefield,
        )
    }

    /// A land with the given basic types and nothing else.
    fn basic_land(state: &mut GameState, name: &str, types: &[SubtypeId]) -> ObjectId {
        let id = permanent(state, me(), name);
        let base = state.object_mut(id).expect("just made it").base_mut();
        base.types = TypeSet::LAND;
        for t in types {
            base.subtypes.insert(*t);
        }
        id
    }

    fn register(state: &mut GameState, controller: PlayerId, modifier: Modifier) {
        state.effects.register(ContinuousEffect {
            id: EffectId::new(0),
            source: None,
            controller,
            layer: modifier.layer(),
            timestamp: 1,
            duration: Duration::Indefinitely,
            filter: EffectFilter::Dsl(&Filter::This),
            modifier,
        });
    }

    /// Puts the state in `player`'s first main phase with an empty stack —
    /// the one moment a sorcery may be cast, so every refusal below is one
    /// thing changed from here.
    fn main_phase_of(state: &mut GameState, player: PlayerId) {
        state.turn.phase = Phase::FirstMain;
        state.turn.step = crate::turn::Step::Main;
        state.turn.active = player;
    }

    fn no_keywords() -> baylee_cards_dsl::KeywordSet {
        baylee_cards_dsl::KeywordSet::EMPTY
    }

    /// CR 117.1a: an instant whenever its controller has priority, and a
    /// noninstant only in its controller's own main phase with the stack
    /// empty. Three things make a sorcery illegal and each of them alone is
    /// enough, which is why they are asked one at a time — and the second
    /// main phase is the fourth row, because the permission is read off the
    /// *phase* and `Step::Main` cannot tell the two apart.
    #[test]
    fn a_sorcery_needs_all_three_of_the_permissions_an_instant_needs_none_of() {
        let mut state = state();
        main_phase_of(&mut state, me());
        let sorcery =
            |state: &GameState| timing_allows(state, me(), TypeSet::SORCERY, no_keywords());
        let instant =
            |state: &GameState| timing_allows(state, me(), TypeSet::INSTANT, no_keywords());

        assert!(sorcery(&state), "your own main phase, stack empty");
        assert!(instant(&state));

        state.turn.phase = Phase::SecondMain;
        assert!(sorcery(&state), "and the other main phase is one too");

        state.turn.phase = Phase::Beginning;
        assert!(!sorcery(&state), "not a main phase");
        assert!(instant(&state), "an instant does not care which phase");

        main_phase_of(&mut state, them());
        assert!(!sorcery(&state), "somebody else's main phase");
        assert!(instant(&state));

        main_phase_of(&mut state, me());
        let name = state.names.intern("Something");
        state.create_bare(me(), ObjectKind::Spell, name, ZoneLocation::Stack);
        assert!(!sorcery(&state), "the stack is not empty");
        assert!(instant(&state), "which is exactly when an instant is for");
    }

    /// Flash (CR 702.8a) puts a card in the instant group whatever its types
    /// say, and Teferi's `+1` does the same for that player's **sorceries**
    /// only. Two halves a "your spells have flash" reading would get wrong:
    /// it is not granted to a creature, and it is not granted to the player
    /// across the table.
    #[test]
    fn flash_and_a_granted_flash_are_read_from_two_different_places() {
        let mut state = state();
        main_phase_of(&mut state, them());
        let flash = baylee_cards_dsl::KeywordSet::FLASH;

        assert!(
            !timing_allows(&state, me(), TypeSet::CREATURE, no_keywords()),
            "a creature on somebody else's turn"
        );
        assert!(
            timing_allows(&state, me(), TypeSet::CREATURE, flash),
            "and the same creature with flash"
        );

        assert!(!timing_allows(
            &state,
            me(),
            TypeSet::SORCERY,
            no_keywords()
        ));
        register(&mut state, them(), Modifier::SorceriesHaveFlash);
        assert!(
            !timing_allows(&state, me(), TypeSet::SORCERY, no_keywords()),
            "the +1 an opponent activated is not mine"
        );

        register(&mut state, me(), Modifier::SorceriesHaveFlash);
        assert!(
            timing_allows(&state, me(), TypeSet::SORCERY, no_keywords()),
            "and the one I activated is"
        );
        assert!(
            !timing_allows(&state, me(), TypeSet::CREATURE, no_keywords()),
            "it says sorceries, and a creature is not one"
        );
    }

    /// Teferi's static is asked **first** and beats flash: an opponent's
    /// instant is pulled back to sorcery speed whatever it says. What it
    /// leaves them is exactly what sorcery speed is — their own main phase
    /// with an empty stack — and its own controller is not their own
    /// opponent, which is the row that says the question is asked per seat.
    #[test]
    fn a_sorcery_speed_lock_beats_flash_and_spares_its_controller() {
        let mut state = state();
        main_phase_of(&mut state, them());
        let flash = baylee_cards_dsl::KeywordSet::FLASH;

        assert!(
            timing_allows(&state, me(), TypeSet::INSTANT, no_keywords()),
            "before the lock, an instant on anybody's turn"
        );
        register(&mut state, them(), Modifier::OpponentsCastAsSorcery);
        assert!(
            !timing_allows(&state, me(), TypeSet::INSTANT, no_keywords()),
            "and under it, not on theirs"
        );
        assert!(
            !timing_allows(&state, me(), TypeSet::CREATURE, flash),
            "flash does not get out from under it"
        );

        main_phase_of(&mut state, me());
        assert!(
            timing_allows(&state, them(), TypeSet::INSTANT, no_keywords()),
            "the seat that controls the lock is not locked by it — and it is \
             not their turn, so nothing else explains this"
        );
        assert!(
            timing_allows(&state, me(), TypeSet::INSTANT, no_keywords()),
            "the lock leaves an opponent their own main phase"
        );
        assert!(timing_allows(&state, me(), TypeSet::SORCERY, no_keywords()));
    }

    /// A face with no text, carrying only the three fields these probes read.
    fn probe_face(
        convoke: bool,
        delve: bool,
        reduction: Option<baylee_cards_dsl::CostReduction>,
    ) -> baylee_cards_dsl::FaceDef {
        let mut face = crate::engine::synthetic::land_face("Probe");
        face.convoke = convoke;
        face.delve = delve;
        face.cost_reduction = reduction;
        face
    }

    fn artifact(state: &mut GameState, owner: PlayerId, name: &str) -> ObjectId {
        let id = permanent(state, owner, name);
        state.object_mut(id).expect("just made it").base_mut().types = TypeSet::ARTIFACT;
        id
    }

    fn creature(state: &mut GameState, owner: PlayerId, name: &str) -> ObjectId {
        let id = permanent(state, owner, name);
        state.object_mut(id).expect("just made it").base_mut().types = TypeSet::CREATURE;
        id
    }

    fn tap(state: &mut GameState, id: ObjectId) {
        state
            .object_mut(id)
            .expect("on the battlefield")
            .status
            .insert(crate::object::Status::TAPPED);
    }

    /// Convoke pays with untapped creatures and artifacts its caster
    /// controls (CR 702.51), and each of those four words is a row: a tapped
    /// one is not a source, an opponent's is not a source, and a land is
    /// not one however untapped it is.
    ///
    /// The count is what the offer and the payment both read. They disagreed
    /// once and the result was a convoke spell offered exactly when its
    /// printed cost was already payable — which is the one case convoke is
    /// not for.
    #[test]
    fn convoke_counts_untapped_permanents_of_two_types_on_one_side() {
        let mut state = state();
        let bear = creature(&mut state, me(), "Bear");
        let mox = artifact(&mut state, me(), "Mox");
        let tapped = creature(&mut state, me(), "Tapped");
        tap(&mut state, tapped);
        let theirs = creature(&mut state, them(), "Theirs");
        tap(&mut state, theirs);
        let untapped_theirs = creature(&mut state, them(), "Also theirs");
        let land = permanent(&mut state, me(), "Land");
        state.object_mut(land).expect("made it").base_mut().types = TypeSet::LAND;

        let sources = convoke_sources(&state, me());
        assert_eq!(
            sources,
            vec![bear, mox],
            "a tapped one, an opponent's, and a land are none of them"
        );
        assert_eq!(
            convoke_sources(&state, them()),
            vec![untapped_theirs],
            "and the other side counts its own"
        );

        // What the keyword is then worth, which is the number the two probes
        // must agree on — and nothing at all on a face that does not print it.
        assert_eq!(
            keyword_reduction(&state, &probe_face(true, false, None), me()),
            2
        );
        assert_eq!(
            keyword_reduction(&state, &probe_face(false, false, None), me()),
            0
        );
    }

    /// Delve pays with the caster's graveyard (CR 702.66a), one card each,
    /// and it stacks with convoke on a face that printed both. Dig Through
    /// Time is the pool's only delve card and was offered at eight mana or
    /// not at all, with a full graveyard doing nothing.
    #[test]
    fn delve_counts_a_graveyard_and_adds_to_whatever_else_the_face_prints() {
        let mut state = state();
        for i in 0..3 {
            let name = state.names.intern(&format!("Buried {i}"));
            state.create_bare(me(), ObjectKind::Card, name, ZoneLocation::Graveyard(me()));
        }
        let name = state.names.intern("Theirs");
        state.create_bare(
            them(),
            ObjectKind::Card,
            name,
            ZoneLocation::Graveyard(them()),
        );
        creature(&mut state, me(), "Bear");

        assert_eq!(
            keyword_reduction(&state, &probe_face(false, true, None), me()),
            3,
            "my graveyard, and not the table's"
        );
        assert_eq!(
            keyword_reduction(&state, &probe_face(false, true, None), them()),
            1
        );
        assert_eq!(
            keyword_reduction(&state, &probe_face(true, true, None), me()),
            4,
            "a face printing both adds them"
        );
    }

    /// A printed reduction is read off the card and asked of the seat:
    /// Surgical Metamorph costs `{1}` less if you were not the starting
    /// player, so the seat it is *for* is the one the offer used to leave it
    /// out for.
    #[test]
    fn a_printed_reduction_reaches_the_seat_it_was_printed_for() {
        let state = state();
        let face = probe_face(
            false,
            false,
            Some(baylee_cards_dsl::CostReduction::NotStartingPlayer(1)),
        );
        let starter = state.starting_player;
        let other = if starter == me() { them() } else { me() };

        assert_eq!(printed_reduction(&state, &face, starter), 0);
        assert_eq!(printed_reduction(&state, &face, other), 1);
        assert_eq!(
            printed_reduction(&state, &probe_face(false, false, None), other),
            0,
            "and a card that prints no reduction gets none"
        );
    }

    /// Mycosynth Lattice's third line, which the affordability checks did not
    /// read at all — so the spell it made payable was never offered as
    /// castable in the first place.
    #[test]
    fn mana_is_wild_only_while_something_says_so() {
        let mut state = state();
        assert!(!mana_is_wild(&state));
        register(&mut state, me(), Modifier::ManaIsAnyColor);
        assert!(
            mana_is_wild(&state),
            "it is a question about the table and not about a seat"
        );
    }

    /// CR 305.6 gives a land one mana ability **per** basic type, so this
    /// shortcut answers only where there is nothing to choose. Answering on
    /// the player's behalf is worse than not answering: Godless Shrine used
    /// to tap for white and never for black, whatever the player needed.
    #[test]
    fn a_land_with_two_basic_types_is_not_answered_for_the_player() {
        let mut state = state();
        let plains = basic_land(&mut state, "Plains", &[land::PLAINS]);
        let shrine = basic_land(&mut state, "Godless Shrine", &[land::PLAINS, land::SWAMP]);
        let waste = basic_land(&mut state, "Wastes", &[]);
        let bear = permanent(&mut state, me(), "Bear");

        assert_eq!(intrinsic_mana(&state, plains), Some(ManaColor::White));
        assert_eq!(
            intrinsic_mana(&state, shrine),
            None,
            "the dual is left to the printed ability that asks"
        );
        assert_eq!(intrinsic_mana(&state, waste), None, "no basic type");
        assert_eq!(intrinsic_mana(&state, bear), None, "not a land at all");
    }

    /// CR 903.8: `{2}` for each previous cast of *this* commander from the
    /// command zone, and nothing at all while the card is somewhere else —
    /// a commander cast from a hand it was bounced to pays no tax.
    #[test]
    fn the_commander_tax_is_per_commander_and_only_in_the_command_zone() {
        let mut state = state();
        let name = state.names.intern("General");
        let general = state.create_bare(
            me(),
            ObjectKind::Permanent,
            name,
            ZoneLocation::Command(me()),
        );
        let name = state.names.intern("Partner");
        let partner = state.create_bare(
            me(),
            ObjectKind::Permanent,
            name,
            ZoneLocation::Command(me()),
        );
        state.commanders[me().get() as usize].push(Commander {
            object: general,
            casts: 2,
            answered: 0,
        });
        state.commanders[me().get() as usize].push(Commander {
            object: partner,
            casts: 0,
            answered: 0,
        });

        assert_eq!(commander_tax(&state, me(), general), 4);
        assert_eq!(
            commander_tax(&state, me(), partner),
            0,
            "a partner deck taxes its two independently"
        );
        assert_eq!(
            commander_tax(&state, them(), general),
            0,
            "and it is the caster's own list that is read"
        );

        state
            .move_object(
                general,
                ZoneLocation::Hand(me()),
                ZonePosition::Top,
                Cause::Effect,
            )
            .expect("it is bounced to hand");
        assert_eq!(
            commander_tax(&state, me(), general),
            0,
            "the tax is on casting it from the command zone"
        );
    }

    /// "If you control a commander" is card text rather than a rule, and it
    /// reads the marker list: a commander on the battlefield has left the
    /// command zone by definition, so asking the zone answers no for exactly
    /// the board the card is printed about.
    #[test]
    fn controlling_a_commander_is_asked_of_the_marker_list() {
        let mut state = state();
        let general = permanent(&mut state, me(), "General");

        assert!(!controls_a_commander(&state, me()), "no commander yet");

        state.commanders[me().get() as usize].push(Commander {
            object: general,
            casts: 0,
            answered: 0,
        });
        assert!(controls_a_commander(&state, me()));
        assert!(
            !controls_a_commander(&state, them()),
            "theirs, not the table's"
        );

        state
            .move_object(
                general,
                ZoneLocation::Graveyard(me()),
                ZonePosition::Top,
                Cause::Effect,
            )
            .expect("it dies");
        assert!(
            !controls_a_commander(&state, me()),
            "a commander in the graveyard is not one you control"
        );
    }

    /// One land a turn (CR 305.2a), and the extra drops are a fold rather
    /// than a flag. The predicate beside it is the one both ends ask — the
    /// offer that puts a land in `legal.lands` and the play that refuses an
    /// answer nobody offered — so that a limit which stops being one cannot
    /// be read two ways.
    #[test]
    fn a_land_drop_is_counted_in_one_place_for_both_ends() {
        let mut state = state();
        assert_eq!(land_drops_allowed(&state, me()), 1);
        assert!(has_a_land_drop_left(&state, me()));

        state.players[me().get() as usize].lands_played_this_turn = 1;
        assert!(!has_a_land_drop_left(&state, me()));

        register(&mut state, me(), Modifier::ExtraLandDrops(1));
        assert_eq!(land_drops_allowed(&state, me()), 2);
        assert!(has_a_land_drop_left(&state, me()), "Exploration");
        assert_eq!(
            land_drops_allowed(&state, them()),
            1,
            "an extra drop is its controller's"
        );

        register(&mut state, me(), Modifier::ExtraLandDrops(2));
        assert_eq!(land_drops_allowed(&state, me()), 4, "they add up");
    }

    /// CR 305.1: the hand needs no permission, and the graveyard is one —
    /// Crucible of Worlds. `GrantsFlashback` is the neighbouring sentence
    /// about a graveyard and says nothing here, because playing a land is
    /// not casting a spell.
    #[test]
    fn a_land_is_played_from_the_hand_and_from_a_graveyard_only_by_permission() {
        let mut state = state();
        assert!(land_zone_open(&state, me(), Zone::Hand));
        assert!(!land_zone_open(&state, me(), Zone::Graveyard));
        assert!(!land_zone_open(&state, me(), Zone::Exile));

        register(&mut state, me(), Modifier::GrantsFlashback);
        assert!(
            !land_zone_open(&state, me(), Zone::Graveyard),
            "flashback is about casting a spell"
        );

        register(&mut state, me(), Modifier::PlayLandsFromGraveyard);
        assert!(land_zone_open(&state, me(), Zone::Graveyard));
        assert!(
            !land_zone_open(&state, them(), Zone::Graveyard),
            "the permission is its controller's"
        );
    }
}
