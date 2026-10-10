//! Building per-seat views (CR 400.2) from engine state.
//!
//! The wire types live in [`baylee_view`] so that clients do not have to link
//! the rules kernel. This module is the only place that translates engine
//! state into them, and it is where the hidden-information rules are enforced:
//! a seat sees public zones in full, its own hand, and only counts for hidden
//! zones belonging to anyone else.
//!
//! Characteristics are taken from the engine's **projected** values, not from
//! the printed card. A client cannot run the layer system, so an anthem, a
//! clone, or an animated land has to arrive already resolved.

use baylee_core::ids::{ObjectId, PlayerId, SeatSet};
use baylee_core::mana::{ManaCost, ManaPayment};
use baylee_engine::choice::Pending;
use baylee_engine::event::LossReason;
use baylee_engine::object::{GameObject, ObjectKind};
use baylee_engine::state::GameState;
use baylee_engine::turn::{DayNight as EngineDayNight, Phase as EnginePhase, Step as EngineStep};
use baylee_engine::zone::{Zone, ZoneLocation};
use baylee_view::{
    AttackerView, BlockerView, CardIdentity, CombatView, CommanderDamage, CommanderView,
    CounterKind, DayNight, GameStatic, HandObject, HouseAnswer, LossCause, Phase, PlayerView,
    PolicyAct, SeatView, Step,
};

pub use baylee_view as wire;

mod object;
mod stack;
mod zones;

use object::{exact_object_view_with_access, public_object, target_objects};
use stack::rules_face;
pub(crate) use stack::stack_item;
use zones::{looking_at, mana_pool, per_seat_zone, zone};

/// Translates the engine's phase into the wire enum.
const fn phase(p: EnginePhase) -> Phase {
    match p {
        EnginePhase::Beginning => Phase::Beginning,
        EnginePhase::FirstMain => Phase::FirstMain,
        EnginePhase::Combat => Phase::Combat,
        EnginePhase::SecondMain => Phase::SecondMain,
        EnginePhase::Ending => Phase::Ending,
    }
}

/// Translates the engine's reason a seat lost into the wire enum.
pub(crate) const fn loss_cause(reason: LossReason) -> LossCause {
    match reason {
        LossReason::Life => LossCause::Life,
        LossReason::EmptyDraw => LossCause::EmptyDraw,
        LossReason::Poison => LossCause::Poison,
        LossReason::CommanderDamage => LossCause::CommanderDamage,
        LossReason::Conceded => LossCause::Conceded,
        LossReason::Effect => LossCause::Effect,
    }
}

/// Translates the engine's day/night designation into the wire enum.
pub(crate) const fn day_night(d: EngineDayNight) -> DayNight {
    match d {
        EngineDayNight::Day => DayNight::Day,
        EngineDayNight::Night => DayNight::Night,
    }
}

/// Translates the engine's step into the wire enum.
const fn step(s: EngineStep) -> Step {
    match s {
        EngineStep::Untap => Step::Untap,
        EngineStep::Upkeep => Step::Upkeep,
        EngineStep::Draw => Step::Draw,
        EngineStep::Main => Step::Main,
        EngineStep::CombatBegin => Step::CombatBegin,
        EngineStep::DeclareAttackers => Step::DeclareAttackers,
        EngineStep::DeclareBlockers => Step::DeclareBlockers,
        EngineStep::CombatDamageFirst => Step::CombatDamageFirst,
        EngineStep::CombatDamage => Step::CombatDamage,
        EngineStep::CombatEnd => Step::CombatEnd,
        EngineStep::End => Step::End,
        EngineStep::Cleanup => Step::Cleanup,
    }
}

/// Translates a counter kind into the wire enum.
pub(crate) const fn counter(kind: baylee_cards_dsl::CounterKind) -> CounterKind {
    use baylee_cards_dsl::CounterKind as K;
    match kind {
        K::Plus { power, toughness } => CounterKind::Plus { power, toughness },
        K::Minus { power, toughness } => CounterKind::Minus { power, toughness },
        K::Loyalty => CounterKind::Loyalty,
        K::Lore => CounterKind::Lore,
        K::Time => CounterKind::Time,
        K::Charge => CounterKind::Charge,
        K::Poison => CounterKind::Poison,
        K::Energy => CounterKind::Energy,
        K::Rad => CounterKind::Rad,
        K::Lifelink => CounterKind::Lifelink,
        K::Level => CounterKind::Level,
        K::Custom(id) => CounterKind::Custom(id as u32),
    }
}

