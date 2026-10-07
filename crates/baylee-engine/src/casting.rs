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
use baylee_cards_dsl::SpendRider;
use baylee_core::generated::subtypes::land;
use baylee_core::ids::{ObjectId, PlayerId};
use baylee_core::mana::{ManaColor, ManaCost, ManaFlags, ManaPool, ManaSpending, RestrictedMana};
use baylee_core::types::TypeSet;
use smallvec::SmallVec;

mod intrinsic;
mod lands;
mod payment;
#[cfg(test)]
mod tests;

pub use intrinsic::*;
pub use lands::*;
pub(crate) use payment::*;

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
    /// A continuous effect forbids this cast outright (CR 601.3a): Silence,
    /// Ranger-Captain of Eos.
    ///
    /// Its own variant rather than [`Self::BadTiming`], which is what a
    /// sorcery-speed lock answers: a player told "sorcery-speed timing not
    /// met" on their own main phase with an empty stack goes looking for a
    /// rule that is not there.
    #[error("an effect forbids casting this spell")]
    Forbidden,
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

/// The live permissions for `player` to spend actual mana as another type.
/// Static-effect synchronization supplies the source's current controller
/// and removes effects whose source left, phased out or lost its ability.
#[must_use]
pub fn mana_spending(state: &GameState, player: PlayerId) -> ManaSpending {
    let mut spending = if mana_is_wild(state) {
        ManaSpending::ANY_COLOR
    } else {
        ManaSpending::EXACT
    };
    for fx in state.effects.iter().filter(|fx| fx.controller == player) {
        if let baylee_cards_dsl::Modifier::SpendManaAs { from, to } = fx.modifier {
            spending.allow(from, to);
        }
    }
    spending
}

/// The permanents convoke may be paid with (CR 702.51a): untapped creatures
/// the caster controls, one `{1}` each.
///
/// One function because the offer and the payment must not disagree. The
/// wizard enumerated these to ask which to tap, and `can_cast` did not
/// count them at all — so a convoke spell was offered as castable exactly
/// when its printed cost was already payable, which is the one case convoke
/// is not for. Clever Concealment and Spirit Water Revival were
/// `Coverage::Implemented` and could never actually be convoked.
///
/// Creatures only: artifacts are what [`waterbend_sources`] adds, and the two
/// were one walk until #229, so a Darksteel Pendant convoked. Over
/// [`GameState::battlefield_seen`], because a phased-out creature is treated
/// as though it does not exist (CR 702.26b) and cannot be tapped for anything.
#[must_use]
pub fn convoke_sources(state: &GameState, player: PlayerId) -> Vec<ObjectId> {
    untapped_of(state, player, TypeSet::CREATURE)
}

/// The permanents a waterbend cost may be paid with (CR 701.67a): untapped
/// artifacts and creatures the caster controls, one `{1}` each — of the
/// waterbend cost alone (CR 701.67b), which is the wizard's bound to set.
#[must_use]
pub fn waterbend_sources(state: &GameState, player: PlayerId) -> Vec<ObjectId> {
    untapped_of(state, player, TypeSet::CREATURE.union(TypeSet::ARTIFACT))
}

