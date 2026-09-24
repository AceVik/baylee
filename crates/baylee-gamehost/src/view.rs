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
use baylee_core::ids::{ObjectId, PlayerId, SeatSet};
use baylee_core::mana::ManaCost;
use baylee_engine::choice::Pending;
use baylee_engine::event::LossReason;
use baylee_engine::object::{GameObject, ObjectKind, PrintedFace};
use baylee_engine::state::GameState;
use baylee_engine::turn::{DayNight as EngineDayNight, Phase as EnginePhase, Step as EngineStep};
use baylee_engine::zone::{Zone, ZoneLocation};
use baylee_view::{
    AttackerView, BlockerView, CardIdentity, CombatView, CommanderDamage, CommanderView,
    CounterEntry, CounterKind, DayNight, GameStatic, HandObject, HouseAnswer, LossCause,
    ObjectStatus, Phase, PlayerView, PublicObject, RulesFace, SeatView, Step, TargetRef,
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

/// Translates the engine's reason a seat lost into the wire enum.
const fn loss_cause(reason: LossReason) -> LossCause {
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
        rules: obj.printed_face().filter(|_| known).map(rules_face),
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
        // The projected keywords, and on the stack "can't be countered" as
        // well when only a rider says so: the Cavern of Souls mana the spell
        // was paid with. The bit comes from the predicate the counter itself
        // asks, and only on the stack, since the rider belongs to the spell
        // and not to the permanent it becomes (#243).
        keywords: chars.keywords.bits()
            | if obj.zone == Zone::Stack && !obj.can_be_countered() {
                baylee_cards::dsl::KeywordSet::UNCOUNTERABLE.bits()
            } else {
                0
            },
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
        //
        // A second instance of the word "target" is drawn on the same list:
        // an arrow is an arrow, and a fight's two creatures are both what the
        // spell points at. Which instance each came from is the engine's
        // business at resolution and nobody's on the table, so the list
        // stays one field and the view keeps its version.
        targets: obj
            .targets
            .iter()
            .map(|t| TargetRef::Object(*t))
            .chain(obj.target_players.iter().map(TargetRef::Player))
            .chain(obj.second_targets().iter().map(|t| TargetRef::Object(*t)))
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
        // The same two facts `casting::can_cast` asks before it lets the
        // card off the graveyard, so the view never says "castable" of a
        // card the engine would refuse for being somewhere else. The price
        // is the card's own mana cost, as the cast charges it.
        flashback: (obj.zone == Zone::Graveyard
            && obj.zone_owner == Some(seat)
            && baylee_engine::casting::flashback_granted(state, id))
        .then_some(chars.mana_cost),
        // Permanents only, for `granted_mana`'s reason: the engine offers a
        // granted ability on a permanent and nowhere else, and each entry
        // here is one of those offers.
        grants: if obj.kind == ObjectKind::Permanent {
            grants(state, id, seat)
        } else {
            Vec::new()
        },
    })
}

/// Who granted each of `id`'s granted activated abilities, in the slot order
/// the engine offers them (#212).
///
/// The offer's own walk — `effects::granted_activated`, capped at
/// `GRANTED_SLOTS` — so entry `n` is the ability offered as
/// `granted_ability(n)`, and neither can be renumbered without the other.
///
/// A grantor is named only where this seat may see it: in a zone the view
/// sends ([`shown_elsewhere`]) and face up ([`may_know_card`]). A nontoken
/// card keeps its handle across zones (#240), so the handle of a grantor
/// since returned to a hand or shuffled into a library would say which card
/// in that hidden zone it is. That entry is all `None`, the same as a grant
/// from an emblem or a rule, which has no grantor at all.
fn grants(state: &GameState, id: ObjectId, seat: PlayerId) -> Vec<baylee_view::GrantSource> {
    baylee_engine::effects::granted_activated(state, id)
        .take(baylee_engine::choice::GRANTED_SLOTS as usize)
        .map(|granted| {
            let Some(grantor) = granted
                .source
                .and_then(|source| state.object(source))
                .filter(|o| shown_elsewhere(o, seat) && may_know_card(o, seat))
            else {
                return baylee_view::GrantSource::default();
            };
            let grant = baylee_cards_dsl::Modifier::GrantActivated {
                cost: granted.cost,
                effects: granted.effects,
                mana_ability: granted.mana_ability,
            };
            let home = grant_home(grantor, &grant);
            baylee_view::GrantSource {
                source: Some(grantor.id),
                rules: home.map(|(face, _)| rules_face(face)),
                text: home.and_then(|(face, index)| stack_text(face, index)),
            }
        })
        .collect()
}

/// Which printed face of `grantor` writes `grant`, and which of that face's
/// abilities it is.
///
/// First the abilities the grantor has (its own, or the copied card's),
/// then, only if none of those wrote it, every face of the card it
/// physically is. The second is for a copy clause: Machine God's Effigy
/// copying something has that thing's abilities, and the `…except it has
/// "{T}: Add {U}."` that granted this is printed on the Effigy. It is also
/// the face a transformed card showed when it wrote the grant.
///
/// Both are asked by value ([`baylee_cards::lines::grant_home`]), so a card
/// that wrote nothing equal to it answers `None` and no sentence is
/// guessed: an Opt in a graveyard named as the source of a grant answers
/// `None`, not its scry.
fn grant_home(
    grantor: &GameObject,
    grant: &baylee_cards_dsl::Modifier,
) -> Option<(PrintedFace, u32)> {
    let found = |face: PrintedFace, abilities| {
        let index = baylee_cards::lines::grant_home(abilities, grant)?;
        Some((face, u32::try_from(index).ok()?))
    };
    let own = grantor
        .printed_face()
        .and_then(|face| found(face, grantor.abilities(&crate::session::RegistryLookup)));
    own.or_else(|| {
        let card = grantor.card?.index;
        let def = baylee_cards::by_index(card)?;
        (0..def.faces.len()).find_map(|face| {
            let printed = PrintedFace::new(card, u8::try_from(face).ok()?)?;
            found(printed, def.abilities_for_face(face))
        })
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
        // `tap_only` and not merely `mana_ability`: a `GrantedMana` has no
        // field for a price, so a grant that charges more than `{T}` —
        // Forgotten Monument sells its Caves a colour for `{T}` and a life —
        // can only be reported as free, and a planner that believed it would
        // tap the land and fail the payment. The ability is still offered in
        // `LegalActions`, so nothing is taken away from the player; what is
        // withheld is a claim the view cannot make honestly.
        .filter(|(_, g)| g.mana_ability && baylee_cards_dsl::tap_only(&g.cost))
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
            text: obj
                .printed_face()
                .and_then(|printed| stack_text(printed, loc.index)),
            rules: obj.printed_face().map(rules_face),
        }),
        _ => None,
    }
}

/// The view's spelling of the engine's [`PrintedFace`].
fn rules_face(face: PrintedFace) -> RulesFace {
    RulesFace {
        card: face.card(),
        face: face.face(),
    }
}

