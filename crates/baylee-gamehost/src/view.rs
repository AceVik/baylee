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

use baylee_ai::pending_player;
use baylee_cards::dsl::AbilityDef;
use baylee_core::ids::{ObjectId, PlayerId};
use baylee_core::mana::ManaCost;
use baylee_engine::choice::Pending;
use baylee_engine::object::{GameObject, ObjectKind};
use baylee_engine::state::GameState;
use baylee_engine::turn::{DayNight as EngineDayNight, Phase as EnginePhase, Step as EngineStep};
use baylee_engine::zone::{Zone, ZoneLocation};
use baylee_view::{
    AttackerView, BlockerView, CardIdentity, CombatView, CommanderDamage, CommanderView,
    CounterEntry, CounterKind, DayNight, GameStatic, HandObject, ObjectStatus, Phase, PlayerView,
    PublicObject, SeatView, Step, TargetRef,
};

pub use baylee_view as wire;

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

/// Translates the engine's day/night designation into the wire enum.
const fn day_night(d: EngineDayNight) -> DayNight {
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
const fn counter(kind: baylee_cards_dsl::CounterKind) -> CounterKind {
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
fn may_know_card(obj: &GameObject, seat: PlayerId) -> bool {
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

/// Projects one object into its public form for `seat`.
fn public_object(state: &GameState, id: ObjectId, seat: PlayerId) -> Option<PublicObject> {
    let obj = state.object(id)?;
    let chars = obj.characteristics();
    let known = may_know_card(obj, seat);
    Some(PublicObject {
        id,
        card: obj.card.filter(|_| known).map(|c| CardIdentity {
            index: c.index,
            print: c.print,
            face: obj.face_index,
        }),
        name: public_name(state, obj, seat),
        controller: obj.controller,
        owner: obj.owner,
        // Deliberately *not* gated on `known`, unlike the card above: a
        // commander is designated openly (CR 903.3), and `commander_view`
        // hands its identity to the whole table for the same reason. Two
        // structs disagreeing about one fact would be worse than either
        // answer.
        //
        // Manifest is the case that will break this, and gating here would
        // not have fixed it: a commander manifested off a library is a card
        // nobody announced, and `CommanderView::object` names its handle, so
        // blanking one field still leaves it identifiable by cross-reference
        // against the face-down permanent. That needs the handle hidden too,
        // and nothing sets `FACE_DOWN` yet.
        commander: is_commander(state, id),
        status: ObjectStatus::from_bits(obj.status.bits()),
        types: chars.types,
        supertypes: chars.supertypes,
        subtypes: chars.subtypes,
        // The engine holds the definition; the client needs the number, and
        // this crate is the one that can see both.
        token: obj.token.map(baylee_cards::tokens::token_id),
        colors: chars.colors,
        keywords: chars.keywords.bits(),
        power: chars.power,
        toughness: chars.toughness,
        // The base, straight off the object rather than out of the plan:
        // `layers::recompute` starts every projection from exactly this and
        // there is nowhere else the printed body survives. It is the copied
        // card's for a permanent that became a copy, which is what CR 706.2
        // makes true of the object and is the answer a player wants.
        base_power: obj.base.power,
        base_toughness: obj.base.toughness,
        loyalty: loyalty_now(obj, chars.loyalty),
        mana_value: chars.mana_cost.cmc(),
        damage: obj.damage,
        counters: obj
            .counters
            .iter()
            .map(|(kind, count)| CounterEntry {
                kind: counter(kind),
                count,
            })
            .collect(),
        attached_to: obj.attached_to,
        // `TargetRef` has had a `Player` arm since the view was written and
        // never carried one, because the engine's target list was objects
        // only. A burn spell aimed at a face now says so on the stack.
        targets: obj
            .targets
            .iter()
            .map(|t| TargetRef::Object(*t))
            .chain(obj.target_players.iter().map(TargetRef::Player))
            .collect(),
        stack_item: stack_item(obj),
        // Permanents only, because the engine's answer is about a creature
        // *on the battlefield* and a creature card in hand would otherwise
        // come back asleep. The creature test itself is the engine's — one
        // reading of CR 302.6, shared with the attack legality the client
        // is offered, so the drawing and the offer cannot disagree.
        summoning_sick: obj.kind == ObjectKind::Permanent
            && baylee_engine::combat::summoning_sick(state, obj),
        // Permanents only, for the same reason as `summoning_sick`: nothing
        // else can be tapped for it. The engine's own offer reads the grant
        // through the same function, so what the planner is told a land makes
        // is what the engine will hand out when it is tapped.
        granted_mana: (obj.kind == ObjectKind::Permanent)
            .then(|| granted_mana(state, id))
            .flatten(),
        // Permanents only, for the reason the two fields above are: nothing
        // else can be tapped. Not gated on `known` either, and it does not
        // need to be: the abilities are read through the object's own
        // *projected* list, so a permanent that has lost its abilities — the
        // shape anything face-down will have to take — offers none to read.
        board_mana: (obj.kind == ObjectKind::Permanent)
            .then(|| board_mana(state, id))
            .flatten(),
    })
}

/// The mana a granted ability lets `id` make, when it is one a client's
/// planner can use.
///
/// This crate is the one that can see both halves — the engine's effect table
/// and the DSL that says what an `AddMana` produces — which is the same reason
/// `token` is resolved here. It is not hidden information: the grant comes
/// from a permanent on the battlefield and the ability is already offered in
/// `LegalActions` to whoever may activate it.
/// The **first** grant that is a mana ability, which is the one a planner can
/// use: a permanent may be granted several (Urza's Saga is granted two), and
/// the plan taps it once either way.
fn granted_mana(state: &GameState, id: ObjectId) -> Option<baylee_view::GrantedMana> {
    baylee_engine::effects::granted_activated(state, id)
        .take(baylee_engine::choice::GRANTED_SLOTS as usize)
        .enumerate()
        .filter(|(_, g)| g.mana_ability)
        .find_map(|(slot, g)| {
            let mana = baylee_cards_dsl::simple_mana(&g.cost, g.effects)?;
            Some(baylee_view::GrantedMana {
                slot: u32::try_from(slot).unwrap_or(u32::MAX),
                colors: mana.colors,
                amount: mana.amount,
            })
        })
}

/// Which colours a printed mana ability of `id` makes, when the printing does
/// not say.
///
/// The twin of [`granted_mana`] one row along, and the same division of
/// labour: that one covers an ability that is on no card, this one an ability
/// that is on the card and still cannot be read off it. Reflecting Pool,
/// Exotic Orchard and Fellwar Stone read the union of `produced_colors` over
/// the lands of one side of the table; Command Tower, Arcane Signet,
/// Commander's Sphere and Path of Ancestry read a seat's commander identity.
/// This crate is again the one that can see both halves — `mana_shape` says
/// which question the card is asking, `resolve::colors_of` is the engine's
/// own answer to it, and it is the very function that hands the mana out when
/// the permanent is tapped, so what the planner is told cannot drift from
/// what it gets.
///
/// Not hidden information: the permanent is on the battlefield, every land
/// the union reads is too, and a commander is designated openly (CR 903.3).
///
/// The **first** printed ability that needs a board and has an answer, which
/// is the bargain [`granted_mana`] makes for the same reason: a permanent is
/// tapped once, so one is all a plan can spend. No card in the pool prints
/// two, and a permanent that did would have its second refused rather than
/// guessed at — a refusal costs one tap by hand, a wrong answer strands a
/// mana run with the permanent already tapped.
///
/// An ability that currently makes **nothing** is skipped, and if it was the
/// only one the answer is `None`. A Reflecting Pool contributes no colour to
/// the union it reads (CR 106.7, through `Characteristics::produced_colors`),
/// so a lone Pool taps for no colour at all — and "no colours" and "no such
/// ability" are the same answer to every caller: there is no tap here to plan
/// with.
fn board_mana(state: &GameState, id: ObjectId) -> Option<baylee_view::BoardMana> {
    let obj = state.object(id)?;
    // `abilities` and not `by_index`: it is the one accessor that answers for
    // an emblem, a token and a copy, and — the case that matters here — for a
    // permanent whose printed list a continuous effect has replaced.
    obj.abilities(&crate::session::RegistryLookup)
        .iter()
        .enumerate()
        .find_map(|(index, ability)| {
            // `ActivatedConditional` beside `Activated`, because a mana
            // ability with a condition on it is still a mana ability and
            // reading only the first variant is the mistake six readers
            // across this workspace made before it had a name.
            let (AbilityDef::Activated {
                cost,
                effects,
                mana_ability: true,
                ..
            }
            | AbilityDef::ActivatedConditional {
                cost,
                effects,
                mana_ability: true,
                ..
            }) = ability
            else {
                return None;
            };
            let (source, _, _) = baylee_cards_dsl::mana_shape(cost, effects)?;
            // Everything the card can answer alone stays the card's: a Forest
            // and a Birds of Paradise have nothing to gain from a projection
            // and would cost the wire a colour list each.
            if !matches!(
                source,
                baylee_cards_dsl::ManaSource::CommanderIdentity
                    | baylee_cards_dsl::ManaSource::LandColor { .. }
                    | baylee_cards_dsl::ManaSource::Chosen
                    | baylee_cards_dsl::ManaSource::ChosenOr(_)
            ) {
                return None;
            }
            // The chosen colour belongs here for the same reason the other
            // two do, and for one more: it is the only one a *client* could
            // not derive even with the whole board in front of it, because
            // the choice is on the object and is printed on no card.
            let colors = baylee_engine::resolve::colors_of(state, obj.controller, source, id);
            (!colors.is_empty()).then(|| baylee_view::BoardMana {
                index: u32::try_from(index).unwrap_or(u32::MAX),
                colors,
            })
        })
}

/// What a stack object is, for objects that are on the stack.
///
/// An ability on the stack is its own object with no card of its own, so
/// without this a client can only draw an anonymous entry — it knows a
/// trigger is resolving, but not whose, and not which of that permanent's
/// abilities it is. The engine already tracks exactly that in
/// `AbilityLoc`; this is where it reaches the client.
fn stack_item(obj: &GameObject) -> Option<baylee_view::StackItem> {
    use baylee_view::StackItem;
    match obj.kind {
        ObjectKind::Spell => Some(StackItem::Spell),
        ObjectKind::AbilityOnStack => obj.ability.map(|loc| StackItem::Ability {
            source: loc.source,
            ability: loc
                .card
                .map(|card| baylee_core::ids::AbilityRef::new(card, loc.index)),
            text: loc
                .card
                .and_then(|card| stack_text(card, loc.index, obj.own_abilities)),
        }),
        _ => None,
    }
}

/// Which face of its card an ability on the stack came from.
///
/// The source's *current* face is the wrong answer and is the trap this
/// exists to avoid. An ability on the stack is independent of its source
/// (CR 113.7a), which may have transformed back or died since — and a
/// wrong face here is caught by nothing downstream, because both faces'
/// sentence counts are English, so the `of` guard agrees and the player is
/// shown the other side's sentence as precise text.
///
/// `own_abilities` is the right answer for free: it is the very `&'static`
/// slice `abilities_for_face` returned, captured at the moment the ability
/// was put on the stack (CR 608.2), so identity settles it. That is also
/// why a copy answers `None` — a Spark Double's ability carries the
/// *copied* card's list while the object's card is the physical one, no
/// face matches, and refusing is right: the alternative prints the wrong
/// card's sentence. An empty list is skipped rather than matched, because
/// it cannot be the source of an ability on the stack and two empty slices
/// may share an address.
fn ability_face(def: &baylee_cards::dsl::CardDef, captured: &'static [AbilityDef]) -> Option<u8> {
    if captured.is_empty() {
        return None;
    }
    (0..def.faces.len())
        .find(|&face| {
            let printed = def.abilities_for_face(face);
            std::ptr::eq(printed.as_ptr(), captured.as_ptr()) && printed.len() == captured.len()
        })
        .and_then(|face| u8::try_from(face).ok())
}

/// Where an ability's printed sentence is, for a client holding the card's
/// text in the player's own language.
///
/// Answers `None` for everything the generated table has no row for — a
/// reserved index (`AbilityRef::SPELL`, `SYNTHETIC`, …), a static ability,
/// a printed one no sentence fits — which is the whole point of it being
/// an `Option` on the wire. `docs/client.md` §"Which ability is on the
/// stack" is normative.
fn stack_text(
    card: baylee_core::ids::CardIndex,
    index: u32,
    captured: Option<&'static [AbilityDef]>,
) -> Option<baylee_view::StackText> {
    let def = baylee_cards::by_index(card)?;
    let face = ability_face(def, captured?)?;
    let line = baylee_cards::lines::ability_line(card, face as usize, index)?;
    Some(baylee_view::StackText {
        face,
        line: line.line,
        of: line.of,
    })
}

/// Collects a public zone into view objects.
fn zone(state: &GameState, loc: ZoneLocation, seat: PlayerId) -> Vec<PublicObject> {
    state
        .zones
        .list(loc)
        .iter()
        .filter_map(|id| public_object(state, *id, seat))
        .collect()
}

/// Collects one zone per seat, indexed by seat order.
fn per_seat_zone(
    state: &GameState,
    loc: fn(PlayerId) -> ZoneLocation,
    seat: PlayerId,
) -> Vec<Vec<PublicObject>> {
    state
        .players
        .iter()
        .map(|p| zone(state, loc(p.id), seat))
        .collect()
}

/// The floating mana of one seat, for the view.
///
/// Restricted mana is summed *per colour* rather than into one number. The
/// engine holds one `RestrictedMana` per production, so two taps of the same
/// Cavern naming white are two entries; the view is what a player reads, and
/// "two restricted white" is the reading — how it got there is not.
fn mana_pool(pool: &baylee_core::mana::ManaPool) -> baylee_view::ManaPoolView {
    use baylee_core::mana::ManaColor;
    let mut restricted = [0u16; 6];
    for mana in pool.restricted() {
        let slot = &mut restricted[mana.color.index()];
        *slot = slot.saturating_add(mana.amount);
    }
    baylee_view::ManaPoolView {
        white: pool.available(ManaColor::White),
        blue: pool.available(ManaColor::Blue),
        black: pool.available(ManaColor::Black),
        red: pool.available(ManaColor::Red),
        green: pool.available(ManaColor::Green),
        colorless: pool.available(ManaColor::Colorless),
        restricted,
    }
}

/// The objects a pending choice puts in front of the seat it is asking.
///
/// Only the choices that can name a *hidden* object are listed. Combat is
/// not: [`Pending::ChooseAttackers`] and [`Pending::ChooseBlockers`] name
/// creatures, and every creature in a declaration is on the battlefield, which
/// the view already carries in full. Neither is [`Pending::YesNo`]'s miracle
/// card — a miracle is revealed from the hand it was drawn into, and that is
/// the asking seat's own hand.
const fn offered(pending: &Pending) -> &[ObjectId] {
    match pending {
        Pending::ChooseCards { options, .. }
        | Pending::ChooseTargets { options, .. }
        | Pending::LegendChoice { options, .. } => options.as_slice(),
        Pending::OrderObjects { objects, .. } => objects.as_slice(),
        _ => &[],
    }
}

/// Whether `seat`'s view already carries this object somewhere.
///
/// The zones a view sends in full are the public ones plus the seat's own
/// hand; a library, a sideboard and somebody else's hand are counts. An
/// object in one of those is an object the client has no other way to draw,
/// which is exactly what [`PlayerView::looking_at`] is for.
const fn shown_elsewhere(obj: &GameObject, seat: PlayerId) -> bool {
    match obj.zone {
        Zone::Battlefield | Zone::Stack | Zone::Graveyard | Zone::Exile | Zone::Command => true,
        Zone::Hand => match obj.zone_owner {
            Some(owner) => owner.get() == seat.get(),
            None => false,
        },
        Zone::Library | Zone::OutsideGame => false,
    }
}

/// The hidden cards this seat is being shown, if any.
///
/// The entitlement rule is one sentence and it is the whole safety argument:
/// **an object the engine asks you about is an object you may see.** A search
/// is only offered to the searcher, a scry only to the scrying player, and
/// the engine has already filtered both down to what that player is allowed
/// to look at — so this function adds no judgement of its own beyond checking
/// that the question is addressed to `seat`.
///
/// It is deliberately not a memory. The list is rebuilt from the outstanding
/// choice on every view, so a card stops being visible the instant the choice
/// is answered, and there is no place for one to linger.
fn looking_at(state: &GameState, seat: PlayerId, pending: Option<&Pending>) -> Vec<PublicObject> {
    let Some(pending) = pending else {
        return Vec::new();
    };
    if pending_player(pending) != Some(seat) {
        return Vec::new();
    }
    offered(pending)
        .iter()
        .filter(|id| {
            state
                .object(**id)
                .is_some_and(|obj| !shown_elsewhere(obj, seat))
        })
        .filter_map(|id| public_object(state, *id, seat))
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
/// The engine charges generic mana and nothing else here, so what comes out
/// is a generic cost. It is a `ManaCost` rather than the engine's `u16`
/// because that `u16` is about *when* an amount is known — a tax can be its
/// own source's power until resolution evaluates it — and by the time a
/// window is open the number is settled, while both readers on the far side
/// (`manapip::cost`, `manaplan::plan`) already take a cost.
#[must_use]
pub fn owed_payment<L: baylee_engine::state::CardLookup>(
    engine: &baylee_engine::engine::Engine<L>,
) -> Option<ManaCost> {
    engine
        .payment_window()
        .map(|(_, mana)| ManaCost::from_symbol_generic(u32::from(mana)))
}

/// Builds the hidden-information-filtered view of `state` for `seat`.
///
/// `awaiting` is the seat the table is waiting for — whoever owes an answer,
/// whatever the question is. It is a parameter because the engine keeps it in
/// the pending choice rather than in the state, and it is the *pending* seat
/// rather than the priority holder because every reader of it means "waiting
/// on them": a seat picking blockers or discarding to hand size holds no
/// priority (CR 117) and is just as much the seat being waited for. Pass
/// `pending_player(engine.pending())`.
///
/// `pending` is the outstanding choice, and it is here for one reason:
/// [`PlayerView::looking_at`]. A tutor, a scry and a revealed hand all ask a
/// seat about objects that are in no zone the view carries, so the choice
/// itself is what decides which hidden objects this seat may see. Pass `None`
/// and the view is exactly what it was before — nothing else reads it.
///
/// `held` is whether this seat's own standing order is currently withholding
/// its priority, and it is a parameter for the same reason `awaiting` is: a
/// hold lives in the engine's `SeatAutomation`, not in the `GameState` this
/// function is handed, so only the caller can read it. Pass
/// `engine.automation(seat).hold.suppresses()`.
///
/// `owed` is the payment the awaited seat is inside a CR 605.3a window for,
/// and is the third of those — the window and the suspended resolution
/// holding its price are both on the `Engine`. Pass
/// `engine.payment_window().map(|(_, mana)| ManaCost::from_symbol_generic(mana.into()))`,
/// or `None` outside a window. It is the same value for every seat, because
/// what a seat has been asked to pay in the open is not hidden from the
/// table: [`PlayerView::awaiting`] already names who owes it.
#[must_use]
pub fn player_view(
    state: &GameState,
    seat: PlayerId,
    awaiting: Option<PlayerId>,
    seq: u64,
    pending: Option<&Pending>,
    held: bool,
    owed: Option<ManaCost>,
) -> PlayerView {
    let hand = state
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
                mana_value: chars.mana_cost.cmc(),
                colors: chars.colors,
                types: chars.types,
                commander: is_commander(state, *id),
            })
        })
        .collect();

    PlayerView {
        seq,
        seat,
        turn: state.turn.number,
        phase: phase(state.turn.phase),
        step: step(state.turn.step),
        active: state.turn.active,
        awaiting,
        priority_held: held,
        owed,
        monarch: state.monarch,
        day_night: state.day_night.map(day_night),
        seats: state
            .players
            .iter()
            .map(|p| SeatView {
                player: p.id,
                life: p.life,
                poison: p.poison,
                energy: p.energy,
                hand_count: state.zones.list(ZoneLocation::Hand(p.id)).len() as u32,
                library_count: state.zones.list(ZoneLocation::Library(p.id)).len() as u32,
                graveyard_count: state.zones.list(ZoneLocation::Graveyard(p.id)).len() as u32,
                has_lost: p.has_lost,
                mana_pool: mana_pool(&p.mana_pool),
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
            .collect(),
        hand,
        battlefield: zone(state, ZoneLocation::Battlefield, seat),
        stack: zone(state, ZoneLocation::Stack, seat),
        graveyards: per_seat_zone(state, ZoneLocation::Graveyard, seat),
        exile: per_seat_zone(state, ZoneLocation::Exile, seat),
        command: per_seat_zone(state, ZoneLocation::Command, seat),
        combat: CombatView {
            attackers: state
                .combat
                .attackers
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
        },
        looking_at: looking_at(state, seat, pending),
        // The same question `casting::timing_allows` asks before it refuses a
        // spell, asked once per view so the seat can be told before it spends
        // anything. It is the effect's *source* that travels, not a flag: the
        // client owes the player the card to point at.
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
    }
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
) -> GameStatic {
    GameStatic {
        view_version: baylee_view::VIEW_VERSION,
        game_id,
        your_seat,
        seats,
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
                            baylee_core::preset::Finish::Normal => baylee_view::Finish::Normal,
                        },
                    })
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use baylee_cards::by_oracle_id;
    use baylee_core::ids::{CardIndex, PrintRef};
    use baylee_core::preset::{
        AIProfile, DeckEntry, Finish, FormatId, GamePreset, HouseRules, PrintInfo, SeatController,
        SeatSpec,
    };
    use baylee_engine::engine::Engine;
    use baylee_engine::state::CardLookup;

    struct Registry;
    impl CardLookup for Registry {
        fn card(&self, index: CardIndex) -> Option<&'static baylee_cards_dsl::CardDef> {
            baylee_cards::by_index(index)
        }
    }

    fn island() -> CardIndex {
        by_oracle_id("b2c6aa39-2d2a-459c-a555-fb48ba993373")
            .unwrap()
            .index
    }

    fn print_info(lang: &str, finish: Finish) -> PrintInfo {
        PrintInfo {
            scryfall_id: uuid::Uuid::nil(),
            lang: lang.to_string(),
            finish,
        }
    }

    /// A deck holding the same card in three different printings, plus one
    /// copy of each already on the battlefield.
    fn mixed_print_preset() -> GamePreset {
        let mut deck: Vec<DeckEntry> = Vec::new();
        for i in 0..60u16 {
            deck.push(DeckEntry {
                card: island(),
                print: PrintRef::new(i % 3),
            });
        }
        let seat = |battlefield: Vec<DeckEntry>| SeatSpec {
            controller: SeatController::Ai(AIProfile::default()),
            capabilities: baylee_core::preset::SeatCapabilities {
                dev_commands: true,
                see_hidden: false,
            },
            deck: deck.clone(),
            sideboard: vec![],
            commanders: vec![],
            starting_life: None,
            starting_hand: Some(vec![
                DeckEntry {
                    card: island(),
                    print: PrintRef::new(0),
                },
                DeckEntry {
                    card: island(),
                    print: PrintRef::new(2),
                },
            ]),
            starting_battlefield: battlefield,
            emblems: vec![],
            team: None,
        };
        GamePreset {
            format: FormatId::Freeform,
            seed: 5,
            house_rules: HouseRules::default(),
            modifiers: vec![],
            prints: vec![
                print_info("EN", Finish::Normal),
                print_info("DE", Finish::Foil),
                print_info("JA", Finish::Etched),
            ],
            seats: vec![
                seat(vec![
                    DeckEntry {
                        card: island(),
                        print: PrintRef::new(1),
                    },
                    DeckEntry {
                        card: island(),
                        print: PrintRef::new(2),
                    },
                ]),
                seat(vec![]),
            ],
        }
    }

    fn teferi_time_raveler() -> CardIndex {
        by_oracle_id("ae7604bb-4818-45a3-960c-cf3d83f15964")
            .unwrap()
            .index
    }

    /// A preset with a Teferi, Time Raveler standing on seat 1's battlefield.
    fn a_table_under_teferi() -> GamePreset {
        let mut preset = mixed_print_preset();
        preset.seats[1].starting_battlefield = vec![DeckEntry {
            card: teferi_time_raveler(),
            print: PrintRef::new(0),
        }];
        preset
    }

    /// The seat Teferi is holding to sorcery speed is told so, and told by
    /// which card.
    ///
    /// This is the one timing fact a client cannot work out for itself. Its
    /// own rule is written to err in the direction that costs the player
    /// nothing — offer a spell the engine then refuses, rather than hide one
    /// it would have allowed — and for this effect it erred the expensive way
    /// round: an instant was offered unconditionally, the tap ran, the lands
    /// were spent, and only then was the spell refused.
    ///
    /// The object and not a flag, because the answer a player is owed is
    /// *which card*, and the bystander is the seat across the table: Teferi's
    /// own controller casts at whatever speed they like (CR 613 — the static
    /// says "each opponent"), so a view that named it for both seats would be
    /// a lock nobody could ever be outside of.
    #[test]
    fn the_seat_teferi_locks_is_told_which_card_is_locking_it() {
        use baylee_engine::choice::{Pending, PlayerAction};

        let preset = a_table_under_teferi();
        let mut engine = Engine::new(&preset, Registry).expect("game starts");
        // Past the mulligans, because a static is registered by a pass of the
        // machine and not by dealing the cards: the effect table is empty
        // until the game has actually started.
        for _ in 0..2 {
            let Pending::Mulligan { player, .. } = engine.pending().clone() else {
                panic!("expected a mulligan")
            };
            engine.apply(player, PlayerAction::MulliganKeep).unwrap();
        }
        let locked = player_view(engine.state(), PlayerId::new(0), None, 0, None, false, None);
        let theirs = player_view(engine.state(), PlayerId::new(1), None, 0, None, false, None);

        let teferi = theirs
            .battlefield
            .iter()
            .find(|o| o.controller == PlayerId::new(1))
            .expect("their walker is on the table");

        assert_eq!(
            locked.sorcery_lock,
            Some(teferi.id),
            "the seat it holds is told which permanent holds it"
        );
        assert_eq!(
            theirs.sorcery_lock, None,
            "and its own controller is not held by it"
        );
    }

    /// The whole point of `PrintRef`: two copies of the *same* card in one
    /// deck can be different printings, and the client has to be told which
    /// is which. The engine never interprets the ref — it carries it — so
    /// this test follows one deck entry all the way to the seat view.
    #[test]
    fn the_same_card_in_two_printings_stays_two_printings_in_the_view() {
        let preset = mixed_print_preset();
        let engine = Engine::new(&preset, Registry).expect("game starts");
        let seat = PlayerId::new(0);
        let view = player_view(engine.state(), seat, None, 0, None, false, None);

        let battlefield: Vec<u16> = view
            .battlefield
            .iter()
            .filter(|o| o.controller == seat)
            .filter_map(|o| o.card.map(|c| c.print.get()))
            .collect();
        assert_eq!(
            battlefield,
            vec![1, 2],
            "the battlefield lost the printings the preset asked for"
        );

        let hand: Vec<u16> = view.hand.iter().map(|o| o.card.print.get()).collect();
        assert_eq!(hand, vec![0, 2], "the hand lost its printings");

        // Same rules identity throughout — only the printing differs.
        assert!(
            view.hand.iter().all(|o| o.card.index == island()),
            "print refs must not disturb card identity"
        );
    }

    /// The print table is a per-game payload: the view carries indices, and
    /// `GameStatic` carries what they mean. A client that only got the
    /// indices could not fetch an image.
    #[test]
    fn the_static_payload_carries_what_a_print_ref_points_at() {
        let preset = mixed_print_preset();
        let shown = vec![true; preset.prints.len()];
        let statics = game_static(
            "g1".into(),
            PlayerId::new(0),
            vec![],
            &preset.prints,
            &shown,
        );
        assert_eq!(statics.prints.len(), 3);
        let entry = |i: u16| statics.print(PrintRef::new(i)).expect("shown");
        assert_eq!(entry(1).lang, "DE");
        assert!(matches!(entry(1).finish, baylee_view::Finish::Foil));
        assert!(matches!(entry(2).finish, baylee_view::Finish::Etched));
        assert_eq!(statics.view_version, baylee_view::VIEW_VERSION);
    }

    /// A printing this seat has not been shown is a hole in the table, not a
    /// shorter table: the index is the `PrintRef` every object points at.
    #[test]
    fn a_printing_a_seat_has_not_seen_is_a_hole_not_a_gap() {
        let preset = mixed_print_preset();
        let statics = game_static(
            "g1".into(),
            PlayerId::new(0),
            vec![],
            &preset.prints,
            &[true, false, true],
        );
        assert_eq!(statics.prints.len(), 3, "the indices do not move");
        assert!(statics.print(PrintRef::new(0)).is_some());
        assert!(statics.print(PrintRef::new(1)).is_none());
        assert!(matches!(
            statics.print(PrintRef::new(2)).map(|p| p.finish),
            Some(baylee_view::Finish::Etched)
        ));
    }

    /// A token has no printing, so `card` is `None` and a client has nothing
    /// to fetch an image with. The token id is the handle that replaces it:
    /// it survives the projection into the view, and it resolves back to the
    /// definition the engine created the object from.
    #[test]
    fn a_token_reaches_the_client_with_the_handle_its_art_is_keyed_on() {
        use baylee_engine::choice::{Pending, PlayerAction};

        let preset = mixed_print_preset();
        let mut engine = Engine::new(&preset, Registry).expect("game starts");
        for _ in 0..2 {
            let Pending::Mulligan { player, .. } = engine.pending().clone() else {
                panic!("expected a mulligan")
            };
            engine.apply(player, PlayerAction::MulliganKeep).unwrap();
        }

        // Every object the view can project; a card-backed one carries a
        // printing and no token id, and the two are mutually exclusive.
        let view = player_view(engine.state(), PlayerId::new(0), None, 1, None, false, None);
        for object in &view.battlefield {
            assert!(
                object.card.is_none() || object.token.is_none(),
                "{} claims to be both a printing and a token",
                object.name
            );
        }

        // And the id round-trips: whatever the view says, the registry can
        // name it. A token filed under `u16::MAX` — one defined in a card
        // file instead of the registry — would fail here.
        for id in 0..u16::try_from(baylee_cards::tokens::ALL.len()).expect("registry fits") {
            let token = baylee_cards::tokens::by_token_id(id).expect("id names a token");
            assert_eq!(baylee_cards::tokens::token_id(token), id);
        }
    }

    /// A card keeps its printing when it changes zone: the ref lives on the
    /// object, not on the zone it happens to be in. The land is played for
    /// real rather than moved by hand, so the whole cast path is covered.
    #[test]
    fn a_printing_survives_a_zone_change() {
        use baylee_engine::choice::{Pending, PlayerAction};

        let preset = mixed_print_preset();
        let mut engine = Engine::new(&preset, Registry).expect("game starts");
        for _ in 0..2 {
            let Pending::Mulligan { player, .. } = engine.pending().clone() else {
                panic!("expected a mulligan")
            };
            engine.apply(player, PlayerAction::MulliganKeep).unwrap();
        }
        let seat = PlayerId::new(0);

        // Walk to seat 0's main phase, where a land may be played.
        for _ in 0..30 {
            if let Pending::Priority { player, legal } = engine.pending()
                && *player == seat
                && !legal.lands.is_empty()
            {
                break;
            }
            let Pending::Priority { player, .. } = engine.pending().clone() else {
                panic!("expected priority, got {:?}", engine.pending())
            };
            engine.apply(player, PlayerAction::PassPriority).unwrap();
        }
        let Pending::Priority { legal, .. } = engine.pending().clone() else {
            panic!("expected priority")
        };
        let card = *legal.lands.first().expect("a land in hand to play");
        let print_before = engine
            .state()
            .object(card)
            .and_then(|o| o.card)
            .expect("a card-backed object")
            .print;
        engine
            .apply(seat, PlayerAction::PlayLand { card })
            .expect("playing a land from hand is legal");

        let view = player_view(engine.state(), seat, None, 1, None, false, None);
        let played = view
            .battlefield
            .iter()
            .find(|o| o.id == card)
            .expect("the land reached the battlefield");
        assert_eq!(
            played.card.expect("card-backed").print,
            print_before,
            "the printing was lost on the way to the battlefield"
        );
        assert!(
            !view.hand.iter().any(|o| o.id == card),
            "the land is still shown in hand"
        );
    }
    /// Every object id that appears anywhere in a view.
    fn ids_in(view: &baylee_view::PlayerView) -> Vec<ObjectId> {
        let mut ids: Vec<ObjectId> = view.hand.iter().map(|c| c.id).collect();
        for zone in [&view.battlefield, &view.stack] {
            ids.extend(zone.iter().map(|o| o.id));
        }
        for per_seat in [&view.graveyards, &view.exile, &view.command] {
            for zone in per_seat {
                ids.extend(zone.iter().map(|o| o.id));
            }
        }
        ids.extend(view.combat.attackers.iter().map(|a| a.creature));
        ids.extend(view.combat.blockers.iter().map(|b| b.blocker));
        ids
    }

    /// The opponent's hand is a number. Not a list the client is trusted to
    /// hide, not ids with the names stripped — a count, with no field the
    /// contents could travel in.
    #[test]
    fn an_opponents_hand_is_a_count_and_nothing_else() {
        let preset = mixed_print_preset();
        let engine = Engine::new(&preset, Registry).expect("game starts");
        let me = PlayerId::new(0);
        let them = PlayerId::new(1);
        let view = player_view(engine.state(), me, None, 1, None, false, None);

        let their_hand = engine.state().zones.list(ZoneLocation::Hand(them));
        assert!(!their_hand.is_empty(), "the opponent holds cards");
        assert_eq!(
            view.seat(them).map(|s| s.hand_count),
            Some(their_hand.len() as u32),
            "the count is what a client gets"
        );
        let visible = ids_in(&view);
        for id in their_hand {
            assert!(
                !visible.contains(id),
                "an opponent's hand card reached seat 0's view: {id:?}"
            );
        }
    }

    /// Nobody's library is in the view — not even the viewing seat's own.
    /// A player who could read their own library order would know every
    /// draw, which is the same leak wearing a friendlier hat.
    #[test]
    fn no_library_card_reaches_any_view() {
        let preset = mixed_print_preset();
        let engine = Engine::new(&preset, Registry).expect("game starts");
        for seat in [PlayerId::new(0), PlayerId::new(1)] {
            let view = player_view(engine.state(), seat, None, 1, None, false, None);
            let visible = ids_in(&view);
            for owner in [PlayerId::new(0), PlayerId::new(1)] {
                let library = engine.state().zones.list(ZoneLocation::Library(owner));
                assert!(
                    library.len() > 40,
                    "the library should still be nearly whole"
                );
                for id in library {
                    assert!(
                        !visible.contains(id),
                        "a library card reached {seat:?}'s view: {id:?}"
                    );
                }
                assert_eq!(
                    view.seat(owner).map(|s| s.library_count),
                    Some(library.len() as u32)
                );
            }
        }
    }

    /// Two seats looking at the same battlefield see different things when a
    /// permanent is face down (CR 708.5): its controller knows what they
    /// played, everyone else gets a blank with no card identity at all.
    #[test]
    fn a_face_down_permanent_is_blank_to_everyone_but_its_controller() {
        let preset = mixed_print_preset();
        let mut engine = Engine::new(&preset, Registry).expect("game starts");
        let me = PlayerId::new(0);
        let them = PlayerId::new(1);
        let land = engine.state().zones.list(ZoneLocation::Battlefield)[0];
        // The test preset grants seat 0 dev commands; a lobby game grants
        // nobody any, which is what makes this the harness and not a hole.
        engine
            .dev_state_mut(me)
            .expect("the test preset grants dev commands")
            .object_mut(land)
            .expect("the permanent is there")
            .status
            .insert(baylee_engine::object::Status::FACE_DOWN);

        let mine = player_view(engine.state(), me, None, 1, None, false, None);
        let theirs = player_view(engine.state(), them, None, 1, None, false, None);
        let of = |v: &baylee_view::PlayerView| {
            v.battlefield
                .iter()
                .find(|o| o.id == land)
                .expect("the permanent is on the shared battlefield")
                .clone()
        };
        assert!(
            of(&mine).card.is_some(),
            "its controller knows what they played"
        );
        assert!(
            of(&theirs).card.is_none(),
            "the opponent was handed the card identity of a face-down permanent"
        );
        assert_eq!(of(&theirs).name, "Face-down");
    }

    /// A search offered to `player`, over `options`.
    fn search(player: PlayerId, options: Vec<ObjectId>) -> Pending {
        Pending::ChooseCards {
            player,
            options,
            min: 1,
            max: 1,
            prompt: baylee_engine::choice::ChoicePrompt::SearchLibrary,
        }
    }

    /// The first `n` cards of a seat's library, as object ids.
    fn library(engine: &Engine<Registry>, seat: PlayerId, n: usize) -> Vec<ObjectId> {
        engine
            .state()
            .zones
            .list(ZoneLocation::Library(seat))
            .iter()
            .take(n)
            .copied()
            .collect()
    }

    /// A tutor hands a seat object ids out of its own library. Every other
    /// zone the client can draw from is in the view already; these are in
    /// none of them, so without `looking_at` the dialog is a row of blanks
    /// and the choice cannot be answered at all.
    #[test]
    fn a_searching_seat_is_shown_the_cards_it_was_offered() {
        let preset = mixed_print_preset();
        let engine = Engine::new(&preset, Registry).expect("game starts");
        let seat = PlayerId::new(0);
        let offered = library(&engine, seat, 3);
        let pending = search(seat, offered.clone());

        let view = player_view(engine.state(), seat, None, 0, Some(&pending), false, None);
        let shown: Vec<ObjectId> = view.looking_at.iter().map(|o| o.id).collect();
        assert_eq!(
            shown, offered,
            "the searcher was not shown what it was asked about"
        );
        assert!(
            view.looking_at.iter().all(|o| o.card.is_some()),
            "a card offered out of a library arrived without its identity"
        );
    }

    /// The entitlement is the question, not the game state: the seat being
    /// asked sees the search, and the table does not. This is the sentence
    /// the whole field rests on, so it is the one with a test.
    #[test]
    fn nobody_else_is_shown_another_seat_s_search() {
        let preset = mixed_print_preset();
        let engine = Engine::new(&preset, Registry).expect("game starts");
        let searcher = PlayerId::new(0);
        let pending = search(searcher, library(&engine, searcher, 3));

        let theirs = player_view(
            engine.state(),
            PlayerId::new(1),
            None,
            0,
            Some(&pending),
            false,
            None,
        );
        assert!(
            theirs.looking_at.is_empty(),
            "an opponent was shown the cards a searching seat is looking through"
        );
    }

    /// And it is not a memory. The list is rebuilt from the outstanding
    /// choice every time, so the moment the question is gone the cards are
    /// gone with it — there is nowhere for one to linger.
    #[test]
    fn a_card_stops_being_shown_when_the_question_ends() {
        let preset = mixed_print_preset();
        let engine = Engine::new(&preset, Registry).expect("game starts");
        let seat = PlayerId::new(0);

        let view = player_view(engine.state(), seat, None, 0, None, false, None);
        assert!(
            view.looking_at.is_empty(),
            "a view with no pending choice was still showing cards"
        );
    }

    /// Most choices name things that are already on the table, and those must
    /// not arrive twice: a client that drew `looking_at` as a dialog would
    /// open one over an ordinary "target creature".
    #[test]
    fn an_offer_of_things_already_in_view_shows_nothing_extra() {
        let preset = mixed_print_preset();
        let engine = Engine::new(&preset, Registry).expect("game starts");
        let seat = PlayerId::new(0);
        let battlefield: Vec<ObjectId> =
            engine.state().zones.list(ZoneLocation::Battlefield).clone();
        assert!(!battlefield.is_empty(), "the preset seats a battlefield");
        let pending = Pending::ChooseTargets {
            player: seat,
            options: battlefield,
            player_options: vec![],
            min: 1,
            max: 1,
            reason: baylee_engine::choice::TargetPrompt::Targets,
        };

        let view = player_view(engine.state(), seat, None, 0, Some(&pending), false, None);
        assert!(
            view.looking_at.is_empty(),
            "objects the view already carries were repeated as things being shown"
        );
    }

    /// A seat earns a printing by seeing the card, and a card out of a
    /// library is a card it now sees. Without this the print table has no
    /// entry for it and the dialog draws rectangles.
    #[test]
    fn a_card_being_shown_earns_its_printing() {
        let preset = mixed_print_preset();
        let engine = Engine::new(&preset, Registry).expect("game starts");
        let seat = PlayerId::new(0);
        let pending = search(seat, library(&engine, seat, 3));

        let view = player_view(engine.state(), seat, None, 0, Some(&pending), false, None);
        for object in &view.looking_at {
            let print = object
                .card
                .expect("a library card is known to its owner")
                .print;
            assert!(
                view.prints().any(|p| p == print),
                "a card being shown did not earn its printing"
            );
        }
    }
    /// A seat's own hand is in its view already, so being asked about it
    /// shows nothing twice. This is the arm of [`shown_elsewhere`] with a
    /// judgement in it: hand is the one zone whose visibility depends on
    /// whose hand it is.
    #[test]
    fn a_seat_asked_about_its_own_hand_is_shown_nothing_extra() {
        let preset = mixed_print_preset();
        let engine = Engine::new(&preset, Registry).expect("game starts");
        let seat = PlayerId::new(0);
        let hand = engine.state().zones.list(ZoneLocation::Hand(seat)).clone();
        assert!(!hand.is_empty(), "the preset deals a starting hand");
        let pending = Pending::ChooseCards {
            player: seat,
            options: hand,
            min: 1,
            max: 1,
            prompt: baylee_engine::choice::ChoicePrompt::PutBackOnTop,
        };

        let view = player_view(engine.state(), seat, None, 0, Some(&pending), false, None);
        assert!(
            view.looking_at.is_empty(),
            "a seat's own hand was repeated as something it is being shown"
        );
    }

    /// And the other side of the same arm, which is where the whole rule is
    /// worth its cost: a discard-at-random or a Thoughtseize asks one seat
    /// about *another* seat's hand. Those cards are hidden from everyone by
    /// default, and the question is what entitles this seat to them — so the
    /// asked seat sees them, in full, and only while it is asked.
    #[test]
    fn a_seat_asked_about_another_hand_is_shown_it() {
        let preset = mixed_print_preset();
        let engine = Engine::new(&preset, Registry).expect("game starts");
        let (me, them) = (PlayerId::new(0), PlayerId::new(1));
        let hand = engine.state().zones.list(ZoneLocation::Hand(them)).clone();
        assert!(!hand.is_empty(), "the preset deals a starting hand");
        let pending = Pending::ChooseCards {
            player: me,
            options: hand.clone(),
            min: 1,
            max: 1,
            prompt: baylee_engine::choice::ChoicePrompt::Generic,
        };

        let view = player_view(engine.state(), me, None, 0, Some(&pending), false, None);
        let shown: Vec<ObjectId> = view.looking_at.iter().map(|o| o.id).collect();
        assert_eq!(
            shown, hand,
            "the asked seat was not shown the hand in question"
        );
        assert!(
            view.looking_at.iter().all(|o| o.card.is_some()),
            "a card this seat is being asked about arrived without its identity"
        );

        // The owner of that hand is being asked nothing, and is shown nothing.
        let theirs = player_view(engine.state(), them, None, 0, Some(&pending), false, None);
        assert!(
            theirs.looking_at.is_empty(),
            "a seat not being asked was handed a list anyway"
        );
    }
    /// A land under a Chromatic Lantern taps for any colour, and there is no
    /// card anywhere a client could read that off — the ability exists only
    /// in the effect table. It is projected for the same reason as an
    /// animated land's types: without it a client's mana planner counts that
    /// land for nothing and the player taps it by hand.
    ///
    /// The opponent's land in the same test is the half that matters as much:
    /// the grant says "lands *you* control", and a projection that ignored
    /// the filter would offer the planner a land the engine refuses.
    #[test]
    fn a_land_under_a_lantern_says_what_it_now_makes() {
        use baylee_engine::choice::{Pending, PlayerAction};

        let lantern = by_oracle_id("539f5396-d99a-417d-a84c-dff7930b5900")
            .expect("Chromatic Lantern is in the pool")
            .index;
        let mut preset = mixed_print_preset();
        let land = DeckEntry {
            card: island(),
            print: PrintRef::new(0),
        };
        preset.seats[0].starting_battlefield = vec![
            land,
            DeckEntry {
                card: lantern,
                print: PrintRef::new(0),
            },
        ];
        preset.seats[1].starting_battlefield = vec![land];

        let mut engine = Engine::new(&preset, Registry).expect("game starts");
        for _ in 0..2 {
            let Pending::Mulligan { player, .. } = engine.pending().clone() else {
                panic!("expected a mulligan")
            };
            engine.apply(player, PlayerAction::MulliganKeep).unwrap();
        }
        let view = player_view(engine.state(), PlayerId::new(0), None, 1, None, false, None);

        let land_of = |seat: u8| {
            view.battlefield
                .iter()
                .find(|o| {
                    o.controller == PlayerId::new(seat)
                        && o.types.contains(baylee_core::types::TypeSet::LAND)
                })
                .expect("each seat has its land")
        };
        let granted = land_of(0)
            .granted_mana
            .as_ref()
            .expect("the Lantern grants the land an ability");
        assert_eq!(granted.amount, 1, "one mana, of a colour it will ask for");
        assert_eq!(
            granted.colors.len(),
            5,
            "any colour, and the client has to know which five"
        );

        assert!(
            land_of(1).granted_mana.is_none(),
            "the grant is `lands you control` and the opponent is not you"
        );
        let lantern_itself = view
            .battlefield
            .iter()
            .find(|o| o.types.contains(baylee_core::types::TypeSet::ARTIFACT))
            .expect("the Lantern is on the battlefield");
        assert!(
            lantern_itself.granted_mana.is_none(),
            "the Lantern's own mana ability is printed on it and is not a grant"
        );

        // And the half that makes the projection worth anything: the engine
        // offers this exact land under this exact handle. A view that said a
        // land makes mana the engine will not hand out is worse than one that
        // said nothing — the planner would tap it and the payment would fail.
        let land = land_of(0).id;
        for _ in 0..30 {
            let Pending::Priority { player, legal } = engine.pending().clone() else {
                break;
            };
            if player == PlayerId::new(0) {
                assert!(
                    legal
                        .abilities
                        .contains(&(land, baylee_engine::choice::GRANTED_ABILITY)),
                    "the engine offers the granted ability the view described"
                );
                assert!(
                    legal.mana_abilities.contains(&land),
                    "and offers it as a mana ability, which is why it needs no stack"
                );
                return;
            }
            engine.apply(player, PlayerAction::PassPriority).unwrap();
        }
        panic!("seat 0 never got priority");
    }

    /// Katara, the Fearless — a legendary creature, so a legal commander.
    fn katara() -> CardIndex {
        by_oracle_id("0972d46e-423b-454e-87c7-a2d40fb6fb6d")
            .unwrap()
            .index
    }

    /// Elesh Norn, Mother of Machines — seat 0's *second* commander.
    ///
    /// The second one is the fixture and not decoration. At a table where
    /// every seat has exactly one commander, a list indexed by seat and a
    /// list indexed by commander have the same length and the same order,
    /// and no assertion can tell the fix from the bug it replaced.
    fn elesh_norn() -> CardIndex {
        by_oracle_id("5ade11c0-41dd-4b6a-9f5b-c5903a3a0d7f")
            .unwrap()
            .index
    }

    /// A Commander table: seat 0 with partners, seat 1 with one commander,
    /// and an Island on each battlefield to contrast against.
    fn commander_preset() -> GamePreset {
        let deck: Vec<DeckEntry> = (0..60)
            .map(|_| DeckEntry {
                card: island(),
                print: PrintRef::new(0),
            })
            .collect();
        let seat = |commanders: Vec<CardIndex>| SeatSpec {
            controller: SeatController::Ai(AIProfile::default()),
            capabilities: baylee_core::preset::SeatCapabilities::default(),
            deck: deck.clone(),
            sideboard: vec![],
            commanders: commanders
                .into_iter()
                .map(|card| DeckEntry {
                    card,
                    print: PrintRef::new(0),
                })
                .collect(),
            starting_life: None,
            starting_hand: Some(vec![]),
            starting_battlefield: vec![DeckEntry {
                card: island(),
                print: PrintRef::new(0),
            }],
            emblems: vec![],
            team: None,
        };
        GamePreset {
            format: FormatId::Commander,
            seed: 5,
            house_rules: HouseRules::default(),
            modifiers: vec![],
            prints: vec![print_info("EN", Finish::Normal)],
            seats: vec![seat(vec![katara(), elesh_norn()]), seat(vec![katara()])],
        }
    }

    fn sorted(mut ids: Vec<ObjectId>) -> Vec<ObjectId> {
        ids.sort_by_key(|o| o.slot());
        ids
    }

    /// A seat's commander line is that seat's, and it is one entry per
    /// commander.
    ///
    /// It used to be `GameState::commander_casts` — one number per *seat* —
    /// cloned whole into every seat's line, which is why the command-zone
    /// panel indexed it by command-zone slot and got away with it: at a duel
    /// where both seats have one commander the two shapes coincide. They
    /// stop coinciding here, in both directions at once.
    #[test]
    fn each_seats_commanders_are_its_own_and_are_listed_one_per_commander() {
        let preset = commander_preset();
        let engine = Engine::new(&preset, Registry).expect("game starts");

        for seat in [0u8, 1] {
            let view = player_view(
                engine.state(),
                PlayerId::new(seat),
                None,
                0,
                None,
                false,
                None,
            );
            assert_eq!(
                view.seats[0].commanders.len(),
                2,
                "seat 0 has partners, and seat {seat}'s view forgot one"
            );
            assert_eq!(
                view.seats[1].commanders.len(),
                1,
                "seat 1 has one commander, and seat {seat}'s view invented another"
            );
            for (i, line) in view.seats.iter().enumerate() {
                assert_eq!(
                    sorted(line.commanders.iter().map(|c| c.object).collect()),
                    sorted(view.command[i].iter().map(|o| o.id).collect()),
                    "seat {i}'s line does not name the cards in seat {i}'s command zone"
                );
                assert!(
                    line.commanders.iter().all(|c| c.casts == 0),
                    "nothing has been cast yet"
                );
            }
        }

        // Everything above still passes if `casts` is filled from the seat
        // total, because at the start of a game every count is zero. So give
        // the three commanders three different numbers — none of which is a
        // number any *seat* could be holding — and read them back. This is
        // the assertion the old shape could not have satisfied: seat 0's two
        // commanders have to answer 2 and 5, and one number per seat cannot
        // say that.
        let mut state = engine.state().clone();
        state.commanders[0][0].casts = 2;
        state.commanders[0][1].casts = 5;
        state.commanders[1][0].casts = 7;
        state.commander_casts = vec![99, 99];

        for seat in [0u8, 1] {
            let view = player_view(&state, PlayerId::new(seat), None, 0, None, false, None);
            let casts = |i: usize| -> Vec<u32> {
                view.seats[i].commanders.iter().map(|c| c.casts).collect()
            };
            assert_eq!(casts(0), vec![2, 5], "seat {seat}'s view of the partners");
            assert_eq!(casts(1), vec![7], "seat {seat}'s view of seat 1");
        }
    }

    /// The marker on the object and the seat's line are the same claim, so
    /// they must never disagree. The object carries it only so that a
    /// renderer holding one card does not have to carry the seat list down
    /// with it — a redundancy that is safe exactly as long as this holds.
    #[test]
    fn the_marker_on_a_card_agrees_with_the_seat_that_claims_it() {
        let preset = commander_preset();
        let engine = Engine::new(&preset, Registry).expect("game starts");
        let view = player_view(engine.state(), PlayerId::new(0), None, 0, None, false, None);

        let named: Vec<ObjectId> = view
            .seats
            .iter()
            .flat_map(|s| s.commanders.iter().map(|c| c.object))
            .collect();
        assert_eq!(named.len(), 3, "two commanders for seat 0, one for seat 1");

        let mut marked = 0;
        for obj in view.command.iter().flatten().chain(&view.battlefield) {
            assert_eq!(
                obj.commander,
                named.contains(&obj.id),
                "the marker on {} disagrees with the seat list",
                obj.name
            );
            marked += usize::from(obj.commander);
        }
        assert_eq!(marked, 3, "the command zones did not carry the marker");
        assert!(
            view.battlefield.iter().all(|o| !o.commander),
            "an Island is not anybody's commander"
        );
    }

    /// A commander that declined CR 903.9b's replacement sits in its owner's
    /// hand, and everything the view says about it has to survive the trip.
    ///
    /// Two claims, and the second is the one with a way to go wrong. The
    /// marker stays, because the card is still a commander. And *every* seat
    /// keeps being told its identity — CR 903.3 designates a commander
    /// openly, so a hand is not a hiding place for the fact that it is one —
    /// which means the printing has to be earned by a seat that can no
    /// longer see the card in any zone it is sent.
    #[test]
    fn a_commander_in_its_owners_hand_keeps_its_marker_and_its_printing() {
        let preset = commander_preset();
        let engine = Engine::new(&preset, Registry).expect("game starts");
        let mut state = engine.state().clone();
        let katara_obj = state.commanders[0][0].object;
        let print = state
            .object(katara_obj)
            .and_then(|o| o.card)
            .expect("a commander has a card")
            .print;
        state
            .move_object(
                katara_obj,
                ZoneLocation::Hand(PlayerId::new(0)),
                baylee_engine::zone::ZonePosition::Top,
                baylee_engine::event::Cause::Effect,
            )
            .expect("the commander reaches its owner's hand");

        let owner = player_view(&state, PlayerId::new(0), None, 0, None, false, None);
        let held = owner
            .hand
            .iter()
            .find(|o| o.id == katara_obj)
            .expect("it is in the hand it was sent to");
        assert!(held.commander, "it is still a commander in a hand");

        let other = player_view(&state, PlayerId::new(1), None, 0, None, false, None);
        assert!(
            other.command[0].iter().all(|o| o.id != katara_obj),
            "it has left the command zone, so no seat sees it there"
        );
        let named = other.seats[0]
            .commanders
            .iter()
            .find(|c| c.object == katara_obj)
            .expect("seat 0's line still names it");
        assert_eq!(
            named.card.map(|c| c.print),
            Some(print),
            "and still says which printing it is"
        );
        assert!(
            other.prints().any(|p| p == print),
            "a seat told about a printing has to earn it, or it draws a hole"
        );
    }

    /// CR 302.6 is a rule about creatures, and the projection says so. The
    /// field used to answer "did this permanent enter this turn", which is
    /// also true of a land the player just played — so every client had to
    /// mask it off again to avoid drawing a whole opening board asleep, and
    /// the fact itself stayed wrong for anything that read it straight.
    #[test]
    fn only_a_creature_is_projected_summoning_sick() {
        let preset = mixed_print_preset();
        let engine = Engine::new(&preset, Registry).expect("game starts");
        let mut state = engine.state().clone();
        let seat = PlayerId::new(0);

        let mut fresh = |name: &str, types| {
            let name = state.names.intern(name);
            let id = state.create_bare(
                seat,
                baylee_engine::object::ObjectKind::Permanent,
                name,
                baylee_engine::zone::ZoneLocation::Battlefield,
            );
            state.object_mut(id).expect("just created").base_mut().types = types;
            id
        };
        let land = fresh("Fresh Land", baylee_core::types::TypeSet::LAND);
        let bear = fresh("Fresh Bear", baylee_core::types::TypeSet::CREATURE);

        let view = player_view(&state, seat, None, 0, None, false, None);
        let asleep = |id: ObjectId| {
            view.battlefield
                .iter()
                .find(|o| o.id == id)
                .expect("the permanent is in the view")
                .summoning_sick
        };
        assert!(
            !asleep(land),
            "a land played this turn was projected summoning sick"
        );
        assert!(asleep(bear), "a creature that entered this turn is asleep");
    }

    /// CR 306.5c: a planeswalker's loyalty is the counters on it, not the
    /// number printed on the card. The client draws the plate off this field,
    /// so a printed number meant a walker stood at its starting loyalty for
    /// the whole game however it was ticked or attacked.
    #[test]
    fn a_planeswalker_is_projected_at_the_loyalty_it_has() {
        let preset = mixed_print_preset();
        let engine = Engine::new(&preset, Registry).expect("game starts");
        let mut state = engine.state().clone();
        let seat = PlayerId::new(0);

        let mut walker = |zone, counters: u16| {
            let name = state.names.intern("Fresh Walker");
            let kind = if matches!(zone, baylee_engine::zone::ZoneLocation::Battlefield) {
                baylee_engine::object::ObjectKind::Permanent
            } else {
                baylee_engine::object::ObjectKind::Card
            };
            let id = state.create_bare(seat, kind, name, zone);
            let obj = state.object_mut(id).expect("just created");
            obj.base_mut().types = baylee_core::types::TypeSet::PLANESWALKER;
            obj.base_mut().loyalty = Some(4);
            obj.counters
                .set(baylee_cards_dsl::CounterKind::Loyalty, counters);
            id
        };
        let ticked = walker(baylee_engine::zone::ZoneLocation::Battlefield, 6);
        let dying = walker(baylee_engine::zone::ZoneLocation::Battlefield, 1);
        let held = walker(baylee_engine::zone::ZoneLocation::Graveyard(seat), 0);

        let view = player_view(&state, seat, None, 0, None, false, None);
        let loyalty = |id: ObjectId| {
            view.battlefield
                .iter()
                .chain(view.graveyards.iter().flatten())
                .find(|o| o.id == id)
                .expect("the object is in the view")
                .loyalty
        };
        // Both directions: a printed number would answer 4 for each of them,
        // so one of these alone proves nothing.
        assert_eq!(loyalty(ticked), Some(6), "a walker that ticked up");
        assert_eq!(loyalty(dying), Some(1), "a walker that has been attacked");
        // Off the battlefield there are no counters and the card is what it
        // prints, which is the answer a graveyard panel wants.
        assert_eq!(
            loyalty(held),
            Some(4),
            "a walker card is its printed number"
        );
    }

    /// The tally is a second life total (CR 903.10a), and it is public: the
    /// seat taking the damage is not the only one who needs to see how close
    /// twenty-one is.
    #[test]
    fn commander_damage_reaches_every_seats_view_keyed_by_the_commander() {
        let preset = commander_preset();
        let engine = Engine::new(&preset, Registry).expect("game starts");
        let mut state = engine.state().clone();
        let katara_obj = state.commanders[0][0].object;
        let norn_obj = state.commanders[0][1].object;
        state.players[1].commander_damage.push((katara_obj, 13));
        state.players[1].commander_damage.push((norn_obj, 4));

        for seat in [0u8, 1] {
            let view = player_view(&state, PlayerId::new(seat), None, 0, None, false, None);
            let taken: Vec<(ObjectId, u16)> = view.seats[1]
                .commander_damage
                .iter()
                .map(|d| (d.source, d.amount))
                .collect();
            assert_eq!(
                taken,
                vec![(katara_obj, 13), (norn_obj, 4)],
                "seat {seat} was not told what seat 1 has taken, and from which commander"
            );
            assert!(
                view.seats[0].commander_damage.is_empty(),
                "seat 0 has taken none"
            );
        }
    }

    /// The face a stack entry's text is read against is recovered by
    /// *identity*, and every answer this can give is one a player sees.
    ///
    /// Sheoldred is the whole reason the face is resolved at all rather
    /// than assumed to be zero: her back face is the only one in the pool
    /// that puts an ability on the stack
    /// (`baylee_cards::lines` counts them). The copy case is the one that
    /// has to answer *nothing* — a Spark Double's ability carries the
    /// copied card's list while the object's card is the physical one, and
    /// "no face matches" is the only honest answer there. Printing the
    /// physical card's sentence instead would put a stranger's text on the
    /// stack, which is worse than the bare label it replaces.
    #[test]
    fn a_stack_entrys_face_is_recovered_from_the_list_it_took_with_it() {
        let sheoldred = baylee_cards::all()
            .find(|d| d.name() == "Sheoldred")
            .expect("the pool has Sheoldred");
        let other = baylee_cards::all()
            .find(|d| d.name() != "Sheoldred" && !d.abilities_for_face(0).is_empty())
            .expect("the pool has some other card with abilities");

        assert_eq!(
            ability_face(sheoldred, sheoldred.abilities_for_face(1)),
            Some(1),
            "the back face's own list names the back face"
        );
        assert_eq!(
            ability_face(sheoldred, sheoldred.abilities_for_face(0)),
            Some(0)
        );
        assert_eq!(
            ability_face(sheoldred, other.abilities_for_face(0)),
            None,
            "a copy carries the copied card's list; no face of the physical card is it"
        );
        assert_eq!(
            ability_face(sheoldred, &[]),
            None,
            "an empty list cannot be the source of an ability, and shares an address"
        );
    }

    use baylee_core::mana::ManaColor;
    use baylee_engine::choice::{Pending, PlayerAction};

    /// The four cards whose mana nothing but a board can name.
    fn card_named(oracle: &str) -> CardIndex {
        by_oracle_id(oracle).expect("the card is in the pool").index
    }

    fn reflecting_pool() -> CardIndex {
        card_named("67f43ac6-2a58-4b53-b5d7-0330e2a252e2")
    }

    fn exotic_orchard() -> CardIndex {
        card_named("27b047e3-0d41-45e2-98e9-9391d7923a1e")
    }

    fn fellwar_stone() -> CardIndex {
        card_named("95560508-7ac9-4be9-8a3f-3c7d5b52807b")
    }

    fn command_tower() -> CardIndex {
        card_named("0895c9b7-ae7d-4bb3-af17-3b75deb50a25")
    }

    fn forest() -> CardIndex {
        card_named("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
    }

    fn plains() -> CardIndex {
        card_named("bc71ebf6-2056-41f7-be35-b2e5c34afa99")
    }

    /// A duel with a named board on each side, past the mulligans.
    ///
    /// The preset is `mixed_print_preset`'s, so the library is Islands and
    /// nothing draws a card that matters; what each test writes is the two
    /// `starting_battlefield`s, which is the only input `board_mana` reads.
    fn board(seat0: &[CardIndex], seat1: &[CardIndex]) -> (Engine<Registry>, PlayerView) {
        let entry = |&card: &CardIndex| DeckEntry {
            card,
            print: PrintRef::new(0),
        };
        let mut preset = mixed_print_preset();
        preset.seats[0].starting_battlefield = seat0.iter().map(entry).collect();
        preset.seats[1].starting_battlefield = seat1.iter().map(entry).collect();

        let mut engine = Engine::new(&preset, Registry).expect("game starts");
        for _ in 0..2 {
            let Pending::Mulligan { player, .. } = engine.pending().clone() else {
                panic!("expected a mulligan")
            };
            engine.apply(player, PlayerAction::MulliganKeep).unwrap();
        }
        let view = player_view(engine.state(), PlayerId::new(0), None, 1, None, false, None);
        (engine, view)
    }

    /// What the host projected onto the one permanent of that name.
    fn projection<'a>(view: &'a PlayerView, name: &str) -> Option<&'a baylee_view::BoardMana> {
        view.battlefield
            .iter()
            .find(|o| o.name == name)
            .unwrap_or_else(|| panic!("{name} is on the battlefield"))
            .board_mana
            .as_ref()
    }

    /// The defect this whole field exists for, measured at the board it was
    /// found on. A Reflecting Pool beside a Forest and a Plains makes white
    /// and green — and said nothing at all before, because the colours are a
    /// union over `produced_colors`, a *projected* characteristic no view
    /// carried. In the game it was found in, a `{3}{R}` creature would not
    /// arm with four untapped lands on the table.
    #[test]
    fn a_reflecting_pool_says_what_the_lands_beside_it_make() {
        let (_, view) = board(&[reflecting_pool(), forest(), plains()], &[]);
        let pool = projection(&view, "Reflecting Pool").expect("the Pool has a board to read");
        assert_eq!(
            pool.colors,
            vec![ManaColor::White, ManaColor::Green],
            "the union of what the lands you control could produce"
        );
    }

    /// And the other half of that comparison, which is what makes the first
    /// one a measurement: a Pool with no other land is a Pool that makes
    /// nothing, and the host says so by projecting nothing at all. A client
    /// that read an empty list as "any colour" would tap it and stall.
    #[test]
    fn a_lone_reflecting_pool_is_nothing_to_plan_with() {
        let (_, view) = board(&[reflecting_pool()], &[forest()]);
        assert!(
            projection(&view, "Reflecting Pool").is_none(),
            "the Pool contributes nothing to its own union, and the Forest is not yours"
        );
    }

    /// Exotic Orchard reads the *other* side of the table, which is the same
    /// rule with `mine` flipped — and the case a projection built from "the
    /// lands I can see" would get exactly backwards.
    #[test]
    fn an_exotic_orchard_reads_the_other_seats_lands() {
        let (_, view) = board(&[exotic_orchard(), forest()], &[plains()]);
        let orchard = projection(&view, "Exotic Orchard").expect("the opponent has a land");
        assert_eq!(
            orchard.colors,
            vec![ManaColor::White],
            "the Plains opposite, and not the Forest beside it"
        );
    }

    /// Fellwar Stone is the same source on a card that is not a land, which
    /// is why the projection is offered for every permanent rather than for
    /// lands alone.
    #[test]
    fn a_fellwar_stone_is_a_board_reader_that_is_not_a_land() {
        let (_, view) = board(&[fellwar_stone()], &[forest(), plains()]);
        let stone = projection(&view, "Fellwar Stone").expect("the opponent has lands");
        assert_eq!(stone.colors, vec![ManaColor::White, ManaColor::Green]);
    }

    /// A Forest carries none, and that is the economy of the field: what a
    /// card can answer on its own stays on the card, so the wire pays a
    /// colour list only for the four permanents that need one.
    #[test]
    fn a_forest_needs_no_projection() {
        let (_, view) = board(&[forest(), plains()], &[]);
        assert!(projection(&view, "Forest").is_none());
        assert!(projection(&view, "Plains").is_none());
    }

    /// The second board-dependent source, folded into the same field so that
    /// the two are never resolved on different sides of the wire. A Command
    /// Tower with no commander at the table is the offline house duel, where
    /// colourless is what the engine's own fallback leaves.
    #[test]
    fn a_command_tower_at_a_table_with_no_commander_is_colorless() {
        let (_, view) = board(&[command_tower()], &[]);
        let tower = projection(&view, "Command Tower").expect("the fallback is still an answer");
        assert_eq!(tower.colors, vec![ManaColor::Colorless]);
    }

    /// And at a Commander table it is the commanders' identity — seat 0 has
    /// two, so this is also the case a projection reading one commander would
    /// get wrong.
    #[test]
    fn a_command_tower_is_its_seats_commander_identity() {
        let mut preset = commander_preset();
        let tower = DeckEntry {
            card: command_tower(),
            print: PrintRef::new(0),
        };
        preset.seats[0].starting_battlefield.push(tower);
        let mut engine = Engine::new(&preset, Registry).expect("game starts");
        for _ in 0..2 {
            let Pending::Mulligan { player, .. } = engine.pending().clone() else {
                panic!("expected a mulligan")
            };
            engine.apply(player, PlayerAction::MulliganKeep).unwrap();
        }
        let view = player_view(engine.state(), PlayerId::new(0), None, 1, None, false, None);

        let identity = |card: CardIndex| {
            baylee_cards::by_index(card)
                .expect("a card at that index")
                .color_identity
        };
        // Katara is `{G}{W}{U}` and Elesh Norn `{4}{W}`, so the union is
        // Katara's — which is worth saying out loud, because it means this
        // fixture cannot show a projection that reads only the first
        // commander. What it can show is the exact answer and its opposite:
        // three named colours, and not the five a constant would give.
        let both = identity(katara()).union(identity(elesh_norn()));
        let projected = projection(&view, "Command Tower").expect("a commander game");
        assert_eq!(
            projected.colors,
            vec![ManaColor::White, ManaColor::Blue, ManaColor::Green],
            "one mana colour per colour of the seat's commanders together"
        );
        assert_eq!(projected.colors.len(), both.iter().count());
        assert!(
            projected.colors.len() < 5,
            "not every commander is five colours, and a Tower that said so \
             would tap for a red the engine then refuses"
        );
    }

    /// The agreement test, and the one that makes the rest worth anything.
    /// The projection is a promise about what the engine will offer, so a
    /// colour list that disagreed with the engine's own `ChooseColor` would
    /// be a land the planner taps and a payment that then fails — worse than
    /// saying nothing. Both sides are the same function by construction;
    /// this is what stops that staying true only by construction.
    #[test]
    fn the_projected_colors_are_the_ones_the_engine_then_offers() {
        let (mut engine, view) = board(&[reflecting_pool(), forest(), plains()], &[]);
        let pool = view
            .battlefield
            .iter()
            .find(|o| o.name == "Reflecting Pool")
            .expect("the Pool is on the battlefield");
        let projected = pool
            .board_mana
            .as_ref()
            .expect("a board to read")
            .colors
            .clone();

        for _ in 0..30 {
            let Pending::Priority { player, legal } = engine.pending().clone() else {
                panic!("expected priority")
            };
            if player != PlayerId::new(0) {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
                continue;
            }
            assert!(
                legal.abilities.contains(&(pool.id, 0)),
                "the engine offers the ability the projection is about"
            );
            engine
                .apply(
                    player,
                    PlayerAction::ActivateAbility {
                        source: pool.id,
                        ability_index: 0,
                    },
                )
                .unwrap();
            let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
                panic!("a Pool with two colours beside it asks which one")
            };
            assert_eq!(
                options, projected,
                "the view promised what the engine then offered"
            );
            return;
        }
        panic!("seat 0 never got priority");
    }
}