fn untapped_of(state: &GameState, player: PlayerId, types: TypeSet) -> Vec<ObjectId> {
    state
        .battlefield_seen()
        .filter(|id| {
            state.object(*id).is_some_and(|o| {
                o.controller == player
                    && o.characteristics().types.intersects(types)
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
///
/// Waterbend counts for nothing here, and that is its rule rather than an
/// omission: its taps pay only the waterbend cost itself (CR 701.67b), which
/// no probe of the printed cost includes, and paying it adds at least as much
/// as they take off. Counting them was #229: Spirit Water Revival was offered
/// off `{U}{U}` and one creature.
///
/// `card` is the card being cast, which delve never counts
/// ([`delve_sources`]).
#[must_use]
pub fn keyword_reduction(
    state: &GameState,
    face: &baylee_cards_dsl::FaceDef,
    player: PlayerId,
    card: ObjectId,
) -> u32 {
    let convoke = if face.convoke {
        u32::try_from(convoke_sources(state, player).len()).unwrap_or(u32::MAX)
    } else {
        0
    };
    let delve = if face.delve {
        u32::try_from(delve_sources(state, player, card).len()).unwrap_or(u32::MAX)
    } else {
        0
    };
    convoke.saturating_add(delve)
}

/// The cards delve (CR 702.66a) may exile to pay for `card`: its caster's
/// graveyard, less the card itself.
///
/// CR 601.2a puts a spell on the stack before any of its costs is paid
/// (601.2h), so a spell cast from a graveyard (a flashback grant) is not
/// there to exile for itself. This engine moves the card at the end of the
/// payment instead, and the delve question offered the whole graveyard: Dig
/// Through Time flashed back by Snapcaster Mage paid {1} of its own cost with
/// itself. One list for the offer's count and the question's options.
#[must_use]
pub fn delve_sources(state: &GameState, player: PlayerId, card: ObjectId) -> Vec<ObjectId> {
    state
        .zones
        .list(ZoneLocation::Graveyard(player))
        .iter()
        .copied()
        .filter(|id| *id != card)
        .collect()
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
    source: ObjectId,
) -> u32 {
    reduction_amount(state, face.cost_reduction, player, source)
}

/// The generic mana `reduction` takes off a cost `player` is paying with
/// `source` — a card being cast or the object whose ability is activated —
/// read now, as the total cost is determined (CR 601.2f, and CR 602.2b for
/// an ability). The taking-off is `ManaCost::with_less_generic`, which
/// touches only the generic component and stops at {0} (CR 118.7a).
#[must_use]
pub fn reduction_amount(
    state: &GameState,
    reduction: Option<baylee_cards_dsl::CostReduction>,
    player: PlayerId,
    source: ObjectId,
) -> u32 {
    match reduction {
        Some(baylee_cards_dsl::CostReduction::NotStartingPlayer(n))
            if player != state.starting_player =>
        {
            n
        }
        Some(baylee_cards_dsl::CostReduction::PerCount { amount, each }) => {
            crate::eval::amount(&amount, state, player, source, None).saturating_mul(each)
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
    // when as many of its modes as it must choose can each be pointed at
    // something, through every instance of the word each prints — one for
    // "choose one", two for "choose two" — and which ones is
    // `cast_options`' question, not this one's.
    let mut modal = abilities.iter().filter_map(|a| match a {
        baylee_cards_dsl::AbilityDef::ModalSpell { modes, choose } => Some((modes, choose)),
        _ => None,
    });
    if let Some((modes, choose)) = modal.next() {
        let takeable = modes
            .iter()
            .filter(|mode| {
                requirement_is_reachable(mode.targets, state, player, card)
                    && requirement_is_reachable(mode.second_targets, state, player, card)
            })
            .count();
        return takeable >= usize::from(choose.min.max(1));
    }
    let spell = abilities.iter().find_map(|a| match a {
        baylee_cards_dsl::AbilityDef::Spell {
            targets,
            second_targets,
            ..
        } => Some((*targets, *second_targets)),
        _ => None,
    });
    let (req, second) = spell.unwrap_or_default();
    // Every instance of the word has to be satisfiable (CR 601.2c): Khalni
    // Ambush with no creature on the other side of the table is not a spell
    // that can be cast, however many of the caster's own it could name.
    (requirement_is_reachable(req, state, player, card)
        || def
            .faces
            .get(face)
            .and_then(|f| f.kicked_targets)
            .is_some_and(|req| requirement_is_reachable(Some(req), state, player, card)))
        && requirement_is_reachable(second, state, player, card)
}

/// Whether the ordinary spell target requirement can be satisfied.
pub(crate) fn ordinary_targets_reachable(
    state: &GameState,
    def: &baylee_cards_dsl::CardDef,
    player: PlayerId,
    card: ObjectId,
) -> bool {
    def.abilities.iter().all(|ability| match ability {
        baylee_cards_dsl::AbilityDef::Spell {
            targets,
            second_targets,
            ..
        } => {
            requirement_is_reachable(*targets, state, player, card)
                && requirement_is_reachable(*second_targets, state, player, card)
        }
        _ => true,
    })
}

/// Complete printed mana price of a kicked spell, before increases/reductions:
/// the mana cost plus every additional cost's mana (CR 601.2f), with one
/// generic symbol as `ManaCost::combine` makes it.
#[must_use]
pub fn kicked_mana_cost(face: &baylee_cards_dsl::FaceDef) -> ManaCost {
    face.additional_costs
        .iter()
        .fold(face.mana_cost, |cost, extra| cost.combine(&extra.mana))
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
    let reqs = def.abilities.iter().find_map(|a| match a {
        baylee_cards_dsl::AbilityDef::ModalSpell { modes, .. } => {
            modes.get(mode).map(|m| (m.targets, m.second_targets))
        }
        _ => None,
    });
    // Both instances of the word, as `face_has_a_legal_target` asks them
    // (CR 601.2c): Archdruid's Charm's second mode with no creature on the
    // other side of the table is not a mode that can be chosen (CR 700.2a).
    match reqs {
        Some((first, second)) => {
            requirement_is_reachable(first, state, player, card)
                && requirement_is_reachable(second, state, player, card)
        }
        None => true,
    }
}

/// Every set of modes a spell that chooses more than one may be cast with
/// (CR 700.2a), as a bitmask — bit `i` is mode `i` — in increasing order:
/// each holds as many modes as `choose` allows, none twice (CR 700.2d).
///
/// Eight modes at most, which is a byte. The pool's widest is four.
pub fn mode_sets(
    modes: &[baylee_cards_dsl::SpellMode],
    choose: baylee_cards_dsl::ModeCount,
) -> impl Iterator<Item = u8> {
    let every = 1_u16 << modes.len().min(8);
    (1..every).filter_map(move |set| {
        let set = u8::try_from(set).ok()?;
        let count = u8::try_from(set.count_ones()).ok()?;
        (choose.min <= count && count <= choose.max).then_some(set)
    })
}

/// The modes a set names, with their indices, in the order they are
/// printed — which is the order they are carried out in (CR 608.2c).
pub fn chosen_modes(
    modes: &'static [baylee_cards_dsl::SpellMode],
    set: u8,
) -> impl Iterator<Item = (usize, &'static baylee_cards_dsl::SpellMode)> {
    modes
        .iter()
        .enumerate()
        .filter(move |(i, _)| *i < 8 && set & (1 << i) != 0)
}

/// What a set of modes costs: `base`, the spell's own price, with every
/// chosen mode's cost added to it (CR 700.2h, 601.2f) — spree's "+ {1}".
///
/// With one generic symbol, as a printed cost has and as
/// [`ManaCost::combine`] keeps it: two "+ {1}" on {W} are {2}{W}, which is
/// what a row draws.
#[must_use]
pub fn mode_set_cost(
    base: ManaCost,
    modes: &'static [baylee_cards_dsl::SpellMode],
    set: u8,
) -> ManaCost {
    chosen_modes(modes, set).fold(base, |cost, (_, mode)| {
        mode.additional_cost
            .map_or(cost, |extra| cost.combine(&extra))
    })
}

/// Whether every mode in a set can be pointed at something: a mode that
/// would be illegal can't be chosen (CR 700.2a), in a set as alone.
#[must_use]
pub fn mode_set_has_legal_targets(
    state: &GameState,
    lookup: &impl crate::state::CardLookup,
    player: PlayerId,
    card: ObjectId,
    set: u8,
) -> bool {
    (0..8_usize)
        .filter(|i| set & (1 << i) != 0)
        .all(|i| mode_has_a_legal_target(state, lookup, player, card, i))
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
pub(crate) fn requirement_is_reachable(
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
            baylee_cards_dsl::TargetSpec::Player(_) | baylee_cards_dsl::TargetSpec::ThisObject
        )
    {
        return true;
    }
    // **Both** readers, because the question a target menu asks is one
    // question over two lists. `Pending::ChooseTargets` carries `options` and
    // `player_options` and a target is picked out of their union (CR 115.4),
    // so counting objects alone made an offer and a refusal disagree:
    // `TargetSpec::AnyTarget` used to be short-circuited to "reachable" by
    // name beside `AnyPlayer` and `AnyOpponent`, which hid it — and the day
    // it stopped being on that list, Lightning Bolt was uncastable at a board
    // with no creature on it, with the opponent sitting right there and on
    // the menu the wizard would have printed.
    //
    // Derived rather than listed, for the same reason: a new `TargetSpec` is
    // counted by whichever of the two readers knows about it, and a spec
    // neither knows is unreachable, which is the honest answer.
    let objects = match x_bounded(req.spec) {
        // "Target spell with mana value X" before any X is announced: a
        // target for some X (`eval::matches_for_some_x`). Enumerated through
        // the same reader with the filter widened, so what makes a spell or
        // permanent targetable at all is still asked the one way.
        Some((widened, filter)) => crate::eval::target_options(&widened, state, player, card)
            .into_iter()
            .filter(|id| {
                state.object(*id).is_some_and(|o| {
                    crate::eval::matches_for_some_x(filter, state, o, player, card)
                })
            })
            .count(),
        None => crate::eval::target_options(&req.spec, state, player, card).len(),
    };
    let players = crate::eval::target_player_options(state, &req.spec, player).len();
    objects + players >= req.min as usize
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

/// Whether `player` may cast back face `face` of `def` now, as far as timing
/// goes (CR 601.3).
///
/// A face that is cast in place of the front is judged by its own
/// characteristics (CR 601.3e): only the face that will be up on the stack
/// is evaluated for an MDFC (CR 712.11c), and only the alternative
/// characteristics for an Adventure (CR 715.3a). So an instant printed on
/// the back of a creature or an enchantment is cast whenever an instant
/// could be, and the front is not. Read off the printed face: nothing can
/// have granted a face that is not up anything, and the two player-scoped
/// effects `timing_allows` reads still apply.
pub(crate) fn face_timing_allows(
    state: &GameState,
    player: PlayerId,
    card: ObjectId,
    def: &baylee_cards_dsl::CardDef,
    face: usize,
) -> bool {
    def.faces
        .get(face)
        .is_some_and(|f| timing_allows(state, player, f.types, def.keywords_for_face(face)))
        && spell_condition_allows(state, player, card, def, face)
}

/// "Cast this spell only [when]" (CR 506.7; Berserk's "only before the
/// combat damage step"): whether the condition the face's spell prints, if
/// it prints one, holds now for `player` casting `card`. A restriction of
/// the card's own, asked beside the timing its type gives it (CR 601.3):
/// an instant restricted to combat is still cast whenever an instant could
/// be, inside that window.
pub(crate) fn spell_condition_allows(
    state: &GameState,
    player: PlayerId,
    card: ObjectId,
    def: &baylee_cards_dsl::CardDef,
    face: usize,
) -> bool {
    def.abilities_for_face(face).iter().all(|a| match a {
        baylee_cards_dsl::AbilityDef::Spell {
            condition: Some(condition),
            ..
        } => crate::eval::condition_holds(state, player, card, *condition),
        _ => true,
    })
}

/// Whether a continuous effect forbids `player` casting `obj` at all
/// (CR 601.3a).
///
/// `Modifier::OpponentsCantCast` is player-scoped like Teferi's lock above,
/// and unlike it names *which* spells: Silence forbids every one, and
/// Ranger-Captain of Eos only the noncreature ones. The filter is evaluated
/// from the **effect's** controller, because "noncreature spells" is that
/// player's sentence about somebody else's card — and `this` is the effect's
/// own source, so a filter naming `Filter::This` means the forbidding
/// permanent rather than the card being cast.
pub(crate) fn cast_is_forbidden(
    state: &GameState,
    player: PlayerId,
    obj: &crate::object::GameObject,
) -> bool {
    state.effects.iter().any(|fx| {
        let baylee_cards_dsl::Modifier::OpponentsCantCast(filter) = fx.modifier else {
            return false;
        };
        state.is_opponent(fx.controller, player)
            && crate::eval::matches(
                filter,
                state,
                obj,
                fx.controller,
                fx.source.unwrap_or(obj.id),
            )
    })
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
pub fn can_cast(
    state: &GameState,
    lookup: &impl crate::state::CardLookup,
    player: PlayerId,
    card: ObjectId,
) -> Result<(), CastError> {
    let normal = can_cast_form(state, lookup, player, card, None);
    if normal.is_ok() {
        return normal;
    }
    if let Some(face) = state
        .object(card)
        .and_then(|o| o.card)
        .and_then(|c| lookup.card(c.index))
        .and_then(|d| d.faces.first())
    {
        for form in [
            face.prototype.map(SpellForm::Prototype),
            face.disguise.map(|_| SpellForm::Disguise),
        ]
        .into_iter()
        .flatten()
        {
            if can_cast_form(state, lookup, player, card, Some(form)).is_ok() {
                return Ok(());
            }
        }
    }
    normal
}

/// Whether `player` may begin to cast a spell at all this turn (CR 601.3).
///
/// Conduit of Worlds' "If you do, you can't cast additional spells this
/// turn" is the one sentence that says no: it names the player and every
/// spell, so it is asked here, beside the table's other refusals, by every
/// door a cast comes through — the priority offer, a free cast an effect
/// makes (cascade, rebound, suspend), a miracle, a prepared copy.
#[must_use]
pub(crate) fn may_begin_casting(state: &GameState, player: PlayerId) -> bool {
    !state
        .per_turn
        .no_more_spells
        .get(player.get() as usize)
        .copied()
        .unwrap_or(false)
}

#[allow(clippy::too_many_lines)] // one gate per casting rule
pub(crate) fn can_cast_form(
    state: &GameState,
    lookup: &impl crate::state::CardLookup,
    player: PlayerId,
    card: ObjectId,
    form: Option<SpellForm>,
) -> Result<(), CastError> {
    let obj = state.object(card).ok_or(CastError::NotInHand)?;
    let in_hand = obj.zone == Zone::Hand && obj.zone_owner == Some(player);
    let in_own_graveyard = obj.zone == Zone::Graveyard && obj.zone_owner == Some(player);
    // Flashback (CR 702.34a): a card may be cast from its owner's graveyard
    // when a grant or its own printed flashback says so. The printed one
    // has its own price, probed below.
    let printed_flashback = obj
        .card
        .and_then(|c| lookup.card(c.index))
        .and_then(|def| def.faces[0].flashback);
    let flashback_ok = !in_hand
        && in_own_graveyard
        && (printed_flashback.is_some() || flashback_granted(state, card));
    // Escape (CR 702.138a): from its owner's graveyard, for its own cost.
    let printed_escape = obj
        .card
        .and_then(|c| lookup.card(c.index))
        .and_then(|def| def.faces[0].escape);
    let escape_ok = !in_hand && in_own_graveyard && printed_escape.is_some();
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
    // "You may play that card this turn" (Dauthi Voidwalker, Expressive
    // Iteration): a permission for this object, from whatever exile it lies
    // in — the owner's, which is not the caster's.
    let permission = if in_hand {
        None
    } else {
        play_permission(state, player, card)
    };
    // Wrenn's emblem and Muldrotha: a permanent card cast from its owner's
    // graveyard at its own price.
    let graveyard_ok = !in_hand && graveyard_cast_permission(state, player, obj).is_some();
    if !in_hand
        && !graveyard_ok
        && !flashback_ok
        && !escape_ok
        && !disturb_ok
        && !adventure_ok
        && !takeover_ok
        && !commander_ok
        && permission.is_none()
    {
        return Err(CastError::NotInHand);
    }
    let projected = form.map(|f| f.project(obj));
    let obj = projected.as_ref().unwrap_or(obj);
    let c = obj.characteristics();
    if c.types.contains(TypeSet::LAND) {
        return Err(CastError::BadTiming);
    }
    // A cast an effect forbids outright (CR 601.3a), asked before timing
    // because the two answers are different and only one of them is true:
    // Silence does not move a spell to sorcery speed, it removes the
    // permission, and a player reading "sorcery-speed timing not met" on
    // their own main phase would go looking for a rule that is not there.
    if cast_is_forbidden(state, player, obj) || !may_begin_casting(state, player) {
        return Err(CastError::Forbidden);
    }
    // Timing (CR 601.3). Read off the projected characteristics, so a
    // granted flash counts. That is the front face's timing, and when it
    // says no, a back face cast in its place may still say yes by its own
    // (`face_timing_allows`): Vantress Visions is an instant on the back of
    // an enchantment, and it is cast in answer to an ability on the stack
    // or not at all. A prototype or a disguise is the front, cast another
    // way.
    let printed = obj.card.and_then(|c| lookup.card(c.index));
    let front_now = timing_allows(state, player, c.types, c.keywords)
        && printed.is_none_or(|def| spell_condition_allows(state, player, card, def, 0));
    let a_back_face_now = printed.is_some_and(|def| {
        castable_back_faces(def, on_adventure)
            .any(|(i, _)| face_timing_allows(state, player, card, def, i))
    });
    if !front_now && (form.is_some() || !a_back_face_now) {
        return Err(CastError::BadTiming);
    }
    // "As an additional cost to cast this spell, sacrifice a creature": a
    // board with nothing to sacrifice cannot pay it, so the spell is not a
    // cast this player can make (CR 601.2h). The reader is the one the cast
    // wizard's `Sacrifice` stage builds its question from, so the offer and
    // the question cannot disagree — Force of Will's pitch is the precedent.
    if let Some(def) = obj.card.and_then(|card| lookup.card(card.index))
        && crate::engine::cast_wizard::additional_sacrifices(&def.faces[0])
            .any(|part| crate::engine::cost_wizard::options(state, player, card, part).is_empty())
    {
        return Err(CastError::NoWayToCast);
    }
    // Restricted mana this spell may be paid with counts towards it; see
    // [`spendable_pool`].
    let with_restricted = planning_pool(
        state,
        player,
        form.map_or(SpendFor::Spell(card), |f| SpendFor::SpellAs(card, f)),
        0,
    );
    let pool = with_restricted
        .as_ref()
        .unwrap_or(&state.players[player.get() as usize].mana_pool);
    // Commander tax (CR 903.8). A cost *increase*, so it lands on every way
    // of casting the card — printed cost, alternative cost and mode alike
    // (CR 601.2f) — which is why it is folded into each probe below rather
    // than into the first one.
    let tax = spell_increase(state, player, obj);
    if let Some(form) = form {
        return if affordable(state, player, pool, &form.cost().with_more_generic(tax)) {
            Ok(())
        } else {
            Err(CastError::NotEnoughMana)
        };
    }
    // Convoke and delve are *reductions* of the generic part, so they go on
    // the same probes the tax does and in the other direction. Read off the
    // printed face: a granted convoke does not exist.
    let printed_face = printed.map(|def| &def.faces[0]);
    let reduction = printed_face.map_or(0, |face| {
        keyword_reduction(state, face, player, card)
            .saturating_add(printed_reduction(state, face, player, card))
    });
    let probe = |cost: &ManaCost| {
        affordable(
            state,
            player,
            pool,
            &cost.with_more_generic(tax).with_less_generic(reduction),
        )
    };
    // A waived mana cost still pays increases (CR 601.2f).
    if permission.is_some_and(|p| p.free) {
        let payable = if let Some(def) = printed.filter(|def| modes_are_the_only_way(def, 0)) {
            def.abilities_for_face(0).iter().any(|a| match a {
                baylee_cards_dsl::AbilityDef::ModalSpell { modes, choose } if !choose.is_one() => {
                    mode_sets(modes, *choose).any(|set| {
                        probe(&mode_set_cost(ManaCost::ZERO, modes, set))
                            && mode_set_has_legal_targets(state, lookup, player, card, set)
                    })
                }
                baylee_cards_dsl::AbilityDef::ModalSpell { modes, .. } => {
                    probe(&ManaCost::ZERO)
                        && modes.iter().enumerate().any(|(i, mode)| {
                            mode.cost_override.is_none()
                                && mode_has_a_legal_target(state, lookup, player, card, i)
                        })
                }
                _ => false,
            })
        } else {
            probe(&ManaCost::ZERO)
        };
        return if payable {
            Ok(())
        } else {
            Err(CastError::NotEnoughMana)
        };
    }
    let probe_back = |def: &baylee_cards_dsl::CardDef, i: usize| {
        let face = &def.faces[i];
        let tax = face_increase(state, player, card, def, i, None);
        let reduction = printed_reduction(state, face, player, card)
            .saturating_add(keyword_reduction(state, face, player, card));
        affordable(
            state,
            player,
            pool,
            &face
                .mana_cost
                .with_x(0)
                .with_more_generic(tax)
                .with_less_generic(reduction),
        )
    };
    // A back face that may be cast now, affordable and with something to
    // point at: the same three questions the wizard asks of it.
    let a_back_face_castable = || {
        printed.is_some_and(|def| {
            castable_back_faces(def, on_adventure).any(|(i, _)| {
                face_timing_allows(state, player, card, def, i)
                    && probe_back(def, i)
                    && face_has_a_legal_target(state, lookup, player, card, i)
            })
        })
    };
    // The front may not be cast now, so no front-face price is an answer:
    // only a back face can make the card castable.
    if !front_now {
        return if a_back_face_castable() {
            Ok(())
        } else {
            Err(CastError::NotEnoughMana)
        };
    }
    // A printed flashback is paid "rather than its mana cost" (CR 702.34a),
    // so from the graveyard its cost is the price — or, beside a grant
    // (Past in Flames), one of two, the grant's being the mana cost below.
    if flashback_ok && let Some(cost) = printed_flashback {
        if probe(&cost.with_x(0)) {
            return Ok(());
        }
        if !flashback_granted(state, card) {
            return Err(CastError::NotEnoughMana);
        }
    }
    // Escape's price is its mana and the other cards it exiles; beside a
    // graveyard permission (Muldrotha) the mana cost below is a second way.
    if escape_ok && let Some(escape) = printed_escape {
        let fodder = escape_exile_options(state, player, card).len() >= usize::from(escape.exile);
        if fodder && probe(&escape.cost.with_x(0)) {
            return Ok(());
        }
        // Too few other cards is a cost that cannot be paid (CR 601.2h),
        // whatever the pool holds, as a missing sacrifice is above.
        if !graveyard_ok {
            return Err(if fodder {
                CastError::NotEnoughMana
            } else {
                CastError::NoWayToCast
            });
        }
    }
    if let Some(def) = printed
        && let Some(req) = def.faces[0].kicked_targets
    {
        let ordinary =
            ordinary_targets_reachable(state, def, player, card) && probe(&c.mana_cost.with_x(0));
        let kicked = requirement_is_reachable(Some(req), state, player, card)
            && probe(&kicked_mana_cost(&def.faces[0]).with_x(0));
        return if ordinary || kicked {
            Ok(())
        } else {
            Err(CastError::NoWayToCast)
        };
    }
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
                    && probe_back(def, i)
                    && face_has_a_legal_target(state, lookup, player, card, i)
            })
        });
        if affordable_disturb {
            return Ok(());
        }
        // Muldrotha or Wrenn's emblem cast the *front* from the graveyard
        // too, at its own price — the probes below.
        if !graveyard_ok {
            return Err(CastError::NotEnoughMana);
        }
    }
    // Printed cost probed with X = 0, and after a reduction printed on the
    // card itself; the full payment is validated when the wizard finishes.
    // A face with no printed cost has no normal way to be cast at all
    // (CR 202.1b) and falls straight through to the alternatives.
    let normal_cost = c.mana_cost;
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
            baylee_cards_dsl::AbilityDef::ModalSpell { modes, choose } if !choose.is_one() => {
                mode_sets(modes, *choose).any(|set| {
                    probe(&mode_set_cost(face.mana_cost, modes, set).with_x(0))
                        && mode_set_has_legal_targets(state, lookup, player, card, set)
                })
            }
            baylee_cards_dsl::AbilityDef::ModalSpell { modes, .. } => {
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
        // Each face at its own timing (`face_timing_allows`), here and in
        // the wizard: a `{1}{W}` instant on the back of a creature is cast
        // whenever an instant could be, and a sorcery on the back of a
        // creature with flash is not.
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
        let any_face = a_back_face_castable();
        // Dash (CR 702.109a), the offer's `CastModeKind::Dash`.
        let any_dash = face.dash.is_some_and(|dash| probe(&dash));
        if !any_alt && !any_mode && !any_face && !any_dash {
            // Which of the two refused matters to whoever reads it. A mode
            // that was affordable and had nothing to point at is not a
            // player one land short, and telling them it is sends them
            // looking for the land.
            let a_mode_was_affordable = def.abilities.iter().any(|a| match a {
                baylee_cards_dsl::AbilityDef::ModalSpell { modes, choose } if !choose.is_one() => {
                    mode_sets(modes, *choose)
                        .any(|set| probe(&mode_set_cost(face.mana_cost, modes, set).with_x(0)))
                }
                baylee_cards_dsl::AbilityDef::ModalSpell { modes, .. } => modes
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

/// Increases are rules effects: match the spell as it will exist on the stack,
/// including its chosen face/form and continuous color changes. A white cost
/// on an alternative mode does not make a black spell white.
pub(crate) fn spell_increase(
    state: &GameState,
    player: PlayerId,
    object: &crate::object::GameObject,
) -> u32 {
    if !state
        .effects
        .iter()
        .any(|fx| matches!(fx.modifier, baylee_cards_dsl::Modifier::SpellsCostMore(_)))
    {
        return commander_tax(state, player, object.id);
    }
    let mut spell = object.clone();
    spell.zone = Zone::Stack;
    spell.controller = player;
    spell.base_controller = player;
    spell.cache.clear();
    let projected = crate::layers::recompute(state, &spell);
    spell.base = std::sync::Arc::new(projected.characteristics);
    state
        .effects
        .iter()
        .filter_map(|fx| match fx.modifier {
            baylee_cards_dsl::Modifier::SpellsCostMore(n)
                if crate::effects::applies_to(state, fx, &spell) =>
            {
                Some(n)
            }
            _ => None,
        })
        .fold(commander_tax(state, player, object.id), u32::saturating_add)
}

/// The total generic increase for a particular printed face or casting form.
pub(crate) fn face_increase(
    state: &GameState,
    player: PlayerId,
    card: ObjectId,
    def: &baylee_cards_dsl::CardDef,
    face: usize,
    form: Option<SpellForm>,
) -> u32 {
    if !state
        .effects
        .iter()
        .any(|fx| matches!(fx.modifier, baylee_cards_dsl::Modifier::SpellsCostMore(_)))
    {
        return commander_tax(state, player, card);
    }
    let Some(object) = state.object(card) else {
        return 0;
    };
    let mut spell = object.clone();
    spell.base = std::sync::Arc::new(crate::object::Characteristics::from_face(
        def,
        face,
        object.base.name,
    ));
    spell.cache.clear();
    spell.face_index = u8::try_from(face).unwrap_or(0);
    if let Some(form) = form {
        spell = form.project(&spell);
    }
    spell_increase(state, player, &spell)
}

/// Additional generic mana for activating an ability of this object. The
/// filter's normal zone boundary deliberately excludes white enchantment
/// *cards* cycling in hand; Gloom names enchantments, i.e. permanents.
pub(crate) fn activation_increase(state: &GameState, source: ObjectId) -> u32 {
    let Some(object) = state.object(source) else {
        return 0;
    };
    state
        .effects
        .iter()
        .filter_map(|fx| match fx.modifier {
            baylee_cards_dsl::Modifier::AbilitiesCostMore(n)
                if crate::effects::applies_to(state, fx, object) =>
            {
                Some(n)
            }
            _ => None,
        })
        .fold(0, u32::saturating_add)
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
        | CostPart::Crew(_)
        | CostPart::ReturnToHand(_)
        | CostPart::ExileFromGraveyard(_)
        | CostPart::DiscardSelf
        | CostPart::ExileSelf
        | CostPart::ReturnSelfToHand
        | CostPart::RemoveCounterSelfX { .. }
        | CostPart::PutCounterSelf { .. }
        | CostPart::PayLifeX => true,
    })
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
