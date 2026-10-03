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

/// A requirement whose filter reads the announced X, as the same spec over
/// every object and the filter it widened away. `None` for every other
/// requirement, and for the kinds no X-reading filter is printed on.
fn x_bounded(
    spec: baylee_cards_dsl::TargetSpec,
) -> Option<(
    baylee_cards_dsl::TargetSpec,
    &'static baylee_cards_dsl::Filter,
)> {
    use baylee_cards_dsl::{Filter, TargetSpec};
    static ANY: Filter = Filter::Any;
    let (widened, filter) = match spec {
        TargetSpec::Spell(f) => (TargetSpec::Spell(&ANY), f),
        TargetSpec::Object(f) => (TargetSpec::Object(&ANY), f),
        TargetSpec::StackOrBattlefield(f) => (TargetSpec::StackOrBattlefield(&ANY), f),
        _ => return None,
    };
    crate::eval::reads_announced_x(filter).then_some((widened, filter))
}

/// Whether `player` can spend `pool` to cover `cost` under current permissions.
pub(crate) fn affordable(
    state: &GameState,
    player: PlayerId,
    pool: &ManaPool,
    cost: &ManaCost,
) -> bool {
    mana_pay::can_pay_with(pool, cost, mana_spending(state, player))
}

/// Pays a cost that admits no restricted mana, under this player's current
/// spending permissions. Spell and activation payments use [`pay_mana_for`].
pub(crate) fn pay_mana(state: &mut GameState, player: PlayerId, cost: &ManaCost) -> bool {
    let spending = mana_spending(state, player);
    if let Some(obligation) = state.constrained_payment(player) {
        let real = &state.players[player.get() as usize].mana_pool;
        let Some(free) = real.payment_receipt(&obligation.required) else {
            return false;
        };
        let mut remainder = free.clone();
        if !mana_pay::pay_with(&mut remainder, cost, spending) {
            return false;
        }
        let Some(spent) = free.payment_receipt(&remainder) else {
            return false;
        };
        let Some(after) = real.payment_receipt(&spent) else {
            return false;
        };
        state.players[player.get() as usize].mana_pool = after;
        return true;
    }
    mana_pay::pay_with(
        &mut state.players[player.get() as usize].mana_pool,
        cost,
        spending,
    )
}

/// All actual mana units by type, with restricted units counted once.
pub(crate) fn mana_units_by_type(pool: &ManaPool) -> [u64; 6] {
    ManaColor::ALL.map(|color| {
        u64::from(pool.available(color))
            + pool
                .restricted()
                .iter()
                .filter(|unit| unit.color == color)
                .map(|unit| u64::from(unit.amount))
                .sum::<u64>()
    })
}

/// Record the actual types consumed by one successful mana payment.
pub(crate) fn record_mana_payment(
    paid: &mut crate::object::PaidRecord,
    before: &ManaPool,
    after: &ManaPool,
) {
    // Captured immediately around the payer; no other operation can produce
    // mana or alter restrictions between these snapshots.
    paid.mana_paid = before
        .payment_receipt(after)
        .expect("a successful payment only consumes existing mana");
    // The payer checks the finite single-payment domain before committing.
    paid.mana_types_spent = mana_units_by_type(&paid.mana_paid)
        .map(|n| u32::try_from(n).expect("checked payment domain"));
    paid.mana_spent = u32::try_from(paid.mana_paid.total()).expect("checked payment domain");
    paid.colors_spent = baylee_core::color::Color::ALL
        .into_iter()
        .filter(|&color| paid.mana_types_spent[ManaColor::from_color(color).index()] > 0)
        .fold(baylee_core::color::ColorSet::EMPTY, |set, color| {
            set.union(baylee_core::color::ColorSet::of(color))
        });
}

/// What a payment is for, which is what restricted mana asks (CR 106.6).
///
/// Cavern of Souls' mana may pay for a creature spell of the named type and
/// for nothing else, so the same pool answers two payments two ways. The
/// offer and the payment have to put the same question to it, or a spell
/// is offered in `LegalActions` and refused the moment it is taken.
/// Characteristics used to announce a special casting form, before payment.
#[derive(Clone, Copy, Debug)]
pub(crate) enum SpellForm {
    Prototype(baylee_cards_dsl::Prototype),
    Disguise,
}

impl SpellForm {
    pub(crate) fn project(self, object: &crate::object::GameObject) -> crate::object::GameObject {
        let mut object = object.clone();
        object.base = std::sync::Arc::new(object.characteristics().clone());
        object.cache.clear();
        let base = object.base_mut();
        match self {
            Self::Prototype(p) => {
                base.mana_cost = p.cost;
                base.colors = p.cost.colors();
                base.power = Some(p.power);
                base.toughness = Some(p.toughness);
            }
            Self::Disguise => {
                base.name = crate::state::NAMELESS;
                base.mana_cost = ManaCost::ZERO;
                base.colors = baylee_core::color::ColorSet::EMPTY;
                base.types = TypeSet::CREATURE;
                base.supertypes = baylee_core::types::SupertypeSet::EMPTY;
                base.subtypes = baylee_core::types::SubtypeSet::EMPTY;
                base.keywords = baylee_cards_dsl::KeywordSet::EMPTY;
                base.power = Some(2);
                base.toughness = Some(2);
                base.loyalty = None;
                base.produced_colors = baylee_core::color::ColorSet::EMPTY;
                base.produced_colorless = false;
                base.produced_chosen = false;
            }
        }
        object
    }