/// Where an ability's printed sentence is, for a client holding the card's
/// text in the player's own language.
///
/// Read against the card the ability is *printed on*, which the engine
/// carried beside the list the ability took with it (`GameObject::own_face`)
/// — not the source's card, which for a copy is the wrong card, and not the
/// source's current face, which a transform may have turned since
/// (CR 113.7a). It used to be recovered from the list's address, and a copy
/// answered nothing because no face of the physical card matched; the
/// address was also never an identity, since two cards with byte-identical
/// lists share one.
///
/// Answers `None` for everything the generated table has no row for — a
/// reserved index (`AbilityRef::SPELL`, `SYNTHETIC`, …), a static ability,
/// a printed one no sentence fits — which is the whole point of it being
/// an `Option` on the wire. `docs/client.md` §"Which ability is on the
/// stack" is normative.
fn stack_text(printed: PrintedFace, index: u32) -> Option<baylee_view::StackText> {
    let line =
        baylee_cards::lines::ability_line(printed.card(), usize::from(printed.face()), index)?;
    Some(baylee_view::StackText {
        face: printed.face(),
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
        Pending::Arrange { cards, .. } => cards.as_slice(),
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
        engine.pending().asked()
    } else {
        deciding.contains(seat).then_some(seat)
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
pub struct SeatContext {
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
    pub owed: Option<ManaCost>,
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
}

/// This seat's own hand, which is the one hand a view spells out.
///
/// Lifted out of [`player_view`] only for its length; it is the same walk it
/// always was. Every *other* seat's hand is a count, and that asymmetry is
/// the point — see the crate docs on hidden information.
fn own_hand(state: &GameState, seat: PlayerId) -> Vec<HandObject> {
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
                mana_value: chars.mana_cost.cmc(),
                colors: chars.colors,
                types: chars.types,
                commander: is_commander(state, *id),
            })
        })
        .collect()
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
    let hand = own_hand(state, seat);

    PlayerView {
        seq,
        seat,
        turn: state.turn.number,
        phase: phase(state.turn.phase),
        step: step(state.turn.step),
        active: state.turn.active,
        awaiting: ctx.awaiting,
        deciding: ctx.deciding,
        decision_remaining_ms: ctx.decision_remaining_ms,
        priority_held: ctx.held,
        owed: ctx.owed,
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
                loss: p.loss.map(loss_cause),
                house_answered: house_answered.get(p.id.get() as usize).copied().flatten(),
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
        let locked = player_view(
            engine.state(),
            PlayerId::new(0),
            0,
            None,
            &SeatContext::default(),
            &[],
        );
        let theirs = player_view(
            engine.state(),
            PlayerId::new(1),
            0,
            None,
            &SeatContext::default(),
            &[],
        );

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
        let view = player_view(engine.state(), seat, 0, None, &SeatContext::default(), &[]);

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
            &preset.house_rules,
        );
        assert_eq!(statics.prints.len(), 3);
        let entry = |i: u16| statics.print(PrintRef::new(i)).expect("shown");
        assert_eq!(entry(1).lang, "DE");
        assert!(matches!(entry(1).finish, baylee_view::Finish::Foil));
        assert!(matches!(entry(2).finish, baylee_view::Finish::Etched));
        assert_eq!(statics.view_version, baylee_view::VIEW_VERSION);
    }

    /// The house rules spell *no limit* as zero and the payload spells it as
    /// `None`, because zero on a seat sheet reads as the opposite: a table
    /// with no time at all rather than one with all the time there is.
    ///
    /// Unreachable through the gateway, which refuses a zero reconnect
    /// window (`clock::MIN_RECONNECT_SECS` is 10) — but a local harness may
    /// choose either, and this payload has to be honest about a table the
    /// gateway did not make.
    #[test]
    fn a_table_with_no_limit_says_none_rather_than_nought() {
        let mut preset = mixed_print_preset();
        preset.house_rules.decision_timeout_secs = 0;
        preset.house_rules.reconnect_window_secs = 0;
        let statics = game_static(
            "g1".into(),
            PlayerId::new(0),
            vec![],
            &preset.prints,
            &[true, true, true],
            &preset.house_rules,
        );
        assert_eq!(statics.decision_secs, None);
        assert_eq!(statics.reconnect_secs, None);

        preset.house_rules.decision_timeout_secs = 30;
        preset.house_rules.reconnect_window_secs = 45;
        let statics = game_static(
            "g1".into(),
            PlayerId::new(0),
            vec![],
            &preset.prints,
            &[true, true, true],
            &preset.house_rules,
        );
        assert_eq!(statics.decision_secs, Some(30));
        assert_eq!(
            statics.reconnect_secs,
            Some(45),
            "the two limits are separate numbers and must not be read from one field"
        );
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
            &preset.house_rules,
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
        let view = player_view(
            engine.state(),
            PlayerId::new(0),
            1,
            None,
            &SeatContext::default(),
            &[],
        );
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

        let view = player_view(engine.state(), seat, 1, None, &SeatContext::default(), &[]);
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
        let view = player_view(engine.state(), me, 1, None, &SeatContext::default(), &[]);

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
            let view = player_view(engine.state(), seat, 1, None, &SeatContext::default(), &[]);
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

    /// A two-seat table dealt seven cards each, with the mulligans open.
    fn a_table_deciding_its_mulligans() -> Engine<Registry> {
        let mut preset = mixed_print_preset();
        for seat in &mut preset.seats {
            seat.starting_hand = None;
            seat.starting_battlefield = vec![];
        }
        Engine::new(&preset, Registry).expect("game starts")
    }

    /// `seat`'s view, with its context built the way a session builds it.
    fn seen_by(engine: &Engine<Registry>, seat: PlayerId) -> baylee_view::PlayerView {
        let ctx = SeatContext {
            awaiting: awaiting_for(engine, seat),
            deciding: deciding(engine),
            ..SeatContext::default()
        };
        player_view(engine.state(), seat, 1, engine.pending_for(seat), &ctx, &[])
    }

    /// Before turn 1 every seat is asked its own mulligan at once (#257), so
    /// a view waits on its own seat while that seat is deciding, and on
    /// nobody once it has kept. From turn 1 on every view waits on the same
    /// seat again, and `deciding` is empty: it is the window.
    #[test]
    fn during_the_mulligans_each_view_waits_on_its_own_seat() {
        use baylee_engine::choice::PlayerAction;

        let mut engine = a_table_deciding_its_mulligans();
        let (me, them) = (PlayerId::new(0), PlayerId::new(1));
        let both: SeatSet = [me, them].into_iter().collect();
        for seat in [me, them] {
            let view = seen_by(&engine, seat);
            assert_eq!(view.awaiting, Some(seat));
            assert_eq!(view.deciding, both);
        }

        engine.apply(me, PlayerAction::MulliganKeep).unwrap();
        let mine = seen_by(&engine, me);
        let theirs = seen_by(&engine, them);
        assert_eq!(mine.awaiting, None, "a seat that has kept is asked nothing");
        assert_eq!(theirs.awaiting, Some(them));
        let still: SeatSet = [them].into_iter().collect();
        assert_eq!((mine.deciding, theirs.deciding), (still, still));

        engine.apply(them, PlayerAction::MulliganKeep).unwrap();
        let asked = engine.pending().asked();
        assert!(asked.is_some(), "turn 1 asks somebody");
        for seat in [me, them] {
            let view = seen_by(&engine, seat);
            assert_eq!(view.deciding, SeatSet::new(), "the window has closed");
            assert_eq!(view.awaiting, asked, "every view waits on the same seat");
        }
    }

    /// Another seat's mulligans reach this seat's view as `deciding` and as
    /// that seat's counts, and as nothing else: not the question it is
    /// answering, not the hands it drew, not the cards it put on the bottom.
    #[test]
    fn another_seats_mulligan_shows_only_as_deciding_and_its_counts() {
        use baylee_engine::choice::PlayerAction;

        let mut engine = a_table_deciding_its_mulligans();
        let (me, them) = (PlayerId::new(0), PlayerId::new(1));
        let before = seen_by(&engine, me);

        // Two takes: a new hand each time, and a bottom owed however the
        // first mulligan is priced.
        engine.apply(them, PlayerAction::MulliganTake).unwrap();
        engine.apply(them, PlayerAction::MulliganTake).unwrap();
        assert_eq!(seen_by(&engine, me), before, "a take shows nothing here");
        engine.apply(them, PlayerAction::MulliganKeep).unwrap();
        let Some(Pending::MulliganBottom { count, .. }) = engine.pending_for(them).cloned() else {
            panic!("two takes owe a bottom");
        };
        assert_eq!(
            seen_by(&engine, me),
            before,
            "their bottom question shows nothing here"
        );

        let hand = engine.state().zones.list(ZoneLocation::Hand(them)).clone();
        let objects = hand.iter().take(usize::from(count)).copied().collect();
        engine
            .apply(them, PlayerAction::ChooseObjects { objects })
            .unwrap();
        let mut after = seen_by(&engine, me);
        let line = after.seat(them).expect("their seat line");
        assert_eq!(line.hand_count, 7 - u32::from(count));
        assert_eq!(after.deciding, [me].into_iter().collect());
        // Put back the three things that may differ, and nothing else did.
        after.deciding = before.deciding;
        after.seats[1].hand_count = before.seats[1].hand_count;
        after.seats[1].library_count = before.seats[1].library_count;
        assert_eq!(after, before);
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

        let mine = player_view(engine.state(), me, 1, None, &SeatContext::default(), &[]);
        let theirs = player_view(engine.state(), them, 1, None, &SeatContext::default(), &[]);
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

        let view = player_view(
            engine.state(),
            seat,
            0,
            Some(&pending),
            &SeatContext::default(),
            &[],
        );
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
            0,
            Some(&pending),
            &SeatContext::default(),
            &[],
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

        let view = player_view(engine.state(), seat, 0, None, &SeatContext::default(), &[]);
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

        let view = player_view(
            engine.state(),
            seat,
            0,
            Some(&pending),
            &SeatContext::default(),
            &[],
        );
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

        let view = player_view(
            engine.state(),
            seat,
            0,
            Some(&pending),
            &SeatContext::default(),
            &[],
        );
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

        let view = player_view(
            engine.state(),
            seat,
            0,
            Some(&pending),
            &SeatContext::default(),
            &[],
        );
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

        let view = player_view(
            engine.state(),
            me,
            0,
            Some(&pending),
            &SeatContext::default(),
            &[],
        );
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
        let theirs = player_view(
            engine.state(),
            them,
            0,
            Some(&pending),
            &SeatContext::default(),
            &[],
        );
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
        let view = player_view(
            engine.state(),
            PlayerId::new(0),
            1,
            None,
            &SeatContext::default(),
            &[],
        );

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

    fn chromatic_lantern() -> CardIndex {
        by_oracle_id("539f5396-d99a-417d-a84c-dff7930b5900")
            .expect("Chromatic Lantern is in the pool")
            .index
    }

    fn machine_gods_effigy() -> CardIndex {
        by_oracle_id("64ebdd6f-acde-4aab-a86b-2798bad5f70c")
            .expect("Machine God's Effigy is in the pool")
            .index
    }

    /// The first grant `card`'s front face writes, and which of its
    /// abilities writes it.
    fn first_grant(card: CardIndex) -> (u32, baylee_cards_dsl::Modifier) {
        let abilities = baylee_cards::by_index(card)
            .expect("in the pool")
            .abilities_for_face(0);
        abilities
            .iter()
            .enumerate()
            .find_map(|(index, ability)| {
                let (_, grant) = baylee_cards::lines::grants_in(ability, &mut 0)
                    .into_iter()
                    .next()?;
                Some((u32::try_from(index).ok()?, *grant))
            })
            .expect("the card grants an ability")
    }

    /// Makes `source` grant `grant` to `target`, as an effect that resolved
    /// would, through the dev door the test preset opens.
    fn grant_from(
        engine: &mut Engine<Registry>,
        source: ObjectId,
        target: ObjectId,
        grant: baylee_cards_dsl::Modifier,
        timestamp: u64,
    ) {
        let state = engine
            .dev_state_mut(PlayerId::new(0))
            .expect("the test preset grants dev commands");
        let filter = baylee_engine::effects::EffectFilter::object(state, target);
        state
            .effects
            .register(baylee_engine::effects::ContinuousEffect {
                id: baylee_core::ids::EffectId::new(0),
                source: Some(source),
                controller: PlayerId::new(0),
                layer: grant.layer(),
                timestamp,
                duration: baylee_cards_dsl::Duration::Indefinitely,
                filter,
                modifier: grant,
            });
    }

    /// The one object of `card` in `zone`.
    fn lying_in(state: &GameState, zone: ZoneLocation, card: Option<CardIndex>) -> ObjectId {
        state
            .zones
            .list(zone)
            .iter()
            .copied()
            .find(|&id| {
                card.is_none_or(|card| {
                    state
                        .object(id)
                        .and_then(|o| o.card)
                        .is_some_and(|c| c.index == card)
                })
            })
            .unwrap_or_else(|| panic!("nothing in {zone:?}"))
    }

    /// What `seat` is told about who granted `permanent`'s abilities.
    fn grants_seen(
        engine: &Engine<Registry>,
        seat: PlayerId,
        permanent: ObjectId,
    ) -> Vec<baylee_view::GrantSource> {
        player_view(engine.state(), seat, 0, None, &SeatContext::default(), &[])
            .battlefield
            .iter()
            .find(|o| o.id == permanent)
            .expect("on the shared battlefield")
            .grants
            .clone()
    }

    /// #212. A land under a Chromatic Lantern taps for an ability printed on
    /// no card it has, and the view says whose sentence it is: the Lantern,
    /// its front face, and the line of it that grants the `{T}`. A client
    /// then draws the Lantern's words on the land's row, in the player's
    /// language, instead of a label of its own.
    ///
    /// Both seats are told, since the Lantern is on the battlefield. The
    /// opponent's land and the Lantern itself are granted nothing: the grant
    /// is "lands *you* control", and the Lantern's own `{T}` is printed on it.
    #[test]
    fn a_granted_ability_names_its_grantor_and_the_sentence_that_grants_it() {
        let card = |card| DeckEntry {
            card,
            print: PrintRef::new(0),
        };
        let (me, them) = (PlayerId::new(0), PlayerId::new(1));
        let mut preset = mixed_print_preset();
        preset.seats[0].starting_battlefield = vec![card(island()), card(chromatic_lantern())];
        preset.seats[1].starting_battlefield = vec![card(island())];
        let mut engine = Engine::new(&preset, Registry).expect("game starts");
        settle(&mut engine, None);

        let state = engine.state();
        let lantern = lying_in(state, ZoneLocation::Battlefield, Some(chromatic_lantern()));
        let land_of = |seat: PlayerId| {
            state
                .zones
                .list(ZoneLocation::Battlefield)
                .iter()
                .copied()
                .find(|&id| {
                    state.object(id).is_some_and(|o| {
                        o.controller == seat && o.card.is_some_and(|c| c.index == island())
                    })
                })
                .expect("each seat has its Island")
        };
        let (mine, theirs) = (land_of(me), land_of(them));

        let named = vec![baylee_view::GrantSource {
            source: Some(lantern),
            rules: Some(baylee_view::RulesFace {
                card: chromatic_lantern(),
                face: 0,
            }),
            text: Some(baylee_view::StackText {
                face: 0,
                line: 0,
                of: 2,
            }),
        }];
        assert_eq!(grants_seen(&engine, me, mine), named);
        assert_eq!(
            grants_seen(&engine, them, mine),
            named,
            "the Lantern is public, so the opponent is told the same"
        );
        assert!(grants_seen(&engine, me, theirs).is_empty());
        assert!(grants_seen(&engine, me, lantern).is_empty());
    }

    /// #212, the hidden-information half. A grantor that lies where this
    /// seat cannot look is not named at all.
    ///
    /// A nontoken card keeps its handle across zones (#240), so the handle
    /// would say which card in that hand or library granted it, and the
    /// sentence would say what the card is. Seat 1's own hand is a place
    /// seat 1 may look, so it is told about its Lantern; nobody is told about
    /// a card in a library. The Lantern is not even asked for text on seat
    /// 0's behalf ([`PlayerView::cards`]).
    #[test]
    fn a_grantor_this_seat_cannot_see_is_not_named() {
        let card = |card| DeckEntry {
            card,
            print: PrintRef::new(0),
        };
        let (me, them) = (PlayerId::new(0), PlayerId::new(1));
        let mut preset = mixed_print_preset();
        preset.seats[0].starting_battlefield = vec![card(island())];
        preset.seats[1].starting_hand = Some(vec![card(chromatic_lantern())]);
        let mut engine = Engine::new(&preset, Registry).expect("game starts");
        settle(&mut engine, None);

        let state = engine.state();
        let land = lying_in(state, ZoneLocation::Battlefield, Some(island()));
        let in_their_hand = lying_in(state, ZoneLocation::Hand(them), Some(chromatic_lantern()));
        let in_my_library = lying_in(state, ZoneLocation::Library(me), None);
        let (_, grant) = first_grant(chromatic_lantern());
        grant_from(&mut engine, in_their_hand, land, grant, 1_000);
        grant_from(&mut engine, in_my_library, land, grant, 1_001);

        assert_eq!(
            grants_seen(&engine, me, land),
            vec![baylee_view::GrantSource::default(); 2],
            "seat 0 may see neither grantor"
        );
        let mine = player_view(engine.state(), me, 0, None, &SeatContext::default(), &[]);
        assert!(
            !mine.cards().any(|c| c == chromatic_lantern()),
            "and is not handed the Lantern's text by another door"
        );
        assert_eq!(
            grants_seen(&engine, them, land),
            vec![
                baylee_view::GrantSource {
                    source: Some(in_their_hand),
                    rules: Some(baylee_view::RulesFace {
                        card: chromatic_lantern(),
                        face: 0,
                    }),
                    text: Some(baylee_view::StackText {
                        face: 0,
                        line: 0,
                        of: 2,
                    }),
                },
                baylee_view::GrantSource::default(),
            ],
            "seat 1 holds the Lantern; nobody may look into seat 0's library"
        );
    }

    /// #212. A grant whose source wrote nothing equal to it names the source
    /// and no sentence: there is no nearest match.
    ///
    /// The source is an Opt that has resolved into its owner's graveyard,
    /// the shape of a spell that granted something until end of turn and
    /// is gone. A graveyard is public, so the Opt is named. Opt writes no
    /// grant at all, so a lookup that fell back to some sentence would print
    /// Opt's scry on the land's row, and one that panicked on a spell would
    /// take the game down.
    #[test]
    fn a_grant_its_source_never_wrote_names_no_sentence() {
        let card = |card| DeckEntry {
            card,
            print: PrintRef::new(0),
        };
        let (me, them) = (PlayerId::new(0), PlayerId::new(1));
        let mut preset = mixed_print_preset();
        preset.seats[0].starting_hand = Some(vec![card(opt())]);
        preset.seats[0].starting_battlefield = vec![card(island()); 2];
        let mut engine = Engine::new(&preset, Registry).expect("game starts");
        let view = settle(&mut engine, None);
        let opt = view
            .hand
            .iter()
            .find(|c| c.name == "Opt")
            .expect("Opt in hand")
            .id;
        let islands: Vec<ObjectId> = view.battlefield_of(me).map(|o| o.id).collect();
        engine
            .apply(me, PlayerAction::ActivateManaAbility { source: islands[0] })
            .expect("an Island taps for blue");
        engine
            .apply(me, PlayerAction::CastSpell { card: opt })
            .expect("the mana for it is floating");
        settle(&mut engine, None);
        assert_eq!(
            engine.state().object(opt).map(|o| o.zone),
            Some(Zone::Graveyard),
            "Opt resolved"
        );

        let (_, grant) = first_grant(chromatic_lantern());
        grant_from(&mut engine, opt, islands[1], grant, 1_000);
        let named_only = vec![baylee_view::GrantSource {
            source: Some(opt),
            rules: None,
            text: None,
        }];
        assert_eq!(grants_seen(&engine, me, islands[1]), named_only);
        assert_eq!(grants_seen(&engine, them, islands[1]), named_only);
    }

    /// #212. A copy's own grant is read off the card it physically is.
    ///
    /// Machine God's Effigy enters as a copy of an artifact "except it has
    /// `{T}: Add {U}`". That clause is printed on the Effigy, while every
    /// ability the permanent has is the copied card's. So the grant's
    /// sentence is the Effigy's even though the permanent's own `rules` is
    /// the copied Lantern's: the two fields are separate for exactly this.
    #[test]
    fn a_copys_own_grant_is_read_off_the_card_it_is() {
        let card = |card| DeckEntry {
            card,
            print: PrintRef::new(0),
        };
        let me = PlayerId::new(0);
        let mut preset = mixed_print_preset();
        preset.seats[0].starting_battlefield = vec![card(machine_gods_effigy())];
        let mut engine = Engine::new(&preset, Registry).expect("game starts");
        settle(&mut engine, None);

        let effigy = lying_in(
            engine.state(),
            ZoneLocation::Battlefield,
            Some(machine_gods_effigy()),
        );
        let lantern = baylee_cards::by_index(chromatic_lantern()).expect("in the pool");
        engine
            .dev_state_mut(me)
            .expect("the test preset grants dev commands")
            .object_mut(effigy)
            .expect("the Effigy is there")
            .take_abilities(baylee_engine::object::AbilityList {
                abilities: lantern.abilities_for_face(0),
                printed: PrintedFace::new(chromatic_lantern(), 0),
            });
        let (clause, grant) = first_grant(machine_gods_effigy());
        grant_from(&mut engine, effigy, effigy, grant, 1_000);

        let view = player_view(engine.state(), me, 0, None, &SeatContext::default(), &[]);
        let copy = view
            .battlefield
            .iter()
            .find(|o| o.id == effigy)
            .expect("the Effigy");
        assert_eq!(
            copy.rules,
            Some(baylee_view::RulesFace {
                card: chromatic_lantern(),
                face: 0,
            }),
            "the copy's abilities are the Lantern's"
        );
        let line = baylee_cards::lines::ability_line(machine_gods_effigy(), 0, clause)
            .expect("the copy clause has a line");
        assert_eq!(
            copy.grants,
            vec![baylee_view::GrantSource {
                source: Some(effigy),
                rules: Some(baylee_view::RulesFace {
                    card: machine_gods_effigy(),
                    face: 0,
                }),
                text: Some(baylee_view::StackText {
                    face: 0,
                    line: line.line,
                    of: line.of,
                }),
            }]
        );
    }

    /// Forgotten Monument grants its other Caves `{T}`, pay 1 life: add one
    /// mana of any colour — and the view says **nothing** about it, on
    /// purpose.
    ///
    /// [`baylee_view::GrantedMana`] carries a slot, the colours and the
    /// amount and has no field for a price, because an ability printed on no
    /// card has nowhere else to put one. Reporting this grant would therefore
    /// tell a planner it may tap the Cave for free; it would tap it, and the
    /// life would never be offered to pay. Saying nothing costs the planner
    /// one land and costs the player nothing at all, which is the asymmetry
    /// `baylee_cards_dsl::tap_only` encodes.
    ///
    /// The second assertion is what keeps the first from being a way to hide
    /// a broken grant: the engine still **offers** the ability under
    /// `GRANTED_ABILITY`, so what the view withholds is a claim, not the
    /// player's button.
    #[test]
    fn granted_mana_refuses_a_priced_grant() {
        use baylee_engine::choice::{Pending, PlayerAction};

        let monument = by_oracle_id("71393988-ad6f-43fd-9978-c0de15ae8e87")
            .expect("Forgotten Monument is in the pool")
            .index;
        let maw = by_oracle_id("952ab8fe-f7d3-4673-89de-8c6d3f8a081f")
            .expect("Cavernous Maw is in the pool")
            .index;
        let mut preset = mixed_print_preset();
        preset.seats[0].starting_battlefield = vec![
            DeckEntry {
                card: maw,
                print: PrintRef::new(0),
            },
            DeckEntry {
                card: monument,
                print: PrintRef::new(0),
            },
        ];

        let mut engine = Engine::new(&preset, Registry).expect("game starts");
        for _ in 0..2 {
            let Pending::Mulligan { player, .. } = engine.pending().clone() else {
                panic!("expected a mulligan")
            };
            engine.apply(player, PlayerAction::MulliganKeep).unwrap();
        }
        let view = player_view(
            engine.state(),
            PlayerId::new(0),
            1,
            None,
            &SeatContext::default(),
            &[],
        );
        let cave = view
            .battlefield
            .iter()
            .find(|o| o.card.is_some_and(|c| c.index == maw))
            .expect("the Cave is on the battlefield");
        assert!(
            cave.granted_mana.is_none(),
            "the grant charges a life beside its {{T}} and `GrantedMana` has \
             nowhere to say so, so the view must withhold it rather than \
             report mana the engine will not hand over for nothing"
        );

        for _ in 0..30 {
            let Pending::Priority { player, legal } = engine.pending().clone() else {
                break;
            };
            if player == PlayerId::new(0) {
                assert!(
                    legal
                        .abilities
                        .contains(&(cave.id, baylee_engine::choice::GRANTED_ABILITY)),
                    "the engine still offers the granted ability the view \
                     declined to describe: {:?}",
                    legal.abilities
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
                0,
                None,
                &SeatContext::default(),
                &[],
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
            let view = player_view(
                &state,
                PlayerId::new(seat),
                0,
                None,
                &SeatContext::default(),
                &[],
            );
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
        let view = player_view(
            engine.state(),
            PlayerId::new(0),
            0,
            None,
            &SeatContext::default(),
            &[],
        );

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

        let owner = player_view(
            &state,
            PlayerId::new(0),
            0,
            None,
            &SeatContext::default(),
            &[],
        );
        let held = owner
            .hand
            .iter()
            .find(|o| o.id == katara_obj)
            .expect("it is in the hand it was sent to");
        assert!(held.commander, "it is still a commander in a hand");

        let other = player_view(
            &state,
            PlayerId::new(1),
            0,
            None,
            &SeatContext::default(),
            &[],
        );
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

        let view = player_view(&state, seat, 0, None, &SeatContext::default(), &[]);
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

        let view = player_view(&state, seat, 0, None, &SeatContext::default(), &[]);
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
            let view = player_view(
                &state,
                PlayerId::new(seat),
                0,
                None,
                &SeatContext::default(),
                &[],
            );
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

    /// An ability on the stack, as `push_ability_to_stack` leaves one:
    /// taken from `source`'s card, carrying `list` printed on `printed`.
    fn stacked(
        state: &mut GameState,
        source_card: CardIndex,
        index: u32,
        list: baylee_engine::object::AbilityList,
    ) -> GameObject {
        let name = state.names.intern("ability");
        let base = state.bare_base(name);
        let mut obj = GameObject::new_ability_on_stack(
            ObjectId::new(900, 0),
            PlayerId::new(0),
            baylee_engine::object::AbilityLoc {
                card: Some(source_card),
                index,
                source: ObjectId::new(901, 0),
            },
            std::iter::empty().collect(),
            base,
        );
        obj.take_abilities(list);
        obj
    }

    /// A stack entry indexes the sentences of the face its ability was taken
    /// from. Sheoldred's back face is the only one in the pool that puts an
    /// ability on the stack; read against the face the source shows *now*,
    /// it would print the other side's sentence as precise text.
    #[test]
    fn a_stack_entry_names_the_face_its_ability_was_taken_from() {
        let engine = Engine::new(&commander_preset(), Registry).expect("game starts");
        let mut state = engine.state().clone();
        let sheoldred = baylee_cards::all()
            .find(|d| d.name() == "Sheoldred")
            .expect("the pool has Sheoldred");

        let back = PrintedFace::new(sheoldred.index, 1).expect("fits");
        let (index, line) = (0..sheoldred.abilities_for_face(1).len())
            .filter_map(|i| u32::try_from(i).ok())
            .find_map(|i| baylee_cards::lines::ability_line(sheoldred.index, 1, i).map(|l| (i, l)))
            .expect("the back face puts an ability on the stack");
        let own = stacked(
            &mut state,
            sheoldred.index,
            index,
            baylee_engine::object::AbilityList {
                abilities: sheoldred.abilities_for_face(1),
                printed: Some(back),
            },
        );
        let Some(baylee_view::StackItem::Ability { text, rules, .. }) = stack_item(&own) else {
            panic!("an ability on the stack is an ability");
        };
        assert_eq!(
            rules,
            Some(RulesFace {
                card: sheoldred.index,
                face: 1
            })
        );
        assert_eq!(
            text,
            Some(baylee_view::StackText {
                face: 1,
                line: line.line,
                of: line.of
            }),
            "the back face's sentence, whatever the source shows now"
        );
    }

    /// A copy's ability on the stack names the card it copied, and indexes
    /// that card's sentences — while the handle a standing answer is filed
    /// under stays the card on the table. A list no card prints (a token's)
    /// names nothing.
    #[test]
    fn a_copys_stack_entry_names_the_card_it_copied() {
        let engine = Engine::new(&commander_preset(), Registry).expect("game starts");
        let mut state = engine.state().clone();
        let solemn = baylee_cards::all()
            .find(|d| d.name() == "Solemn Simulacrum")
            .expect("the pool has Solemn Simulacrum");
        let spark_double = baylee_cards::all()
            .find(|d| d.name() == "Spark Double")
            .expect("the pool has Spark Double");

        let (index, line) = (0..solemn.abilities_for_face(0).len())
            .filter_map(|i| u32::try_from(i).ok())
            .find_map(|i| baylee_cards::lines::ability_line(solemn.index, 0, i).map(|l| (i, l)))
            .expect("Solemn's triggers have sentences");
        let copied = stacked(
            &mut state,
            spark_double.index,
            index,
            baylee_engine::object::AbilityList {
                abilities: solemn.abilities_for_face(0),
                printed: PrintedFace::new(solemn.index, 0),
            },
        );
        let Some(baylee_view::StackItem::Ability {
            ability,
            text,
            rules,
            ..
        }) = stack_item(&copied)
        else {
            panic!("an ability on the stack is an ability");
        };
        assert_eq!(
            ability.map(|a| a.card),
            Some(spark_double.index),
            "the handle stays the card on the table, which a standing answer is filed under"
        );
        assert_eq!(
            rules,
            Some(RulesFace {
                card: solemn.index,
                face: 0
            }),
            "but the ability is printed on the card it copied"
        );
        assert_eq!(
            text,
            Some(baylee_view::StackText {
                face: 0,
                line: line.line,
                of: line.of
            })
        );

        let token = stacked(
            &mut state,
            spark_double.index,
            0,
            baylee_engine::object::AbilityList {
                abilities: solemn.abilities_for_face(0),
                printed: None,
            },
        );
        let Some(baylee_view::StackItem::Ability { text, rules, .. }) = stack_item(&token) else {
            panic!("an ability on the stack is an ability");
        };
        assert_eq!(
            (text, rules),
            (None, None),
            "a list no card prints has no sentence to point at"
        );
    }

    /// `rules` is the card itself for everything that is not a copy, and is
    /// withheld exactly where `card` is: a face-down permanent's controller
    /// sees it, and nobody else does.
    #[test]
    fn an_object_names_its_own_card_unless_it_may_not_be_known() {
        let engine = Engine::new(&commander_preset(), Registry).expect("game starts");
        let mut state = engine.state().clone();
        let mut seen = 0;
        for seat in [0u8, 1] {
            let view = player_view(
                &state,
                PlayerId::new(seat),
                0,
                None,
                &SeatContext::default(),
                &[],
            );
            for object in view.battlefield.iter().chain(view.command.iter().flatten()) {
                let Some(card) = object.card else { continue };
                assert_eq!(
                    object.rules,
                    Some(RulesFace {
                        card: card.index,
                        face: card.face
                    }),
                    "{}",
                    object.name
                );
                seen += 1;
            }
        }
        assert!(seen > 0, "the preset puts cards where a view shows them");

        let hidden = state
            .zones
            .list(ZoneLocation::Battlefield)
            .first()
            .copied()
            .or_else(|| state.commanders[0].first().map(|c| c.object))
            .expect("some object to turn face down");
        let owner = state.object(hidden).expect("it exists").controller;
        state
            .object_mut(hidden)
            .expect("it exists")
            .status
            .insert(baylee_engine::object::Status::FACE_DOWN);
        let other = PlayerId::new(1 - owner.get());
        for (seat, entitled) in [(owner, true), (other, false)] {
            let view = player_view(&state, seat, 0, None, &SeatContext::default(), &[]);
            let Some(object) = view
                .battlefield
                .iter()
                .chain(view.command.iter().flatten())
                .find(|o| o.id == hidden)
            else {
                continue;
            };
            assert_eq!(object.card.is_some(), entitled);
            assert_eq!(
                object.rules.is_some(),
                entitled,
                "the card a face-down permanent's abilities are printed on is the card"
            );
        }
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
        let view = player_view(
            engine.state(),
            PlayerId::new(0),
            1,
            None,
            &SeatContext::default(),
            &[],
        );
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
        let view = player_view(
            engine.state(),
            PlayerId::new(0),
            1,
            None,
            &SeatContext::default(),
            &[],
        );

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

    /// A seat in the game carries no loss; a seat that is out carries the
    /// reason the engine recorded, and the view's `has_lost` reads it.
    #[test]
    fn a_lost_seat_carries_the_engines_reason_and_a_live_one_carries_none() {
        let preset = mixed_print_preset();
        let mut engine = Engine::new(&preset, Registry).expect("game starts");
        let (me, them) = (PlayerId::new(0), PlayerId::new(1));
        let view = player_view(engine.state(), me, 0, None, &SeatContext::default(), &[]);
        assert!(view.seats.iter().all(|s| s.loss.is_none() && !s.has_lost()));

        engine
            .apply(them, baylee_engine::choice::PlayerAction::Concede)
            .expect("a seated player may always concede");
        let view = player_view(engine.state(), me, 1, None, &SeatContext::default(), &[]);
        let seat = |p: PlayerId| view.seat(p).expect("seated");
        assert_eq!(seat(them).loss, Some(LossCause::Conceded));
        assert!(seat(them).has_lost());
        assert_eq!(seat(me).loss, None);
    }

    /// Each engine reason reaches the wire under its own name. The mapping is
    /// an exhaustive match, so a new reason cannot be forgotten; this is what
    /// catches two arms swapped.
    #[test]
    fn every_loss_reason_reaches_the_wire_as_itself() {
        for reason in [
            LossReason::Life,
            LossReason::EmptyDraw,
            LossReason::Poison,
            LossReason::CommanderDamage,
            LossReason::Conceded,
            LossReason::Effect,
        ] {
            assert_eq!(format!("{:?}", loss_cause(reason)), format!("{reason:?}"));
        }
    }

    /// Who answered for a seat is on that seat and no other, and a seat the
    /// host's record does not reach reads as having answered itself.
    #[test]
    fn a_house_answer_is_on_the_seat_the_host_names_and_no_other() {
        let preset = mixed_print_preset();
        let engine = Engine::new(&preset, Registry).expect("game starts");
        let (me, them) = (PlayerId::new(0), PlayerId::new(1));
        let ctx = SeatContext::default();
        let view = player_view(
            engine.state(),
            me,
            0,
            None,
            &ctx,
            &[None, Some(HouseAnswer::Clock)],
        );
        assert_eq!(
            view.seat(them).map(|s| s.house_answered),
            Some(Some(HouseAnswer::Clock))
        );
        assert_eq!(view.seat(me).map(|s| s.house_answered), Some(None));

        let short = player_view(
            engine.state(),
            me,
            0,
            None,
            &ctx,
            &[Some(HouseAnswer::StandIn)],
        );
        assert_eq!(
            short.seat(me).map(|s| s.house_answered),
            Some(Some(HouseAnswer::StandIn))
        );
        assert_eq!(short.seat(them).map(|s| s.house_answered), Some(None));
    }

    fn opt() -> CardIndex {
        by_oracle_id("713332c1-5bd8-400f-bfff-c1ca0697a043")
            .unwrap()
            .index
    }

    fn snapcaster_mage() -> CardIndex {
        by_oracle_id("2bb2eda7-3b38-4c56-870f-c3218a1056f5")
            .unwrap()
            .index
    }

    /// Answers everything until seat 0 holds priority in its own first main
    /// phase with the stack empty, and returns seat 0's view there. A target
    /// question gets `aim`, a scry keeps its card on top, nobody attacks, and
    /// a Cavern of Souls names Bird.
    fn settle(engine: &mut Engine<Registry>, aim: Option<ObjectId>) -> PlayerView {
        let me = PlayerId::new(0);
        for _ in 0..100 {
            let (player, action) = match engine.pending().clone() {
                Pending::Mulligan { player, .. } => (player, PlayerAction::MulliganKeep),
                Pending::Priority { player, .. } => {
                    let view =
                        player_view(engine.state(), me, 0, None, &SeatContext::default(), &[]);
                    if player == me
                        && view.stack.is_empty()
                        && view.active == me
                        && view.phase == Phase::FirstMain
                    {
                        return view;
                    }
                    (player, PlayerAction::PassPriority)
                }
                Pending::ChooseTargets { player, .. } => (
                    player,
                    PlayerAction::ChooseObjects {
                        objects: aim.into_iter().collect(),
                    },
                ),
                Pending::Arrange { player, cards, .. } => (
                    player,
                    PlayerAction::Arrange {
                        piles: vec![cards, vec![]],
                    },
                ),
                Pending::ChooseAttackers { player, .. } => {
                    (player, PlayerAction::DeclareAttackers { attackers: vec![] })
                }
                Pending::ChooseSubtype { player, .. } => (
                    player,
                    PlayerAction::ChooseSubtype(baylee_core::generated::subtypes::creature::BIRD),
                ),
                other => panic!("unexpected question: {other:?}"),
            };
            engine.apply(player, action).expect("a legal answer");
        }
        panic!("seat 0 never came back to its main phase");
    }

    /// What `seat` is told it may pay to cast `card` from the graveyard.
    fn flashback_for(
        engine: &Engine<Registry>,
        seat: PlayerId,
        card: ObjectId,
    ) -> Option<ManaCost> {
        let view = player_view(engine.state(), seat, 0, None, &SeatContext::default(), &[]);
        view.graveyards
            .iter()
            .flatten()
            .find(|o| o.id == card)
            .unwrap_or_else(|| panic!("seat {seat:?} sees the card in a graveyard"))
            .flashback
    }

    /// #242. A card this seat may cast from its graveyard says so, at the
    /// price the cast will charge, and says so to that seat alone.
    ///
    /// The engine names a graveyard spell in `LegalActions::castable` only
    /// once its cost is already floating, so a planner that walked its hand
    /// never tapped for one: Snapcaster Mage gave Opt flashback, and Opt
    /// stayed where it was beside an untapped Island. Played, not built:
    /// Opt is cast and resolves, then the Mage enters and targets it.
    ///
    /// The opponent sees the same Opt in the same graveyard and is told
    /// nothing, because it may not cast it (`casting::can_cast` asks for the
    /// caster's own graveyard); and nobody is told anything once the grant
    /// has ended with the turn.
    #[test]
    fn a_granted_flashback_is_shown_to_the_seat_that_may_cast_it() {
        let (me, them) = (PlayerId::new(0), PlayerId::new(1));
        let card = |card| DeckEntry {
            card,
            print: PrintRef::new(0),
        };
        let mut preset = mixed_print_preset();
        let printed = baylee_cards::by_index(opt()).expect("Opt").faces[0].mana_cost;
        preset.seats[0].starting_hand = Some(vec![card(opt()), card(snapcaster_mage())]);
        preset.seats[0].starting_battlefield = vec![card(island()); 3];
        let mut engine = Engine::new(&preset, Registry).expect("game starts");

        let view = settle(&mut engine, None);
        let in_hand = |name: &str| {
            view.hand
                .iter()
                .find(|c| c.name == name)
                .unwrap_or_else(|| panic!("{name} in hand"))
                .id
        };
        let (opt, mage) = (in_hand("Opt"), in_hand("Snapcaster Mage"));
        let islands: Vec<ObjectId> = view.battlefield_of(me).map(|o| o.id).collect();
        let cast = |engine: &mut Engine<Registry>, lands: &[ObjectId], card: ObjectId| {
            for &source in lands {
                engine
                    .apply(me, PlayerAction::ActivateManaAbility { source })
                    .expect("an Island taps for blue");
            }
            engine
                .apply(me, PlayerAction::CastSpell { card })
                .expect("the mana for it is floating");
        };

        cast(&mut engine, &islands[..1], opt);
        settle(&mut engine, None);
        assert_eq!(
            (
                flashback_for(&engine, me, opt),
                flashback_for(&engine, them, opt)
            ),
            (None, None),
            "Opt in the graveyard before the grant is castable by nobody"
        );

        cast(&mut engine, &islands[1..], mage);
        settle(&mut engine, Some(opt));
        assert_eq!(
            flashback_for(&engine, me, opt),
            Some(printed),
            "the Mage's grant costs Opt's own mana cost, and the owner is told"
        );
        assert_eq!(
            flashback_for(&engine, them, opt),
            None,
            "the opponent may not cast a card out of somebody else's graveyard"
        );

        engine.apply(me, PlayerAction::PassPriority).unwrap();
        settle(&mut engine, None);
        assert_eq!(
            flashback_for(&engine, me, opt),
            None,
            "until end of turn: seat 0's next main phase has no grant left"
        );
    }

    /// The other door to [`PublicObject::flashback`], pinned shut.
    ///
    /// A granted flashback costs the card's own mana cost, and that is the
    /// price the field carries. A printed one costs what the card prints —
    /// Faithless Looting is `{R}` and flashes back for `{2}{R}` — and no face
    /// in the pool has one written yet: each card that prints the keyword
    /// says so in its coverage. The day one is written, this goes red, and
    /// the projection has to learn the printed price first, or the view
    /// tells a planner the cast costs what the front of the card says.
    #[test]
    fn no_face_that_prints_flashback_has_it_written_yet() {
        let printed: Vec<(&str, bool)> = baylee_cards::all()
            .flat_map(|def| (0..def.faces.len()).map(move |face| (def, face)))
            .filter(|&(def, face)| {
                baylee_cards::oracle::face(def.index, face)
                    .is_some_and(|text| text.lines().any(|line| line.starts_with("Flashback")))
            })
            .map(|(def, _)| {
                (
                    def.name(),
                    matches!(def.coverage, baylee_cards::dsl::Coverage::Implemented),
                )
            })
            .collect();
        // Four on 24.09.2026: Faithless Looting, Memory Deluge, Past in
        // Flames, Sevinne's Reclamation. Floor and ceiling both, since a
        // misread line count reads as zero and an overbroad one as many.
        assert!(
            (4..=12).contains(&printed.len()),
            "{} faces print flashback: {printed:?}",
            printed.len()
        );
        let written: Vec<&str> = printed
            .iter()
            .filter(|(_, implemented)| *implemented)
            .map(|(name, _)| *name)
            .collect();
        assert!(
            written.is_empty(),
            "{written:?} print flashback and are implemented: the view prices a \
             graveyard cast at the card's mana cost, which is right for a \
             granted flashback only"
        );
    }

    fn cavern_of_souls() -> CardIndex {
        by_oracle_id("89ca686a-7c72-4d8f-9290-e89635624a83")
            .unwrap()
            .index
    }

    fn sea_eagle() -> CardIndex {
        by_oracle_id("acb57162-7093-4a3c-9818-d3b61ce757c6")
            .unwrap()
            .index
    }

    /// Whether `seat` is told the object can't be countered.
    fn uncounterable_to(engine: &Engine<Registry>, seat: PlayerId, id: ObjectId) -> bool {
        let view = player_view(engine.state(), seat, 0, None, &SeatContext::default(), &[]);
        let object = view.object(id).expect("the object is in public view");
        object.keywords & baylee_cards::dsl::KeywordSet::UNCOUNTERABLE.bits() != 0
    }

    /// #243. A spell that can't be countered says so on the stack, to every
    /// seat, even when no printed word makes it so.
    ///
    /// The printed "can't be countered" rode the projected keywords all
    /// along. The Cavern of Souls kind is a rider on the spell that no
    /// characteristic carries, so a creature cast with that mana looked
    /// counterable to every seat, the house AI among them. The bit is now
    /// set from `GameObject::can_be_countered`, the predicate the counter
    /// itself asks.
    ///
    /// The same card is cast twice, once off Islands and once off the Cavern,
    /// so only the mana differs between the two answers. The Cavern's Eagle
    /// then loses the bit once it is a permanent, because the rider belongs
    /// to the spell.
    #[test]
    fn a_spell_that_cannot_be_countered_says_so_on_the_stack() {
        let (me, them) = (PlayerId::new(0), PlayerId::new(1));
        let card = |card| DeckEntry {
            card,
            print: PrintRef::new(0),
        };
        let mut preset = mixed_print_preset();
        preset.seats[0].starting_hand = Some(vec![card(sea_eagle()), card(sea_eagle())]);
        preset.seats[0].starting_battlefield = vec![
            card(cavern_of_souls()),
            card(island()),
            card(island()),
            card(island()),
        ];
        let mut engine = Engine::new(&preset, Registry).expect("game starts");

        let view = settle(&mut engine, None);
        let eagles: Vec<ObjectId> = view.hand.iter().map(|c| c.id).collect();
        let lands = |name: &str| -> Vec<ObjectId> {
            view.battlefield_of(me)
                .filter(|o| o.name == name)
                .map(|o| o.id)
                .collect()
        };
        let (cavern, islands) = (lands("Cavern of Souls")[0], lands("Island"));

        for &source in &islands[..2] {
            engine
                .apply(me, PlayerAction::ActivateManaAbility { source })
                .unwrap();
        }
        engine
            .apply(me, PlayerAction::CastSpell { card: eagles[0] })
            .expect("two Islands pay {1}{U}");
        assert!(
            !uncounterable_to(&engine, me, eagles[0])
                && !uncounterable_to(&engine, them, eagles[0]),
            "an Eagle paid for with Islands can be countered, and nobody is told otherwise"
        );
        settle(&mut engine, None);

        engine
            .apply(
                me,
                PlayerAction::ActivateAbility {
                    source: cavern,
                    ability_index: 1,
                },
            )
            .expect("the Cavern's restricted mana");
        engine
            .apply(me, PlayerAction::ChooseColor(ManaColor::Blue))
            .unwrap();
        engine
            .apply(me, PlayerAction::ActivateManaAbility { source: islands[2] })
            .unwrap();
        engine
            .apply(me, PlayerAction::CastSpell { card: eagles[1] })
            .expect("the Cavern's blue and an Island pay {1}{U}");
        assert!(
            engine
                .state()
                .object(eagles[1])
                .is_some_and(|o| !o.can_be_countered()),
            "the Cavern's mana paid for this one, so the engine will not counter it"
        );
        assert!(
            uncounterable_to(&engine, me, eagles[1]) && uncounterable_to(&engine, them, eagles[1]),
            "and every seat is told so while it is on the stack"
        );

        settle(&mut engine, None);
        let eagle = player_view(engine.state(), me, 0, None, &SeatContext::default(), &[])
            .battlefield_of(me)
            .filter(|o| o.name == "Sea Eagle")
            .map(|o| o.id)
            .max()
            .expect("the second Eagle has resolved");
        assert!(
            !uncounterable_to(&engine, me, eagle),
            "the rider belongs to the spell, and the permanent it became is not a spell"
        );
    }
}