/// Whether `seat` is entitled to know what card backs `obj`.
///
/// Face-down permanents are the one case where two seats looking at the same
/// battlefield legitimately see different things (CR 708.5): the controller
/// knows what they played, everyone else sees a blank. Returning `None` for
/// the card identity — rather than sending it and trusting the client to hide
/// it — is what makes the leak unrepresentable.
pub(crate) fn may_know_card(obj: &GameObject, seat: PlayerId) -> bool {
    !obj.status
        .contains(baylee_engine::object::Status::FACE_DOWN)
        || obj.controller == seat
}

/// Whether `id` is one of the table's commanders (CR 903.3).
fn is_commander(state: &GameState, id: ObjectId) -> bool {
    state.commanders.iter().flatten().any(|c| c.object == id)
}

/// One of a seat's commanders, as the whole table may see it.
///
/// The identity is not filtered per seat the way a battlefield object's is:
/// a commander is designated openly before the first turn (CR 903.3), so
/// every seat has already seen this card in the command zone and knowing it
/// again — in a hand, after declining CR 903.9b — reveals nothing new.
fn commander_view(state: &GameState, c: &baylee_engine::state::Commander) -> CommanderView {
    let obj = state.object(c.object);
    CommanderView {
        object: c.object,
        card: obj.and_then(|o| {
            o.card.map(|card| CardIdentity {
                index: card.index,
                print: card.print,
                face: o.face_index,
            })
        }),
        name: obj.map_or_else(String::new, |o| {
            state.names.get(o.characteristics().name).to_string()
        }),
        casts: c.casts,
    }
}

/// The public name of an object as `seat` may know it.
fn public_name(state: &GameState, obj: &GameObject, seat: PlayerId) -> String {
    if may_know_card(obj, seat) {
        state.names.get(obj.characteristics().name).to_string()
    } else {
        "Face-down".to_string()
    }
}

/// A planeswalker's loyalty as it stands, not as it is printed.
///
/// `Characteristics::loyalty` is the number on the card and never moves;
/// CR 306.5c says the loyalty of a planeswalker on the battlefield is the
/// number of loyalty counters on it, which is what CR 306.5b puts there as it
/// enters and what the engine's own state-based check reads when it puts one
/// at zero into a graveyard. Sending the printed number meant a client drew a
/// walker at its starting loyalty for the whole game: it ticked up, was
/// attacked down and died, and the plate never moved.
///
/// Off the battlefield the printed number is the right answer and there are no
/// counters to read, so the object's kind decides. A face that prints no
/// loyalty stays `None` either way — the field says "this is a planeswalker's
/// plate", and a permanent carrying loyalty counters without being one is
/// `CounterEntry` business, not this.
fn loyalty_now(obj: &GameObject, printed: Option<u16>) -> Option<u16> {
    let printed = printed?;
    if obj.kind == ObjectKind::Permanent {
        Some(obj.counters.get(baylee_cards_dsl::CounterKind::Loyalty))
    } else {
        Some(printed)
    }
}

/// What `seat` would pay to cast `obj` from its own graveyard, if it may:
/// [`PublicObject::flashback`].
///
/// The same facts `casting::can_cast` asks before it lets the card off the
/// graveyard, so the view never says "castable" of a card the engine would
/// refuse for being somewhere else. The price is what the cast charges: a
/// printed flashback's own cost (CR 702.34a), or for a granted one the
/// card's mana cost. A permanent card that Muldrotha or Wrenn's emblem lets
/// the seat cast from there is priced at its own mana cost too. Escape last
/// (CR 702.138a): its mana, once the graveyard holds the other cards it
/// exiles, counted off the list the cast wizard asks from.
fn graveyard_price(
    state: &GameState,
    id: ObjectId,
    obj: &GameObject,
    seat: PlayerId,
    mana_cost: ManaCost,
) -> Option<ManaCost> {
    if obj.zone != Zone::Graveyard || obj.zone_owner != Some(seat) {
        return None;
    }
    let face = obj
        .card
        .and_then(|c| baylee_cards::by_index(c.index))
        .map(|def| &def.faces[0]);
    face.and_then(|face| face.flashback)
        .or_else(|| {
            (baylee_engine::casting::flashback_granted(state, id)
                || baylee_engine::casting::graveyard_cast_permission(state, seat, obj).is_some())
            .then_some(mana_cost)
        })
        .or_else(|| {
            face.and_then(|face| face.escape)
                .filter(|escape| {
                    baylee_engine::casting::escape_exile_options(state, seat, id).len()
                        >= usize::from(escape.exile)
                })
                .map(|escape| escape.cost)
        })
}