    fn cost(self) -> ManaCost {
        match self {
            Self::Prototype(p) => p.cost,
            Self::Disguise => const { ManaCost::parse("{3}") },
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum SpendFor {
    /// Casting this spell (CR 601.2h).
    Spell(ObjectId),
    /// An announced prototype or face-down creature.
    SpellAs(ObjectId, SpellForm),
    /// Activating an ability of this source (CR 602.2b, CR 113.7).
    Ability(
        #[expect(
            dead_code,
            reason = "no restriction the DSL can write names an ability yet"
        )]
        ObjectId,
    ),
    /// Anything else that costs mana: suspend (a special action, CR 116.2f),
    /// an "unless you pay", echo, a pact. No restriction names these.
    Other,
}

/// The restricted mana in `player`'s pool that may pay for `what`, in pool
/// order, each with the permanent that made it and the rider it carries.
///
/// An entry counts when its own filter matches the spell. The rider is not
/// consulted: it changes what the spell becomes once the mana is spent,
/// never whether the mana may be spent. A spell restriction admits no
/// ability. The walk is over the pool's list and never over the map, which
/// is keyed by a hash.
fn admitted(
    state: &GameState,
    player: PlayerId,
    what: SpendFor,
) -> SmallVec<[(RestrictedMana, ObjectId, SpendRider); 4]> {
    let (card, form) = match what {
        SpendFor::Spell(card) => (card, None),
        SpendFor::SpellAs(card, form) => (card, Some(form)),
        _ => return SmallVec::new(),
    };
    let Some(original) = state.object(card) else {
        return SmallVec::new();
    };
    let projected = form.map(|f| f.project(original));
    let spell = projected.as_ref().unwrap_or(original);
    state.players[player.get() as usize]
        .mana_pool
        .restricted()
        .iter()
        .filter_map(|mana| {
            let &(source, filter, rider) = state.restriction_info.get(&mana.restriction.0)?;
            crate::eval::matches(filter, state, spell, player, source)
                .then_some((*mana, source, rider))
        })
        .collect()
}

/// The mana in `player`'s pool that carries a rider for `what` and restricts
/// nothing (#232), in pool order, each with the permanent that made it and
/// its rider: the units whose filter matches the spell.
///
/// These units are in the plain counters already, so they are nothing to
/// merge. A spell the filter does not match, and every ability, spends them
/// as the ordinary mana they are, and no rider goes off (CR 106.6).
fn ridden_for(
    state: &GameState,
    player: PlayerId,
    what: SpendFor,
) -> SmallVec<[(RestrictedMana, ObjectId, SpendRider); 4]> {
    let (card, form) = match what {
        SpendFor::Spell(card) => (card, None),
        SpendFor::SpellAs(card, form) => (card, Some(form)),
        _ => return SmallVec::new(),
    };
    let Some(original) = state.object(card) else {
        return SmallVec::new();
    };
    let projected = form.map(|f| f.project(original));
    let spell = projected.as_ref().unwrap_or(original);
    state.players[player.get() as usize]
        .mana_pool
        .ridden()
        .iter()
        .filter_map(|mana| {
            let &(source, filter, rider) = state.restriction_info.get(&mana.restriction.0)?;
            crate::eval::matches(filter, state, spell, player, source)
                .then_some((*mana, source, rider))
        })
        .collect()
}

/// `pool` with every admitted restricted unit added to its plain counters.
fn merged(pool: &ManaPool, entries: &[(RestrictedMana, ObjectId, SpendRider)]) -> ManaPool {
    let mut merged = pool.clone();
    for (mana, ..) in entries {
        if mana.flags.contains(ManaFlags::SNOW) {
            merged.add_snow(mana.color, u32::from(mana.amount));
        } else {
            merged.add(mana.color, u32::from(mana.amount));
        }
    }
    merged
}

/// The pool a payment for `what` may draw on, or `None` when that is simply
/// the player's pool.
///
/// Restricted mana does not live in the pool's plain counters, and
/// [`mana_pay::can_pay`] reads nothing else. So a player whose only mana
/// came off a Cavern was offered **nothing to cast**, while the payment on
/// the far side of the wizard would have spent it without complaint. The
/// offer and the payment were answering different questions about the same
/// pool. They ask one question here, and [`pay_mana_for`] pays from this
/// same pool.
///
/// The card is still in a hand rather than on the stack, which is the one
/// difference from the payment site and does not reach these filters: they
/// read characteristics and a chosen subtype, neither of which the stack
/// confers.
///
/// It is `pub(crate)` because it has **two** callers and they must not
/// disagree: the wizard enumerates the ways to cast a spell with the same
/// probe `can_cast` used to offer it, or a spell is offered in
/// `LegalActions` and then refused as "no way to cast this spell" — which is
/// exactly what the convoke count above this one was written to stop
/// happening.
pub(crate) fn spendable_pool(
    state: &GameState,
    player: PlayerId,
    what: SpendFor,
) -> Option<ManaPool> {
    let pool = &state.players[player.get() as usize].mana_pool;
    if pool.restricted().is_empty() {
        return None;
    }
    Some(merged(pool, &admitted(state, player, what)))
}

/// Planning a payment may count optional life-to-mana actions. Their cost
/// remains a separate explicit player action; actual payment never uses this pool.
pub(crate) fn planning_pool(
    state: &GameState,
    player: PlayerId,
    what: SpendFor,
    reserved_life: u32,
) -> Option<ManaPool> {
    let mut merged = spendable_pool(state, player, what);
    let amount = state.granted_colorless_after_life(player, reserved_life);
    if amount > 0 {
        let pool = merged
            .get_or_insert_with(|| state.players[usize::from(player.get())].mana_pool.clone());
        pool.add(ManaColor::Colorless, amount);
    }
    merged
}

/// Mana units this payment can spend, without counting the restricted
/// entries a second time after merging them into the plain counters.
pub(crate) fn spendable_units(state: &GameState, player: PlayerId, what: SpendFor) -> u32 {
    let merged = spendable_pool(state, player, what);
    let pool = merged
        .as_ref()
        .unwrap_or(&state.players[usize::from(player.get())].mana_pool);
    let total: u64 = ManaColor::ALL
        .iter()
        .map(|color| u64::from(pool.available(*color)))
        .sum();
    u32::try_from(total).unwrap_or(u32::MAX)
}

/// Greatest affordable value within a known finite bound. Increasing X
/// cannot make a fixed cost cheaper, so this takes at most 32 probes even
/// when a player has a very large mana pool or life total.
pub(crate) fn greatest_affordable(mut upper: u32, mut affordable: impl FnMut(u32) -> bool) -> u32 {
    let mut lower = 0;
    while lower < upper {
        let middle = lower + (upper - lower).div_ceil(2);
        if affordable(middle) {
            lower = middle;
        } else {
            upper = middle - 1;
        }
    }
    lower
}

/// Pays `cost` for `what` out of `player`'s pool, and returns the restricted
/// mana it spent, and the rider-carrying mana it spent on a spell that rider
/// names ([`ridden_for`]), each part with its source and rider. `None`
/// leaves the pool untouched, because partial payments are not allowed
/// (CR 601.2h).
///
/// The payment is solved once, on the same pool [`spendable_pool`] offers
/// from, so it pays whatever was offered. The solver prefers the admitted
/// restricted units wherever it has a choice. That is what spends the
/// Cavern's unit rather than a land's beside it, so the rider lands. It
/// then charges what was consumed to the restricted entries first, colour
/// by colour and snow apart from the rest, and takes only what is left from
/// the plain counters. Surplus restricted mana stays in the pool as it was.
///
/// The payer this replaces took each admitted entry whole and subtracted it
/// from the cost one unit at a time, generic before the coloured pip,
/// because `ManaCost` sorts the generic part first. So a Cavern's {U} paid
/// the {1} of a {1}{U} spell and left the {U} to a Forest, which cannot pay
/// it. Three {C} from a Workshop were all taken for a {1}. A restricted {G}
/// that matched no pip was taken and lost. Under Mycosynth Lattice it
/// matched colours literally, so it refused what the Lattice made payable.
pub(crate) fn pay_mana_for(
    state: &mut GameState,
    player: PlayerId,
    what: SpendFor,
    cost: &ManaCost,
) -> Option<SmallVec<[(RestrictedMana, ObjectId, SpendRider); 4]>> {
    pay_mana_restricting_generic(state, player, what, cost, None)
}

/// The ordinary payment with an actual-mana restriction on a generic part.
pub(crate) fn pay_mana_restricting_generic(
    state: &mut GameState,
    player: PlayerId,
    what: SpendFor,
    cost: &ManaCost,
    restriction: Option<(ManaColor, u32)>,
) -> Option<SmallVec<[(RestrictedMana, ObjectId, SpendRider); 4]>> {
    let obligation = state.constrained_payment(player).cloned();
    let is_selected_spell = obligation.as_ref().is_some_and(|payment| match what {
        SpendFor::Spell(card) | SpendFor::SpellAs(card, _) => {
            payment.card.object == card && state.source_identity(card) == Some(payment.card)
        }
        _ => false,
    });
    let spending = mana_spending(state, player);
    let mut entries = admitted(state, player, what);
    let mut riding = ridden_for(state, player, what);
    if let Some(payment) = &obligation {
        entries.sort_by_key(|(mana, ..)| {
            !payment
                .required
                .restricted()
                .iter()
                .any(|unit| unit.restriction == mana.restriction)
        });
        riding.sort_by_key(|(mana, ..)| {
            !payment
                .required
                .ridden()
                .iter()
                .any(|unit| unit.restriction == mana.restriction)
        });
    }
    let real = &state.players[player.get() as usize].mana_pool;
    let merged = merged(real, &entries);
    let mut prefer = [0_u32; 6];
    for (mana, ..) in entries.iter().chain(&riding) {
        let slot = &mut prefer[mana.color.index()];
        *slot = slot.saturating_add(u32::from(mana.amount));
    }
    let Some(required) = obligation.as_ref().map_or(Some([0; 6]), |payment| {
        crate::constrained_payment::amounts(&payment.required)
    }) else {
        state.numeric_failure = Some("generated-mana obligation exceeds u32 per color");
        return None;
    };
    for i in 0..6 {
        prefer[i] = prefer[i].max(required[i]);
    }
    let paid = mana_pay::payment_consuming(
        &merged,
        cost,
        spending,
        prefer,
        restriction,
        if is_selected_spell { required } else { [0; 6] },
    )?;

    let (mut used_plain, mut used_snow) = consumed_mana(&merged, &paid, obligation.as_ref());

    // Restricted entries first, in pool order, each class charged apart.
    let mut pool = real.clone();
    let mut spent = SmallVec::new();
    for (mana, source, rider) in entries {
        let budget = if mana.flags.contains(ManaFlags::SNOW) {
            &mut used_snow[mana.color.index()]
        } else {
            &mut used_plain[mana.color.index()]
        };
        let k =
            u16::try_from(u32::from(mana.amount).min(*budget)).expect("bounded by entry amount");
        if k == 0 {
            continue;
        }
        let taken = pool.take_restricted_units(mana.restriction.0, k)?;
        *budget -= u32::from(taken.amount);
        spent.push((taken, source, rider));
    }
    // Then the rider units the spell sets off, out of the plain counters
    // they are counted in, before an ordinary spend keeps them back.
    for (mana, source, rider) in riding {
        let budget = if mana.flags.contains(ManaFlags::SNOW) {
            &mut used_snow[mana.color.index()]
        } else {
            &mut used_plain[mana.color.index()]
        };
        let k =
            u16::try_from(u32::from(mana.amount).min(*budget)).expect("bounded by entry amount");
        if k == 0 {
            continue;
        }
        let taken = pool.take_ridden_units(mana.restriction.0, k)?;
        *budget -= u32::from(taken.amount);
        spent.push((taken, source, rider));
    }
    // The rest off the plain counters. The solver paid from the real pool
    // plus the admitted units, and every class was charged to those units
    // first, so what is left fits the real pool. A remainder that did not
    // would be a solver fault, and it refuses rather than half-paying.
    for color in ManaColor::ALL {
        if !pool.spend(color, used_plain[color.index()]) {
            return None;
        }
        if !pool.spend_snow_units(color, used_snow[color.index()]) {
            return None;
        }
    }
    commit_mana_payment(state, player, pool, is_selected_spell, obligation.as_ref())?;
    Some(spent)
}

/// Validate the exact constrained receipt before publishing any pool debit.
fn commit_mana_payment(
    state: &mut GameState,
    player: PlayerId,
    pool: ManaPool,
    is_selected_spell: bool,
    obligation: Option<&crate::constrained_payment::ConstrainedPayment>,
) -> Option<()> {
    let before = state.players[player.get() as usize].mana_pool.clone();
    if is_selected_spell && let Some(payment) = obligation {
        let receipt = before.payment_receipt(&pool)?;
        if payment.required.restricted().iter().any(|unit| {
            receipt
                .restricted()
                .iter()
                .filter(|spent| spent.restriction == unit.restriction)
                .map(|spent| u32::from(spent.amount))
                .sum::<u32>()
                < u32::from(unit.amount)
        }) {
            return None;
        }
    }
    if before.payment_receipt(&pool)?.total() > u64::from(u32::MAX) {
        state.numeric_failure = Some("one recorded mana payment exceeds u32 total units");
        return None;
    }
    state.players[player.get() as usize].mana_pool = pool;
    state.note_constrained_payment(player, &before);
    Some(())
}

fn consumed_mana(
    merged: &ManaPool,
    paid: &ManaPool,
    obligation: Option<&crate::constrained_payment::ConstrainedPayment>,
) -> ([u32; 6], [u32; 6]) {
    // What the payment consumed, per colour, and how much of it was snow.
    // Exact, because `spend` takes ordinary units before snow ones and
    // `spend_snow` takes only snow ones.
    let mut used_snow = [0_u32; 6];
    let mut used_plain = [0_u32; 6];
    for color in ManaColor::ALL {
        let used = merged.available(color) - paid.available(color);
        let mut snow = merged.snow_available(color) - paid.snow_available(color);
        if let Some(payment) = obligation {
            let wanted = u64::from(payment.required.snow_available(color))
                + crate::constrained_payment::restricted_snow(&payment.required, color);
            snow =
                snow.max(u32::try_from(wanted.min(u64::from(used))).expect("bounded by used mana"));
        }
        used_snow[color.index()] = snow;
        used_plain[color.index()] = used - snow;
    }

    (used_plain, used_snow)
}

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

/// The permanent types a spell can be cast "of" under Muldrotha's allowance:
/// every permanent type but land, which is played and not cast (CR 305.9).
const SPELL_PERMANENT_TYPES: [TypeSet; 5] = [
    TypeSet::ARTIFACT,
    TypeSet::CREATURE,
    TypeSet::ENCHANTMENT,
    TypeSet::PLANESWALKER,
    TypeSet::BATTLE,
];

/// Whether each card in `cards` can be given a permanent type of its own,
/// no two the same: "a permanent spell of each permanent type", with a card
/// of several types using one of them. Five types at most, so the search is
/// small enough to try every assignment.
fn one_type_each(cards: &[TypeSet]) -> bool {
    fn assign(cards: &[TypeSet], used: u8) -> bool {
        let Some((first, rest)) = cards.split_first() else {
            return true;
        };
        SPELL_PERMANENT_TYPES.iter().enumerate().any(|(i, t)| {
            used & (1 << i) == 0 && first.contains(*t) && assign(rest, used | (1 << i))
        })
    }
    cards.len() <= SPELL_PERMANENT_TYPES.len() && assign(cards, 0)
}

/// Which permission lets a card be played from its owner's graveyard, when
/// the one that does is counted.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GraveyardPermission {
    /// Uncounted: Wrenn and Realmbreaker's emblem, Crucible of Worlds.
    Unlimited,
    /// Muldrotha's allowance, which this play uses up a part of.
    EachType {
        /// The allowance's source.
        source: ObjectId,
        /// Its version.
        version: u32,
    },
}

/// The Muldrotha-style allowances `player` holds right now: during their own
/// turn only ("during each of your turns"), each with the plays already made
/// under it this turn.
fn each_type_allowances(
    state: &GameState,
    player: PlayerId,
) -> impl Iterator<Item = (ObjectId, u32, Vec<TypeSet>)> + '_ {
    let my_turn = state.turn.active == player;
    state
        .effects
        .iter()
        .filter(move |fx| {
            my_turn
                && fx.controller == player
                && matches!(
                    fx.modifier,
                    baylee_cards_dsl::Modifier::PermanentOfEachTypeFromGraveyard
                )
        })
        .filter_map(move |fx| {
            let source = fx.source?;
            let version = state.object(source)?.version;
            let plays = state
                .per_turn
                .graveyard_plays
                .iter()
                .filter(|p| p.player == player && p.source == source && p.version == version)
                .map(|p| p.types)
                .collect();
            Some((source, version, plays))
        })
}

/// Which permission, if any, lets `player` cast the card `obj` from their
/// own graveyard: Forgotten Cellar's first, the one that casts any spell;
/// then, for a permanent card, Wrenn's emblem, because it costs nothing to
/// use, and a Muldrotha allowance with a type still open for it. A land card
/// is played and never cast (CR 305.9), whichever permission is there.
#[must_use]
pub fn graveyard_cast_permission(
    state: &GameState,
    player: PlayerId,
    obj: &crate::object::GameObject,
) -> Option<GraveyardPermission> {
    let types = obj.characteristics().types;
    if obj.zone != Zone::Graveyard
        || obj.zone_owner != Some(player)
        || types.contains(TypeSet::LAND)
    {
        return None;
    }
    // "You may cast spells from your graveyard this turn": every spell, so
    // it is asked before the word "permanent" is.
    if state.effects.iter().any(|fx| {
        fx.controller == player
            && matches!(
                fx.modifier,
                baylee_cards_dsl::Modifier::CastSpellsFromGraveyard
            )
    }) {
        return Some(GraveyardPermission::Unlimited);
    }
    if !types.is_permanent() {
        return None;
    }
    if state.effects.iter().any(|fx| {
        fx.controller == player
            && matches!(
                fx.modifier,
                baylee_cards_dsl::Modifier::CastPermanentSpellsFromGraveyard
            )
    }) {
        return Some(GraveyardPermission::Unlimited);
    }
    each_type_allowances(state, player).find_map(|(source, version, mut plays)| {
        plays.retain(|t| !t.contains(TypeSet::LAND));
        plays.push(types);
        one_type_each(&plays).then_some(GraveyardPermission::EachType { source, version })
    })
}