/// Projects one object into its public form for `seat`.
fn damage_sources(
    state: &GameState,
    seat: PlayerId,
    pending: Option<&Pending>,
    ctx: &SeatContext,
) -> Vec<baylee_view::DamageSourceView> {
    let Some(Pending::ChooseDamageSource {
        player, options, ..
    }) = pending
    else {
        return Vec::new();
    };
    if *player != seat && ctx.awaiting != Some(seat) {
        return Vec::new();
    }
    options
        .iter()
        .filter_map(|&source| {
            exact_object_view_with_access(state, seat, source, ctx.controlled_players)
        })
        .collect()
}

/// The payment a CR 605.3a window is open for, in the shape the view carries.
///
/// One conversion, in one place, for the same reason `granted_activated` is
/// one lookup: an offer and a projection that disagreed about what a seat
/// owes would be a window the player is told to use and a price the engine
/// does not charge. Every caller of [`player_view`] that has an `Engine` in
/// hand passes this.
///
/// Includes colored debts such as Pact of Negation's {3}{U}{U}; the client
/// must plan for the actual symbols, not just the total amount.
#[must_use]
pub fn owed_payment<L: baylee_engine::state::CardLookup>(
    engine: &baylee_engine::engine::Engine<L>,
) -> Option<ManaPayment> {
    engine.payment_window().map(|(_, cost)| cost)
}

/// The seats still deciding their opening mulligan, for
/// [`SeatContext::deciding`]: every seat the engine is waiting on whose own
/// question is a `Mulligan` or a `MulliganBottom`.
///
/// Neither question exists outside the window, so this is empty from turn 1
/// on and doubles as the answer to whether the window is open.
#[must_use]
pub fn deciding<L: baylee_engine::state::CardLookup>(
    engine: &baylee_engine::engine::Engine<L>,
) -> SeatSet {
    engine
        .awaited()
        .iter()
        .filter(|seat| {
            matches!(
                engine.pending_for(*seat),
                Some(Pending::Mulligan { .. } | Pending::MulliganBottom { .. })
            )
        })
        .collect()
}

/// Who `seat`'s view says the table is waiting for, for
/// [`SeatContext::awaiting`].
///
/// From turn 1 on, the one seat [`Engine::pending`] is addressed to, the same
/// in every view. During the opening mulligans every seat is asked at once,
/// so it is `seat` itself while it is still deciding and nobody once it has
/// kept: each view names the one question its own seat can answer.
///
/// [`Engine::pending`]: baylee_engine::engine::Engine::pending
#[must_use]
pub fn awaiting_for<L: baylee_engine::state::CardLookup>(
    engine: &baylee_engine::engine::Engine<L>,
    seat: PlayerId,
) -> Option<PlayerId> {
    let deciding = deciding(engine);
    if deciding.is_empty() {
        engine.decision_actor()
    } else {
        deciding.contains(seat).then_some(seat)
    }
}

/// Resource owner of this view's question. Simultaneous opening decisions
/// belong to each asked seat; afterwards the one global question may be
/// answered by a different player controlling its resource owner.
#[must_use]
pub fn decision_player_for<L: baylee_engine::state::CardLookup>(
    engine: &baylee_engine::engine::Engine<L>,
    seat: PlayerId,
) -> Option<PlayerId> {
    if deciding(engine).is_empty() {
        engine.pending().asked()
    } else {
        engine.pending_for(seat).and_then(Pending::asked)
    }
}

/// What a per-seat view needs that the [`GameState`] cannot supply.
///
/// Four facts live on the `Engine` and not in the state it hands out — who
/// the table is waiting for, who is still deciding a mulligan, whether this
/// seat's own standing order is withholding its priority, and what it owes
/// inside a payment window — so each of them has to be carried across. The
/// first three travelled as positional arguments until the fourth was
/// proposed, at which point `player_view` would have taken eight and stopped
/// compiling: clippy's `too_many_arguments` allows seven, and this workspace
/// builds with `-D warnings`.
///
/// A struct rather than an `#[allow]`, because the argument list had a
/// failure the limit is only a proxy for. Every call site passes these
/// positionally, and two `Option`s of different types can be swapped in
/// silence by a rebase; a field is set by **name** and cannot be. That is a
/// type where the convention was.
///
/// [`Default`] is derived for the tests, which are most of the call sites and
/// genuinely do not care about any of this. **Production callers build it
/// exhaustively** and must keep doing so: a `..Default::default()` tail turns
/// the next field added here into a silent `None` at every site carrying it,
/// compiling everywhere and read nowhere. `session.rs` and `harness.rs` are
/// the two that should go red when the next field arrives.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SeatContext<'a> {
    /// The resource owner of the current decision, even when another player answers.
    pub decision_player: Option<PlayerId>,
    /// Other players whose private information this viewer may inspect (CR 720.4).
    pub controlled_players: SeatSet,
    /// Libraries whose changed top must stay hidden until announcement ends (CR 401.5).
    pub library_reveal_blocked: SeatSet,
    /// The seat the table is waiting for, as this seat's view tells it. Pass
    /// [`awaiting_for`], which is per seat during the opening mulligans.
    pub awaiting: Option<PlayerId>,
    /// The seats still deciding their opening mulligan. Pass [`deciding`].
    pub deciding: SeatSet,
    /// Whether *this* seat's standing order is currently withholding its own
    /// priority. Pass `engine.automation(seat).hold.suppresses()`.
    pub held: bool,
    /// What the awaited seat owes inside a CR 605.3a payment window. Pass
    /// [`owed_payment`].
    pub owed: Option<ManaPayment>,
    /// How long the awaited seat has left to answer, in milliseconds. Pass
    /// [`Session::decision_remaining_ms`](crate::Session::decision_remaining_ms).
    ///
    /// The odd one out in this struct, and deliberately so. Its neighbours
    /// are read off the `Engine` by whoever builds the context;
    /// this one cannot be, because **this crate is forbidden a wall clock** —
    /// a session that timed itself would replay differently on every machine.
    /// So it is measured outside and handed in.
    ///
    /// That also makes it the one field here that must not reach a rules
    /// decision. A view sent to a socket carries it; a view built for an
    /// agent to answer from leaves it `None`, because elapsed machine time is
    /// not an authorized input to a decision and an agent that read it would
    /// play the same position differently on a slow machine. Same invariant
    /// as #87 and the same reason.
    pub decision_remaining_ms: Option<u32>,
    /// Per seat, in seat order, what its own per-ability policies answered
    /// for it since it last answered by hand (#234). Pass the host's window
    /// ([`Session`](crate::Session) keeps it), or `&[]`.
    ///
    /// The whole table rather than this seat's row, and the second odd one
    /// out: like `house_answered`, only the host knows it. The view picks the
    /// seat's own row, so no other seat's reaches it
    /// ([`PlayerView::policy_acts`]).
    pub policy_acts: &'a [Vec<PolicyAct>],
}

/// This seat's own hand, which is the one hand a view spells out.
///
/// Lifted out of [`player_view`] only for its length; it is the same walk it
/// always was. Every *other* seat's hand is a count, and that asymmetry is
/// the point — see the crate docs on hidden information.
pub(crate) fn own_hand(state: &GameState, seat: PlayerId) -> Vec<HandObject> {
    state
        .zones
        .list(ZoneLocation::Hand(seat))
        .iter()
        .filter_map(|id| {
            let obj = state.object(*id)?;
            let card = obj.card?;
            let chars = obj.characteristics();
            Some(HandObject {
                id: *id,
                card: CardIdentity {
                    index: card.index,
                    print: card.print,
                    face: obj.face_index,
                },
                name: state.names.get(chars.name).to_string(),
                mana_value: chars.mana_value(),
                colors: chars.colors,
                types: chars.types,
                commander: is_commander(state, *id),
            })
        })
        .collect()
}