/// Which permission lets `player` play a land from their own graveyard:
/// Crucible of Worlds' first, then a Muldrotha allowance whose land is still
/// unplayed this turn.
#[must_use]
pub fn graveyard_land_permission(
    state: &GameState,
    player: PlayerId,
) -> Option<GraveyardPermission> {
    if state.effects.iter().any(|fx| {
        fx.controller == player
            && matches!(
                fx.modifier,
                baylee_cards_dsl::Modifier::PlayLandsFromGraveyard
            )
    }) {
        return Some(GraveyardPermission::Unlimited);
    }
    each_type_allowances(state, player).find_map(|(source, version, plays)| {
        (!plays.iter().any(|t| t.contains(TypeSet::LAND)))
            .then_some(GraveyardPermission::EachType { source, version })
    })
}

/// Writes down a play made under a Muldrotha allowance.
pub fn note_graveyard_play(
    state: &mut GameState,
    player: PlayerId,
    permission: Option<GraveyardPermission>,
    types: TypeSet,
) {
    if let Some(GraveyardPermission::EachType { source, version }) = permission {
        state
            .per_turn
            .graveyard_plays
            .push(crate::state::GraveyardPlay {
                player,
                source,
                version,
                types,
            });
    }
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
        Zone::Graveyard => graveyard_land_permission(state, player).is_some(),
        Zone::Library => state.effects.iter().any(|fx| {
            fx.controller == player
                && matches!(
                    fx.modifier,
                    baylee_cards_dsl::Modifier::PlayLandsFromLibraryTop
                )
        }),
        _ => false,
    }
}

/// The cards `player` may exile to pay `card`'s escape cost: every other
/// card in their graveyard (CR 702.138a, "exile [N] other cards"). The same
/// list for the offer's count and the cast wizard's question.
#[must_use]
pub fn escape_exile_options(state: &GameState, player: PlayerId, card: ObjectId) -> Vec<ObjectId> {
    state
        .zones
        .list(ZoneLocation::Graveyard(player))
        .iter()
        .copied()
        .filter(|id| *id != card)
        .collect()
}