/// The combat in progress, public to every seat: who attacks whom, who
/// blocks what, and which attackers are banded (CR 702.22c).
fn combat_view(state: &GameState) -> CombatView {
    CombatView {
        attackers: state
            .combat
            .attackers()
            .iter()
            .map(|a| AttackerView {
                creature: a.creature,
                defending: a.defending,
                blocked: a.blocked,
            })
            .collect(),
        blockers: state
            .combat
            .blockers
            .iter()
            .map(|b| BlockerView {
                blocker: b.blocker,
                attacker: b.attacker,
            })
            .collect(),
        bands: state.combat.bands(),
    }
}

/// Builds the hidden-information-filtered view of `state` for `seat`.
///
/// `ctx` carries the facts that are on the `Engine` rather than in the
/// state; see [`SeatContext`].
///
/// `house_answered` is per seat, in seat order: who answered that seat's most
/// recent decision in its place ([`SeatView::house_answered`]). It is neither
/// state nor on the `Engine`; only the host knows who produced an answer, so
/// a host passes what it recorded and anything else passes `&[]`. A seat the
/// slice does not reach reads `None`.
///
/// `pending` is the outstanding choice, and it is here for one reason:
/// [`PlayerView::looking_at`]. A tutor, a scry and a revealed hand all ask a
/// seat about objects that are in no zone the view carries, so the choice
/// itself is what decides which hidden objects this seat may see. Pass `None`
/// and the view is exactly what it was before — nothing else reads it.
#[must_use]
pub fn player_view(
    state: &GameState,
    seat: PlayerId,
    seq: u64,
    pending: Option<&Pending>,
    ctx: &SeatContext,
    house_answered: &[Option<HouseAnswer>],
) -> PlayerView {
    PlayerView {
        decision_player: ctx.decision_player,
        controlled_hands: ctx
            .controlled_players
            .iter()
            .map(|player| baylee_view::SharedHand {
                player,
                cards: own_hand(state, player),
            })
            .collect(),
        seq,
        seat,
        turn: state.turn.number,
        phase: phase(state.turn.phase),
        step: step(state.turn.step),
        active: state.turn.active,
        awaiting: ctx.awaiting,
        deciding: ctx.deciding,
        decision_remaining_ms: ctx.decision_remaining_ms,
        clocks: Vec::new(),
        lost: Vec::new(),
        priority_held: ctx.held,
        policy_acts: ctx
            .policy_acts
            .get(seat.get() as usize)
            .cloned()
            .unwrap_or_default(),
        owed: ctx.owed,
        monarch: state.monarch,
        day_night: state.day_night.map(day_night),
        seats: seat_views(state, house_answered),
        hand: own_hand(state, seat),
        // Empty here, in every view, and filled on the way to a socket
        // (`Session::show_hands`): a view an agent answers from never
        // passes that way, so no agent is ever handed a teammate's hand.
        shared_hands: Vec::new(),
        hand_shared_with: SeatSet::new(),
        hand_requests: SeatSet::new(),
        hand_requested: SeatSet::new(),
        battlefield: zone(
            state,
            ZoneLocation::Battlefield,
            seat,
            ctx.controlled_players,
        ),
        stack: zone(state, ZoneLocation::Stack, seat, ctx.controlled_players),
        graveyards: per_seat_zone(state, ZoneLocation::Graveyard, seat, ctx.controlled_players),
        exile: per_seat_zone(state, ZoneLocation::Exile, seat, ctx.controlled_players),
        command: per_seat_zone(state, ZoneLocation::Command, seat, ctx.controlled_players),
        combat: combat_view(state),
        looking_at: looking_at(state, seat, pending, ctx),
        damage_sources: damage_sources(state, seat, pending, ctx),
        target_objects: target_objects(state, seat, pending, ctx),
        library_tops: state
            .players
            .iter()
            .filter(|p| {
                state.library_top_revealed(p.id) && !ctx.library_reveal_blocked.contains(p.id)
            })
            .filter_map(|p| state.zones.list(ZoneLocation::Library(p.id)).last())
            .filter_map(|id| public_object(state, *id, seat))
            .collect(),
        // The same question `casting::timing_allows` asks before it refuses a
        // spell, asked once per view so the seat can be told before it spends
        // anything. It is the effect's *source* that travels, not a flag: the
        // client owes the player the card to point at.
        targeting: None,
        casting: None,
        sorcery_lock: state
            .effects
            .iter()
            .find(|fx| {
                matches!(
                    fx.modifier,
                    baylee_cards_dsl::Modifier::OpponentsCastAsSorcery
                ) && state.is_opponent(fx.controller, seat)
            })
            .and_then(|fx| fx.source),
        sorceries_have_flash: sorceries_have_flash(state, seat),
    }
}

/// Every seat's public numbers, alike in every view and the spectator's.
fn seat_views(state: &GameState, house_answered: &[Option<HouseAnswer>]) -> Vec<SeatView> {
    state
        .players
        .iter()
        .map(|p| SeatView {
            player: p.id,
            life: p.life,
            poison: p.poison,
            energy: p.energy,
            hand_count: state.zones.list(ZoneLocation::Hand(p.id)).len() as u32,
            no_max_hand_size: state.no_max_hand_size(p.id),
            library_count: state.zones.list(ZoneLocation::Library(p.id)).len() as u32,
            graveyard_count: state.zones.list(ZoneLocation::Graveyard(p.id)).len() as u32,
            loss: p.loss.map(loss_cause),
            house_answered: house_answered.get(p.id.get() as usize).copied().flatten(),
            mana_pool: mana_pool(state, p.id),
            commanders: state
                .commanders
                .get(p.id.get() as usize)
                .map_or_else(Vec::new, |list| {
                    list.iter().map(|c| commander_view(state, c)).collect()
                }),
            commander_damage: p
                .commander_damage
                .iter()
                .map(|&(source, amount)| CommanderDamage { source, amount })
                .collect(),
        })
        .collect()
}

/// The viewer a spectator's objects are built for: no seat at all.
///
/// Above [`SeatSet::MAX_SEAT`], so no [`SeatSet`] holds it and no player's
/// id equals it (`spectator_is_no_seat`). It enters exactly one place,
/// [`spectator_view`]'s zones, where a face-down object is known only to a
/// viewer that controls it; nobody controls anything as this id.
pub const SPECTATOR: PlayerId = PlayerId::new(u8::MAX);

/// The table as a spectator sees it (`docs/protocol.md` §"Spectators").
///
/// Its own builder rather than [`player_view`] for some seat: the type has
/// no field a hand, a question or a looked-at card could travel in, and
/// nothing here asks which seat is looking except the public-object walk,
/// which is asked for [`SPECTATOR`], a viewer who controls nothing and is in
/// no set. `awaiting` is the seat every other seat is told the table waits
/// for, and `library_reveal_blocked` holds a top back as it does for seats.
#[must_use]
pub fn spectator_view(
    state: &GameState,
    seq: u64,
    awaiting: Option<PlayerId>,
    deciding: SeatSet,
    library_reveal_blocked: SeatSet,
    house_answered: &[Option<HouseAnswer>],
) -> baylee_view::SpectatorView {
    let nobody = SeatSet::new();
    baylee_view::SpectatorView {
        seq,
        turn: state.turn.number,
        phase: phase(state.turn.phase),
        step: step(state.turn.step),
        active: state.turn.active,
        awaiting,
        deciding,
        monarch: state.monarch,
        day_night: state.day_night.map(day_night),
        seats: seat_views(state, house_answered),
        battlefield: zone(state, ZoneLocation::Battlefield, SPECTATOR, nobody),
        stack: zone(state, ZoneLocation::Stack, SPECTATOR, nobody),
        graveyards: per_seat_zone(state, ZoneLocation::Graveyard, SPECTATOR, nobody),
        exile: per_seat_zone(state, ZoneLocation::Exile, SPECTATOR, nobody),
        command: per_seat_zone(state, ZoneLocation::Command, SPECTATOR, nobody),
        combat: combat_view(state),
        library_tops: state
            .players
            .iter()
            .filter(|p| state.library_top_revealed(p.id) && !library_reveal_blocked.contains(p.id))
            .filter_map(|p| state.zones.list(ZoneLocation::Library(p.id)).last())
            .filter_map(|id| public_object(state, *id, SPECTATOR))
            .collect(),
    }
}