/// The permission `player` holds to play `card` this turn, if any
/// ([`crate::state::PlayPermission`]): one given for this very object, so a
/// card that has moved since holds none (CR 400.7).
#[must_use]
pub fn play_permission(
    state: &GameState,
    player: PlayerId,
    card: ObjectId,
) -> Option<crate::state::PlayPermission> {
    let version = state.object(card)?.version;
    state
        .per_turn
        .playable
        .iter()
        .copied()
        .find(|p| p.player == player && p.card == card && p.version == version)
}

/// The zone permission plus the particular card restriction. A library
/// permission never grants access to a card below the top. A permission for
/// the card itself ([`play_permission`]) opens it wherever it lies, an
/// opponent's exile included — unless it lets the card be cast and nothing
/// else (Ragavan, Nimble Pilferer), which plays no land (CR 601.1a).
#[must_use]
pub fn land_card_open(state: &GameState, player: PlayerId, card: ObjectId) -> bool {
    let version = state.object(card).map(|o| o.version);
    if state
        .per_turn
        .playable
        .iter()
        .any(|p| p.player == player && p.card == card && Some(p.version) == version && !p.cast_only)
    {
        return true;
    }
    state.object(card).is_some_and(|obj| {
        obj.zone_owner == Some(player)
            && land_zone_open(state, player, obj.zone)
            && (obj.zone != Zone::Library
                || state.zones.list(ZoneLocation::Library(player)).last() == Some(&card))
    })
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
    play_land_with_timing(state, player, card, false)
}

/// A resolving instruction waives priority/main-phase timing, not the turn or allowance.
pub(crate) fn play_land_by_effect(
    state: &mut GameState,
    player: PlayerId,
    card: ObjectId,
) -> Result<(), CastFailure> {
    play_land_with_timing(state, player, card, true)
}