/// Teferi, Time Raveler's +1 for `seat`: its sorceries may be cast as though
/// they had flash. The same reading `casting::timing_allows` makes, an
/// effect the seat controls; an opponent's +1 is not this seat's.
fn sorceries_have_flash(state: &GameState, seat: PlayerId) -> bool {
    state.effects.iter().any(|fx| {
        matches!(fx.modifier, baylee_cards_dsl::Modifier::SorceriesHaveFlash)
            && fx.controller == seat
    })
}

/// Reads the house rules' spelling of *no limit* into the view's.
///
/// `HouseRules` says it with zero and the view says it with `None`, and the
/// translation belongs here rather than at each reader: zero drawn on a seat
/// sheet is a table with no time at all, which is the opposite of what it
/// means.
const fn no_limit_is_none(secs: u32) -> Option<u32> {
    if secs == 0 { None } else { Some(secs) }
}

/// Builds the once-per-game static payload a client needs before it can render
/// anything: who sits where, and the print table its images are keyed by.
///
/// `shown` decides, entry by entry, whether this seat has earned the printing.
/// The table is shared by the whole game and deduplicated per card, so a seat
/// handed all of it would be handed the union of every deck at the table. An
/// entry the seat has not earned is `None` rather than absent: the index *is*
/// the [`PrintRef`](baylee_core::ids::PrintRef), and renumbering it would
/// change what every object in every view points at.
#[must_use]
pub fn game_static(
    game_id: String,
    your_seat: PlayerId,
    seats: Vec<baylee_view::SeatIdentity>,
    prints: &[baylee_core::preset::PrintInfo],
    shown: &[bool],
    house_rules: &baylee_core::preset::HouseRules,
) -> GameStatic {
    GameStatic {
        view_version: baylee_view::VIEW_VERSION,
        game_id,
        your_seat,
        seats,
        decision_secs: no_limit_is_none(house_rules.decision_timeout_secs),
        reconnect_secs: no_limit_is_none(house_rules.reconnect_window_secs),
        prints: prints
            .iter()
            .enumerate()
            .map(|(i, p)| {
                shown
                    .get(i)
                    .copied()
                    .unwrap_or(false)
                    .then(|| baylee_view::PrintEntry {
                        scryfall_id: p.scryfall_id.to_string(),
                        lang: p.lang.clone(),
                        finish: match p.finish {
                            baylee_core::preset::Finish::Foil => baylee_view::Finish::Foil,
                            baylee_core::preset::Finish::Etched => baylee_view::Finish::Etched,
                            baylee_core::preset::Finish::Holographic => {
                                baylee_view::Finish::Holographic
                            }
                            baylee_core::preset::Finish::Glitter => baylee_view::Finish::Glitter,
                            baylee_core::preset::Finish::Galaxy => baylee_view::Finish::Galaxy,
                            baylee_core::preset::Finish::Normal => baylee_view::Finish::Normal,
                        },
                    })
            })
            .collect(),
    }
}

/// Explains only the current chooser's decision; never another seat's hand.
pub(crate) fn targeting_context<L: baylee_engine::state::CardLookup>(
    engine: &baylee_engine::engine::Engine<L>,
    seat: PlayerId,
) -> Option<baylee_view::TargetingContext> {
    if engine.decision_actor() != Some(seat)
        || !matches!(engine.pending(), Pending::ChooseTargets { .. })
    {
        return None;
    }
    let context = engine.decision_context();
    let id = context.source?;
    let mut source = public_object(engine.state(), id, seat)?;
    if let Some(printed) = context.printed {
        source.rules = Some(rules_face(printed));
    }
    if let Some(token) = context
        .provenance
        .and_then(|origin| origin.origin)
        .and_then(baylee_engine::object::AbilityOrigin::token)
    {
        source.rules = None;
        source.token = Some(token.get() - 1);
    }
    let text = source.rules.and_then(|rules| {
        let line = if let Some(mode) = context.mode {
            baylee_cards::lines::mode_line(rules.card, usize::from(rules.face), mode)
        } else {
            baylee_cards::lines::ability_line(
                rules.card,
                usize::from(rules.face),
                context.ability_index?,
            )
        }?;
        Some(baylee_view::StackText {
            face: rules.face,
            line: line.line,
            of: line.of,
        })
    });
    Some(baylee_view::TargetingContext {
        source,
        text,
        whole_spell: context.whole_spell,
        second: context.second_instance,
        batch_count: engine.target_batch_count(),
    })
}

#[cfg(test)]
mod tests;