fn play_land_with_timing(
    state: &mut GameState,
    player: PlayerId,
    card: ObjectId,
    by_effect: bool,
) -> Result<(), CastFailure> {
    let obj = state.object(card).ok_or(CastFailure::NoSuchObject)?;
    if !land_card_open(state, player, card) {
        return Err(CastFailure::Legality(CastError::NotInHand));
    }
    if !obj.characteristics().types.contains(TypeSet::LAND) {
        return Err(CastFailure::Legality(CastError::BadTiming));
    }
    let main_phase = matches!(state.turn.phase, Phase::FirstMain | Phase::SecondMain);
    if state.turn.active != player || (!by_effect && (!main_phase || !state.zones.stack_is_empty()))
    {
        return Err(CastFailure::Legality(CastError::BadTiming));
    }
    if !has_a_land_drop_left(state, player) {
        return Err(CastFailure::Legality(CastError::BadTiming));
    }
    // A land from the graveyard under Muldrotha's allowance uses its land
    // for the turn; one a permission for this very card opened does not.
    if obj.zone == Zone::Graveyard && play_permission(state, player, card).is_none() {
        let permission = graveyard_land_permission(state, player);
        note_graveyard_play(state, player, permission, TypeSet::LAND);
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

/// Every mana color a land's **basic types** entitle it to (CR 305.6), in
/// the rules' own order.
///
/// One entry per basic type present, because CR 305.6 gives the land one
/// mana ability per type and the controller picks which to activate. Empty
/// for a land with no basic type and for anything that is not a land.
///
/// **A land can gain a basic type it was not printed with**, and that is why
/// this returns a list rather than an `Option`. It used to answer `None` for
/// a land with several — deliberately, so that Godless Shrine would not tap
/// for white and never for black — and the dual was left to the
/// `AddManaChoice` ability printed on its card, which does ask. That works
/// for a card whose *printed* type line carries both types. It cannot work
/// for a type a continuous effect adds: a basic Forest under Urborg, Tomb of
/// Yawgmoth is a Forest Swamp, and the only ability it prints is the green
/// one, so the black CR 305.6 gives it existed nowhere. The land kept making
/// green and silently made no black at all — which is the shape of this
/// defect and worth stating precisely, because "Urborg turns off your
/// lands" would have been the wrong reading and the wrong fix. Four cards in
/// this pool add a basic land type: Urborg, Yavimaya, Blanket of Night and
/// Ashaya.
#[must_use]
pub fn intrinsic_mana_colors(state: &GameState, source: ObjectId) -> Vec<ManaColor> {
    let Some(obj) = state.object(source) else {
        return Vec::new();
    };
    if !obj.characteristics().types.contains(TypeSet::LAND)
        || obj.characteristics().abilities_lost.is_some()
    {
        return Vec::new();
    }
    let s = &obj.characteristics().subtypes;
    [
        (land::PLAINS, ManaColor::White),
        (land::ISLAND, ManaColor::Blue),
        (land::SWAMP, ManaColor::Black),
        (land::MOUNTAIN, ManaColor::Red),
        (land::FOREST, ManaColor::Green),
    ]
    .into_iter()
    .filter(|(subtype, _)| s.contains(*subtype))
    .map(|(_, color)| color)
    .collect()
}

/// The colors the CR 305.6 shortcut may still offer for this land: its basic
/// types, less whatever its own card already prints a mana ability for.
///
/// **The subtraction is the whole of it.** The ten original duals print a
/// `Add {R} or {G}` ability that codegen wrote off the very same type line
/// this rule reads — one ability, rendered twice — so a shortcut that
/// ignored the card would offer Taiga two ways to tap and `land_mana_tests`
/// would say so, ten times over. What the subtraction keeps is the half the
/// card *cannot* print: a basic Forest under Urborg, Tomb of Yawgmoth is a
/// Forest Swamp, prints nothing at all, and needs the rule for both colours;
/// Taiga under the same Urborg needs it for the black alone, beside the
/// printed ability that still makes its red and green. Three mana abilities
/// on one land is what CR 305.6 actually says, and this is the only reader
/// that can count them.
///
/// Read through [`baylee_cards_dsl::mana_made`] and not `simple_mana`,
/// because a restricted printed ability is still a printed one: what is
/// being asked is "does the card already say this colour", not "may a
/// planner spend it".
#[must_use]
pub fn intrinsic_mana_offer(
    state: &GameState,
    lookup: &impl crate::state::CardLookup,
    source: ObjectId,
) -> Vec<ManaColor> {
    let mut colors = intrinsic_mana_colors(state, source);
    if colors.len() < 2 {
        // One basic type is the case this shortcut has always served, and no
        // land in the pool prints an ability duplicating its single type —
        // leaving it alone keeps every existing offer byte for byte.
        return colors;
    }
    let Some(obj) = state.object(source) else {
        return Vec::new();
    };
    for ability in obj.abilities(lookup) {
        let (baylee_cards_dsl::AbilityDef::Activated { cost, effects, .. }
        | baylee_cards_dsl::AbilityDef::ActivatedConditional { cost, effects, .. }) = ability
        else {
            continue;
        };
        if let Some((made, _restricted)) = baylee_cards_dsl::mana_made(cost, effects) {
            colors.retain(|c| !made.colors.contains(c));
        }
    }
    colors
}

/// The colours `player` may tap `source` for through the CR 305.6 shortcut
/// right now, and so what `PlayerAction::ActivateManaAbility` does with it:
/// empty when the shortcut is closed (a land it cannot activate now, or one
/// whose own card prints every colour its types give it), one colour to add,
/// several to ask.
///
/// The one predicate for the offer (`legal.mana_abilities`) and for `apply`.
/// They used to ask two: the offer this, and `apply` only
/// [`can_activate_mana`]. A dual land prints its own "Add {G} or {U}", so its
/// shortcut is empty and it is offered through that printed ability; under
/// Chromatic Lantern it is also in `mana_abilities` for the granted "{T}: Add
/// one mana of any color". `apply` saw an untapped land with basic types,
/// took the shortcut, found no colour in it and refused the press the offer
/// had just listed (Breeding Pool, Stomping Ground, Canopy Vista: 63 refusals
/// in 10,000 fuzzed games), where the granted ability was the one to take.
#[must_use]
pub fn intrinsic_mana_choices(
    state: &GameState,
    lookup: &impl crate::state::CardLookup,
    player: PlayerId,
    source: ObjectId,
) -> Vec<ManaColor> {
    if !can_activate_mana(state, player, source) {
        return Vec::new();
    }
    intrinsic_mana_offer(state, lookup, source)
}

/// The one color a land's basic types entitle it to, where there is exactly
/// one and so nothing to ask.
///
/// `None` where the land has several, which is a question and not an answer;
/// [`intrinsic_mana_colors`] is what a caller that can ask reads instead.
#[must_use]
pub fn intrinsic_mana(state: &GameState, source: ObjectId) -> Option<ManaColor> {
    match intrinsic_mana_colors(state, source).as_slice() {
        [only] => Some(*only),
        _ => None,
    }
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

pub(crate) fn intrinsic_mana_price(state: &GameState, source: ObjectId) -> ManaCost {
    ManaCost::ZERO.with_more_generic(activation_increase(state, source))
}

pub(crate) fn pay_intrinsic_mana_price(
    state: &mut GameState,
    player: PlayerId,
    source: ObjectId,
) -> bool {
    let cost = intrinsic_mana_price(state, source);
    pay_mana_for(state, player, SpendFor::Ability(source), &cost).is_some()
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
        && !intrinsic_mana_colors(state, source).is_empty()
        && affordable(state, player, &state.players[player.get() as usize].mana_pool,
            &intrinsic_mana_price(state, source))
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
    let colors = intrinsic_mana_colors(state, source);
    let [color] = colors.as_slice() else {
        // Several basic types is a question, and this function cannot ask
        // one: `actions.rs` taps the land and publishes `Pending::ChooseColor`
        // instead, then finishes here through `add_intrinsic_mana`. Reaching
        // this arm means a caller skipped that fork.
        return Err(CastFailure::Legality(CastError::BadTiming));
    };
    if !pay_intrinsic_mana_price(state, player, source) {
        return Err(CastFailure::Legality(CastError::NotEnoughMana));
    }
    add_intrinsic_mana(state, player, source, *color);
    Ok(())
}

/// Taps a land for one mana of `color` and pays it into the pool — the half
/// of [`activate_mana`] that happens once the colour is settled.
///
/// Its own function because the colour arrives two ways: straight out of the
/// land's one basic type, or out of a `Pending::ChooseColor` the player
/// answered because the land has several. One door, so the two cannot come
/// out as different events — the tap is journalled under [`Cause::Cost`] and
/// the mana under [`GameEvent::ManaProduced`] either way, and a trigger
/// watching for either sees the same thing.
pub fn add_intrinsic_mana(
    state: &mut GameState,
    player: PlayerId,
    source: ObjectId,
    color: ManaColor,
) {
    if state.players[player.get() as usize]
        .mana_pool
        .available(color)
        == u32::MAX
    {
        state.numeric_failure = Some("mana production exceeds u32 per color");
        return;
    }
    let obligation_before = state
        .constrained_payment(player)
        .map(|_| state.players[usize::from(player.get())].mana_pool.clone());
    state.set_tapped(source, true);
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
    if let Some(before) = obligation_before {
        state.note_constrained_production(player, &before);
    }
    state.journal.record(GameEvent::ManaProduced {
        player,
        color,
        amount: 1,
        source: Some(source),
    });
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
            origin: crate::effects::EffectOrigin::Resolution,
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

    /// A card of the given types in `owner`'s hand, carrying nothing else.
    fn card_in_hand(
        state: &mut GameState,
        owner: PlayerId,
        name: &str,
        types: TypeSet,
    ) -> ObjectId {
        let name = state.names.intern(name);
        let id = state.create_bare(owner, ObjectKind::Card, name, ZoneLocation::Hand(owner));
        state.object_mut(id).expect("just made it").base_mut().types = types;
        id
    }

    /// A cast an effect forbids outright is a different answer from a
    /// sorcery-speed lock, which is the whole reason `OpponentsCantCast` is
    /// its own variant: every row below is taken in `me()`'s own main phase
    /// with an empty stack, the one moment `timing_allows` says yes to
    /// everything.
    ///
    /// Three rows, each alone the point. The spell is castable before the
    /// effect exists — so the refusal is the effect and not the setup. It is
    /// refused while an opponent's effect names it. And it is **not**
    /// refused for the seat that controls the effect, because "your
    /// opponents" is asked per seat and never reaches you.
    #[test]
    fn a_cast_lock_is_asked_per_seat_and_not_of_its_own_controller() {
        let mut state = state();
        main_phase_of(&mut state, me());
        let spell = card_in_hand(&mut state, me(), "Probe", TypeSet::INSTANT);

        let obj = state.object(spell).expect("just made it");
        assert!(
            !cast_is_forbidden(&state, me(), obj),
            "nothing forbids it yet"
        );

        register(
            &mut state,
            them(),
            Modifier::OpponentsCantCast(&Filter::Any),
        );
        let obj = state.object(spell).expect("still there");
        assert!(
            cast_is_forbidden(&state, me(), obj),
            "an opponent's lock reaches this seat"
        );
        assert!(
            !cast_is_forbidden(&state, them(), obj),
            "and not the seat that controls it — \"your opponents\" is asked \
             per seat"
        );
    }

    /// The filter the lock carries is read, and is read from the **effect's**
    /// controller: Ranger-Captain of Eos forbids noncreature spells, so a
    /// creature card in the same hand is still castable. Without this the
    /// variant would be a bit rather than a sentence, and Silence and the
    /// Ranger-Captain would be the same card.
    #[test]
    fn a_cast_lock_forbids_only_what_its_filter_names() {
        let mut state = state();
        main_phase_of(&mut state, me());
        let instant = card_in_hand(&mut state, me(), "Probe", TypeSet::INSTANT);
        let creature = card_in_hand(&mut state, me(), "Bear", TypeSet::CREATURE);
        register(
            &mut state,
            them(),
            Modifier::OpponentsCantCast(&Filter::NONCREATURE),
        );

        assert!(
            cast_is_forbidden(&state, me(), state.object(instant).expect("there")),
            "an instant is a noncreature spell"
        );
        assert!(
            !cast_is_forbidden(&state, me(), state.object(creature).expect("there")),
            "and a creature is not"
        );
    }

    #[test]
    fn disguise_uses_creature_characteristics_for_restrictions_and_payment() {
        let mut state = state();
        main_phase_of(&mut state, me());
        let card = card_in_hand(&mut state, me(), "land", TypeSet::LAND);
        register(
            &mut state,
            them(),
            Modifier::OpponentsCantCast(&Filter::NONCREATURE),
        );
        let original = state.object(card).unwrap();
        let disguise = SpellForm::Disguise.project(original);
        assert!(!crate::eval::matches(
            &Filter::Named("land"),
            &state,
            &disguise,
            me(),
            card
        ));
        assert_eq!(state.names.get(disguise.characteristics().name), "");
        assert!(cast_is_forbidden(&state, me(), original));
        assert!(!cast_is_forbidden(
            &state,
            me(),
            &SpellForm::Disguise.project(original)
        ));
        let id = baylee_core::mana::RestrictionId(42);
        state
            .restriction_info
            .insert(42, (card, &Filter::CREATURE, SpendRider::None));
        state.players[0].mana_pool.add_restricted(RestrictedMana {
            color: ManaColor::Colorless,
            amount: 3,
            flags: ManaFlags::default(),
            restriction: id,
        });
        let cost = ManaCost::parse("{3}");
        assert!(pay_mana_for(&mut state, me(), SpendFor::Spell(card), &cost).is_none());
        assert!(
            pay_mana_for(
                &mut state,
                me(),
                SpendFor::SpellAs(card, SpellForm::Disguise),
                &cost
            )
            .is_some()
        );
        assert!(state.players[0].mana_pool.restricted().is_empty());
        assert_eq!(
            state.object(card).unwrap().characteristics().types,
            TypeSet::LAND,
            "a probe never changes the card"
        );
    }

    #[test]
    fn directed_spending_preserves_restrictions_snow_and_actual_rider_units() {
        for snow in [false, true] {
            for ridden in [false, true] {
                let mut state = state();
                let creature = card_in_hand(&mut state, me(), "Creature", TypeSet::CREATURE);
                let instant = card_in_hand(&mut state, me(), "Instant", TypeSet::INSTANT);
                register(
                    &mut state,
                    me(),
                    Modifier::SpendManaAs {
                        from: ManaColor::White,
                        to: ManaColor::Red,
                    },
                );
                state
                    .restriction_info
                    .insert(42, (creature, &Filter::CREATURE, SpendRider::Uncounterable));
                let mana = RestrictedMana {
                    color: ManaColor::White,
                    amount: 2,
                    flags: if snow {
                        ManaFlags::SNOW
                    } else {
                        ManaFlags::NONE
                    },
                    restriction: baylee_core::mana::RestrictionId(42),
                };
                if ridden {
                    state.players[0].mana_pool.add_ridden(mana);
                } else {
                    state.players[0].mana_pool.add_restricted(mana);
                    let before = state.players[0].mana_pool.clone();
                    for what in [
                        SpendFor::Spell(instant),
                        SpendFor::Ability(creature),
                        SpendFor::Other,
                    ] {
                        assert!(
                            pay_mana_for(&mut state, me(), what, &ManaCost::parse("{R}")).is_none()
                        );
                        assert_eq!(state.players[0].mana_pool, before);
                    }
                }
                state.players[0].mana_pool.add(ManaColor::Red, 1);
                let cost = ManaCost::parse("{R}");
                let pool = spendable_pool(&state, me(), SpendFor::Spell(creature));
                assert!(affordable(
                    &state,
                    me(),
                    pool.as_ref().unwrap_or(&state.players[0].mana_pool),
                    &cost
                ));
                let paid =
                    pay_mana_for(&mut state, me(), SpendFor::Spell(creature), &cost).unwrap();
                assert_eq!(
                    paid.as_slice(),
                    &[(
                        RestrictedMana { amount: 1, ..mana },
                        creature,
                        SpendRider::Uncounterable
                    )]
                );
                let pool = &state.players[0].mana_pool;
                assert_eq!(
                    pool.available(ManaColor::Red),
                    1,
                    "the preferred white unit paid red"
                );
                let remaining = if ridden {
                    pool.ridden()
                } else {
                    pool.restricted()
                };
                assert_eq!(remaining, &[RestrictedMana { amount: 1, ..mana }]);
                if snow {
                    assert!(
                        pay_mana_for(
                            &mut state,
                            me(),
                            SpendFor::Spell(creature),
                            &ManaCost::parse("{R}{S}")
                        )
                        .is_some()
                    );
                    assert!(state.players[0].mana_pool.is_empty());
                }
            }
        }
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
        state.set_tapped(id, true);
    }

    /// Convoke pays with untapped creatures its caster controls (CR
    /// 702.51a), and each of those words is a row: a tapped one is not a
    /// source, an opponent's is not a source, a land is not one however
    /// untapped it is, and a phased-out one does not exist (CR 702.26b).
    /// Waterbend pays with the same and with artifacts too (CR 701.67a).
    ///
    /// #229: the two were one walk over creatures and artifacts, and this
    /// test asserted it. The walk read the whole battlefield, phased-out
    /// permanents included.
    ///
    /// The count is what the offer and the payment both read. They disagreed
    /// once and the result was a convoke spell offered exactly when its
    /// printed cost was already payable — which is the one case convoke is
    /// not for. A waterbend adds nothing to the offer (CR 701.67b).
    #[test]
    fn convoke_taps_creatures_and_waterbend_artifacts_too_on_one_side() {
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
        for phased in [
            creature(&mut state, me(), "Phased creature"),
            artifact(&mut state, me(), "Phased artifact"),
        ] {
            state
                .object_mut(phased)
                .expect("made it")
                .status
                .insert(crate::object::Status::PHASED_OUT);
        }

        assert_eq!(
            convoke_sources(&state, me()),
            vec![bear],
            "an artifact, a tapped one, an opponent's, a land and a phased-out \
             one are none of them"
        );
        assert_eq!(
            waterbend_sources(&state, me()),
            vec![bear, mox],
            "waterbend taps the artifact as well, and nothing else more"
        );
        assert_eq!(
            convoke_sources(&state, them()),
            vec![untapped_theirs],
            "and the other side counts its own"
        );

        // What the keyword is then worth, which is the number the two probes
        // must agree on — and nothing at all on a face that does not print it.
        assert_eq!(
            keyword_reduction(&state, &probe_face(true, false, None), me(), bear),
            1
        );
        assert_eq!(
            keyword_reduction(&state, &probe_face(false, false, None), me(), bear),
            0
        );
        let mut waterbend = probe_face(false, false, None);
        waterbend.waterbend = true;
        assert_eq!(
            keyword_reduction(&state, &waterbend, me(), bear),
            0,
            "a waterbend's taps pay the waterbend and never the printed cost"
        );
    }

    /// Delve pays with the caster's graveyard (CR 702.66a), one card each,
    /// and it stacks with convoke on a face that printed both. Dig Through
    /// Time is the pool's only delve card and was offered at eight mana or
    /// not at all, with a full graveyard doing nothing.
    #[test]
    fn delve_counts_a_graveyard_and_adds_to_whatever_else_the_face_prints() {
        let mut state = state();
        let buried: Vec<ObjectId> = (0..3)
            .map(|i| {
                let name = state.names.intern(&format!("Buried {i}"));
                state.create_bare(me(), ObjectKind::Card, name, ZoneLocation::Graveyard(me()))
            })
            .collect();
        let name = state.names.intern("Theirs");
        state.create_bare(
            them(),
            ObjectKind::Card,
            name,
            ZoneLocation::Graveyard(them()),
        );
        let bear = creature(&mut state, me(), "Bear");

        assert_eq!(
            keyword_reduction(&state, &probe_face(false, true, None), me(), bear),
            3,
            "my graveyard, and not the table's"
        );
        assert_eq!(
            keyword_reduction(&state, &probe_face(false, true, None), them(), bear),
            1
        );
        assert_eq!(
            keyword_reduction(&state, &probe_face(true, true, None), me(), bear),
            4,
            "a face printing both adds them"
        );
        // Cast out of that graveyard, the spell is on the stack while it is
        // paid for (CR 601.2a) and is not one of the cards it may exile.
        assert_eq!(
            keyword_reduction(&state, &probe_face(false, true, None), me(), buried[0]),
            2,
            "the card being cast is not its own delve"
        );
    }

    /// A printed reduction is read off the card and asked of the seat:
    /// Surgical Metamorph costs `{1}` less if you were not the starting
    /// player, so the seat it is *for* is the one the offer used to leave it
    /// out for.
    #[test]
    fn a_printed_reduction_reaches_the_seat_it_was_printed_for() {
        let mut state = state();
        let card = permanent(&mut state, me(), "Probe");
        let face = probe_face(
            false,
            false,
            Some(baylee_cards_dsl::CostReduction::NotStartingPlayer(1)),
        );
        let starter = state.starting_player;
        let other = if starter == me() { them() } else { me() };

        assert_eq!(printed_reduction(&state, &face, starter, card), 0);
        assert_eq!(printed_reduction(&state, &face, other, card), 1);
        assert_eq!(
            printed_reduction(&state, &probe_face(false, false, None), other, card),
            0,
            "and a card that prints no reduction gets none"
        );
    }

    /// "Costs {1} less … for each creature you control" counts for the seat
    /// paying: two creatures of mine take two off my price, and my
    /// opponent's one creature takes one off theirs. `each` multiplies.
    #[test]
    fn a_counted_reduction_takes_generic_mana_per_thing_counted() {
        let mut state = state();
        let card = permanent(&mut state, me(), "Probe");
        creature(&mut state, me(), "Mine");
        creature(&mut state, me(), "Also mine");
        creature(&mut state, them(), "Theirs");
        let per = |each| {
            probe_face(
                false,
                false,
                Some(baylee_cards_dsl::CostReduction::PerCount {
                    amount: baylee_cards_dsl::Amount::CountOf {
                        filter: &Filter::YOUR_CREATURE,
                        zone: baylee_cards_dsl::ZoneSel::Battlefield,
                    },
                    each,
                }),
            )
        };
        assert_eq!(printed_reduction(&state, &per(1), me(), card), 2);
        assert_eq!(printed_reduction(&state, &per(1), them(), card), 1);
        assert_eq!(printed_reduction(&state, &per(2), me(), card), 4);
        let cost = ManaCost::parse("{1}{G}");
        assert_eq!(
            cost.with_less_generic(printed_reduction(&state, &per(1), me(), card)),
            ManaCost::parse("{G}"),
            "generic only, and never below nothing (CR 118.7a)"
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

    /// CR 305.6 gives a land one mana ability **per** basic type, so a land
    /// with two of them is a question rather than an answer — and the two
    /// readers say so differently on purpose. `intrinsic_mana` is the
    /// single-answer shortcut and stays `None`, because answering on the
    /// player's behalf is worse than not answering: Godless Shrine used to
    /// tap for white and never for black, whatever the player needed.
    /// `intrinsic_mana_colors` is what a caller that *can* ask reads, and it
    /// names both.
    #[test]
    fn a_land_with_two_basic_types_is_a_question_and_not_an_answer() {
        let mut state = state();
        let plains = basic_land(&mut state, "Plains", &[land::PLAINS]);
        let shrine = basic_land(&mut state, "Godless Shrine", &[land::PLAINS, land::SWAMP]);
        let waste = basic_land(&mut state, "Wastes", &[]);
        let bear = permanent(&mut state, me(), "Bear");

        assert_eq!(intrinsic_mana(&state, plains), Some(ManaColor::White));
        assert_eq!(
            intrinsic_mana(&state, shrine),
            None,
            "the shortcut does not pick for the player"
        );
        assert_eq!(intrinsic_mana(&state, waste), None, "no basic type");
        assert_eq!(intrinsic_mana(&state, bear), None, "not a land at all");

        assert_eq!(
            intrinsic_mana_colors(&state, plains),
            vec![ManaColor::White]
        );
        assert_eq!(
            intrinsic_mana_colors(&state, shrine),
            vec![ManaColor::White, ManaColor::Black],
            "both abilities the dual has, in the rules' own order"
        );
        assert!(
            intrinsic_mana_colors(&state, waste).is_empty(),
            "no basic type is no ability at all — Wastes prints its own"
        );
        assert!(intrinsic_mana_colors(&state, bear).is_empty());
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
