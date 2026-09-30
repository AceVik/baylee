//! What the trained AI is shown of one decision: encoder v3.
//!
//! v2 ([`crate::features`]) stays as it is for the nets trained on it. v3
//! reads the same view and differs where v2 fell short:
//!
//! - **Piles.** Identical objects are one row with a count: the grouping key
//!   is the client's own ([`PublicObject::summary_key`], every property a
//!   decision reads) with the zone and what the question offers for the
//!   object. A question can offer one of two identical objects and not the
//!   other; the key keeps them apart, so a pile's member is always as
//!   offered as its row says. Objects others point at (an Aura and what it
//!   enchants, what a spell targets, an attacked planeswalker) and face-down
//!   ones keep rows of their own. In combat the key also holds what an
//!   attacker attacks and who blocks it, and what a blocker blocks: a swarm
//!   attacking together is one row. Consecutive identical items on the stack (the
//!   same card, kind, controller and targets) are one row too. Grouping
//!   comes before the entity cap, and offered rows before the others, so a
//!   board of hundreds of tokens loses nothing a question can name.
//! - **Seats** are rows of their own, up to [`MAX_SEATS`], in turn order from
//!   the deciding seat (row 0), with its side and whether it is still in.
//! - **The seat's own deck list**, with how many of each card are not yet
//!   seen: what a player knows of their library. Only its own list: another
//!   seat's list is hidden information, and [`encode`] is not given one.
//! - **Cards by structure.** [`card_table`] is every card's
//!   [`crate::cardwalk::features`], which the net reads beside the id
//!   embedding, so a card it never saw is not a random vector.
//!
//! The answer a pile's row stands for is its lowest unpicked member
//! ([`options`]); a recorded answer naming another member maps to the same
//! row, so a label never depends on which of two identical objects the
//! house took.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use baylee_core::ids::{CardIndex, Defender, ObjectId, PlayerId};
use baylee_engine::choice::Pending;
use baylee_engine::engine::Engine;
use baylee_engine::zone::ZoneLocation;
use baylee_gamehost::RegistryLookup;
use baylee_view::{
    CounterKind, HandObject, ObjectSummaryKey, PlayerView, PublicObject, StackItem, TargetRef,
};

use crate::features::{
    MAX_ENTITIES, TOKEN_BASE, clamp, flag, offered, public_id, question, rel, zone,
};
use crate::policy::{self, Choice, Opt, Picked};

/// The encoder's version; a dataset and a model name the one they use.
pub const VERSION: u32 = 3;

/// Seats a decision has rows for.
pub const MAX_SEATS: usize = 8;

/// Distinct cards of a deck list kept (a Commander deck is 100 singletons
/// and its basics).
pub const MAX_DECK: usize = 128;

/// Entity columns, in order: v2's, then the pile's size and how many of it
/// are picked.
pub const ENT_COLS: [&str; 40] = [
    "zone",
    "controller_rel",
    "owner_rel",
    "status",
    "types",
    "supertypes",
    "colors",
    "mana_value",
    "power",
    "toughness",
    "base_power",
    "base_toughness",
    "flags",
    "loyalty",
    "damage",
    "plus_counters",
    "minus_counters",
    "lore_counters",
    "time_counters",
    "charge_counters",
    "other_counters",
    "attached_to",
    "attacks_player_rel",
    "attacks_row",
    "blocks_row",
    "stack_pos",
    "stack_kind",
    "target_row",
    "target_player_rel",
    "offered",
    "keywords0",
    "keywords1",
    "keywords2",
    "keywords3",
    "keywords4",
    "keywords5",
    "keywords6",
    "keywords7",
    "count",
    "picked",
];

/// Seat columns, one row per seat, row `r` the seat `r` places after the
/// deciding one in turn order.
pub const SEAT_COLS: [&str; 20] = [
    "present",
    "rel",
    "same_team",
    "eliminated",
    "active",
    "asked",
    "monarch",
    "life",
    "poison",
    "energy",
    "hand",
    "library",
    "graveyard",
    "commander_damage",
    "pool_w",
    "pool_u",
    "pool_b",
    "pool_r",
    "pool_g",
    "pool_c",
];

/// Global columns.
pub const GLOB_COLS: [&str; 10] = [
    "turn",
    "phase",
    "step",
    "i_am_asked",
    "seats",
    "sides",
    "priority_held",
    "day_night",
    "pending_kind",
    "n_options",
];

/// Deck columns, one row per distinct card of the seat's list.
pub const DECK_COLS: [&str; 2] = ["in_deck", "left"];

/// What a decision needs beyond the view: who plays with whom (the roster,
/// `GameStatic`) and the seat's own deck list. Never another seat's list.
#[derive(Clone, Copy, Debug, Default)]
pub struct Table<'a> {
    /// Per seat its side; `None` plays alone. Empty: every seat alone.
    pub teams: &'a [Option<u8>],
    /// The deciding seat's deck list (commanders included), card and count.
    pub deck: &'a [(CardIndex, u32)],
}

/// One decision, encoded for the actor.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Encoded {
    /// Per entity: its id in the embedding table.
    pub cards: Vec<i32>,
    /// Per entity: [`ENT_COLS`] values.
    pub rows: Vec<[i16; ENT_COLS.len()]>,
    /// Per entity: its objects, lowest id first (one unless a pile or a run).
    pub members: Vec<Vec<ObjectId>>,
    /// Per seat, in turn order from the deciding one: [`SEAT_COLS`].
    pub seats: Vec<[i16; SEAT_COLS.len()]>,
    /// [`GLOB_COLS`].
    pub globals: Vec<i16>,
    /// Per distinct card of the deck list: its id.
    pub deck_cards: Vec<i32>,
    /// Per distinct card of the deck list: [`DECK_COLS`].
    pub deck_rows: Vec<[i16; DECK_COLS.len()]>,
    /// Objects past [`MAX_ENTITIES`] rows that were left out.
    pub dropped: usize,
    /// Of those, objects the question offered.
    pub offered_dropped: usize,
    /// The question's kind ([`crate::features::PENDING_KINDS`]).
    pub kind: i16,
    /// How many answers it has ([`question`]).
    pub options: u32,
    /// Each kept object's row.
    pub slots: BTreeMap<ObjectId, i16>,
}

/// An object the seat can see, and where.
enum Seen<'a> {
    Hand(&'a HandObject),
    Public(&'a PublicObject, usize),
}

/// One row before its columns are written.
struct Entity<'a> {
    zone: i16,
    seen: Seen<'a>,
    members: Vec<ObjectId>,
}

/// What the question lets the seat do with each object ([`offered`]).
fn offers(pending: &Pending) -> BTreeMap<ObjectId, i16> {
    let mut offers: BTreeMap<ObjectId, i16> = BTreeMap::new();
    let mut offer = |id: ObjectId, bit: i16| *offers.entry(id).or_default() |= bit;
    match pending {
        Pending::Priority { legal, .. } => {
            for id in &legal.lands {
                offer(*id, offered::LAND);
            }
            for id in legal.castable.iter().chain(&legal.suspendable) {
                offer(*id, offered::CAST);
            }
            for (id, _) in &legal.abilities {
                offer(*id, offered::ABILITY);
            }
            for id in &legal.mana_abilities {
                offer(*id, offered::MANA);
            }
        }
        Pending::ChooseAttackers { attackers, .. } => {
            for id in attackers {
                offer(*id, offered::CHOOSE);
            }
        }
        Pending::ChooseBlockers { blockers, .. } => {
            for b in blockers {
                offer(b.blocker, offered::CHOOSE);
            }
        }
        Pending::LegendChoice { options, .. }
        | Pending::ChooseCards { options, .. }
        | Pending::ChooseTargets { options, .. } => {
            for id in options {
                offer(*id, offered::CHOOSE);
            }
        }
        Pending::Arrange { cards, .. } => {
            for id in cards {
                offer(*id, offered::CHOOSE);
            }
        }
        _ => {}
    }
    offers
}

/// Objects that keep a row of their own: others point at them, or they
/// point at others, or two of them look alike without being alike.
fn individual(view: &PlayerView) -> BTreeSet<ObjectId> {
    let mut out = BTreeSet::new();
    for a in &view.combat.attackers {
        if let Defender::Planeswalker(pw) = a.defending {
            out.insert(pw);
        }
    }
    let publics = view
        .battlefield
        .iter()
        .chain(&view.stack)
        .chain(view.command.iter().flatten())
        .chain(&view.looking_at)
        .chain(&view.library_tops)
        .chain(view.graveyards.iter().flatten())
        .chain(view.exile.iter().flatten());
    for o in publics {
        if let Some(to) = o.attached_to {
            out.insert(o.id);
            out.insert(to);
        }
        if o.status.is_face_down() {
            out.insert(o.id);
        }
        for t in &o.targets {
            if let TargetRef::Object(id) = t {
                out.insert(*id);
            }
        }
    }
    out
}

/// What combat says about an object, as part of its pile key: an attacker's
/// defender, whether it is blocked and by whom; a blocker's attackers.
type CombatKey = (u8, Option<Defender>, bool, Vec<ObjectId>);

fn combat_keys(view: &PlayerView) -> BTreeMap<ObjectId, CombatKey> {
    let mut blockers_of: BTreeMap<ObjectId, Vec<ObjectId>> = BTreeMap::new();
    let mut attacked_by: BTreeMap<ObjectId, Vec<ObjectId>> = BTreeMap::new();
    for b in &view.combat.blockers {
        blockers_of.entry(b.attacker).or_default().push(b.blocker);
        attacked_by.entry(b.blocker).or_default().push(b.attacker);
    }
    let mut out = BTreeMap::new();
    for a in &view.combat.attackers {
        let mut by = blockers_of.remove(&a.creature).unwrap_or_default();
        by.sort_unstable();
        out.insert(a.creature, (1, Some(a.defending), a.blocked, by));
    }
    for (blocker, mut attackers) in attacked_by {
        attackers.sort_unstable();
        out.entry(blocker).or_insert((2, None, false, attackers));
    }
    out
}

/// The rows of one decision, grouped and in keeping order (offered first).
#[allow(clippy::too_many_lines)] // one pass per zone
fn entities<'a>(
    view: &'a PlayerView,
    offers: &BTreeMap<ObjectId, i16>,
    offered_ids: &BTreeSet<ObjectId>,
) -> Vec<Entity<'a>> {
    let alone = individual(view);
    let mut out: Vec<Entity<'a>> = view
        .hand
        .iter()
        .map(|h| Entity {
            zone: zone::HAND,
            seen: Seen::Hand(h),
            members: vec![h.id],
        })
        .collect();

    // The stack: a run of identical items is one row.
    let mut previous: Option<&PublicObject> = None;
    for (pos, o) in view.stack.iter().enumerate() {
        let joins = previous.is_some_and(|p| {
            !alone.contains(&p.id)
                && !alone.contains(&o.id)
                && public_id(p) == public_id(o)
                && p.stack_item == o.stack_item
                && p.controller == o.controller
                && p.targets == o.targets
        });
        match out.last_mut() {
            Some(last) if joins => last.members.push(o.id),
            _ => out.push(Entity {
                zone: zone::STACK,
                seen: Seen::Public(o, pos),
                members: vec![o.id],
            }),
        }
        previous = Some(o);
    }

    // Every other public zone: piles by the client's key, the zone and the
    // offer.
    let publics: [(i16, Vec<&PublicObject>); 6] = [
        (zone::BATTLEFIELD, view.battlefield.iter().collect()),
        (zone::COMMAND, view.command.iter().flatten().collect()),
        (zone::LOOKING, view.looking_at.iter().collect()),
        (zone::LIBRARY_TOP, view.library_tops.iter().collect()),
        (zone::GRAVEYARD, view.graveyards.iter().flatten().collect()),
        (zone::EXILE, view.exile.iter().flatten().collect()),
    ];
    // Looked up only, never iterated: the order is the view's.
    let combat = combat_keys(view);
    let mut piles: HashMap<(i16, ObjectSummaryKey, i16, Option<CombatKey>), usize> = HashMap::new();
    for (z, objects) in publics {
        for o in objects {
            if alone.contains(&o.id) {
                out.push(Entity {
                    zone: z,
                    seen: Seen::Public(o, 0),
                    members: vec![o.id],
                });
                continue;
            }
            let key = (
                z,
                o.summary_key(),
                offers.get(&o.id).copied().unwrap_or(0),
                combat.get(&o.id).cloned(),
            );
            if let Some(&at) = piles.get(&key) {
                out[at].members.push(o.id);
            } else {
                piles.insert(key, out.len());
                out.push(Entity {
                    zone: z,
                    seen: Seen::Public(o, 0),
                    members: vec![o.id],
                });
            }
        }
    }
    for e in &mut out {
        e.members.sort_unstable();
    }
    // Offered rows first, so a cap loses only rows no answer can name.
    let (mut first, rest): (Vec<_>, Vec<_>) = out
        .into_iter()
        .partition(|e| e.members.iter().any(|m| offered_ids.contains(m)));
    first.extend(rest);
    first
}

/// The seat rows: turn order from `seat`, each seat's side and state.
fn seat_rows(view: &PlayerView, teams: &[Option<u8>]) -> Vec<[i16; SEAT_COLS.len()]> {
    let seat = view.seat;
    let n = view.seats.len();
    let team_of = |p: PlayerId| teams.get(usize::from(p.get())).copied().flatten();
    let mut rows = vec![[0_i16; SEAT_COLS.len()]; n.min(MAX_SEATS)];
    for sv in &view.seats {
        let r = rel(seat, sv.player, n);
        let Some(row) = usize::try_from(r).ok().and_then(|r| rows.get_mut(r)) else {
            continue;
        };
        let same_team =
            sv.player == seat || team_of(seat).is_some_and(|t| team_of(sv.player) == Some(t));
        let commander_damage = sv
            .commander_damage
            .iter()
            .map(|d| i64::from(d.amount))
            .max()
            .unwrap_or(0);
        let p = &sv.mana_pool;
        let values = [
            1,
            i64::from(r),
            i64::from(same_team),
            i64::from(sv.loss.is_some()),
            i64::from(view.active == sv.player),
            i64::from(view.awaiting == Some(sv.player) || view.deciding.contains(sv.player)),
            i64::from(view.monarch == Some(sv.player)),
            i64::from(sv.life),
            i64::from(sv.poison),
            i64::from(sv.energy),
            i64::from(sv.hand_count),
            i64::from(sv.library_count),
            i64::from(sv.graveyard_count),
            commander_damage,
            i64::from(p.white),
            i64::from(p.blue),
            i64::from(p.black),
            i64::from(p.red),
            i64::from(p.green),
            i64::from(p.colorless),
        ];
        for (k, v) in values.into_iter().enumerate() {
            row[k] = clamp(v);
        }
    }
    rows
}

/// The seat's deck list with how many of each card it has not seen yet: its
/// hand, and every public object it owns, count as seen. Read from the view
/// alone, so a live client can make the same rows.
fn deck_rows(view: &PlayerView, deck: &[(CardIndex, u32)]) -> (Vec<i32>, Vec<[i16; 2]>) {
    let seat = view.seat;
    let mut seen: BTreeMap<CardIndex, i64> = BTreeMap::new();
    for h in &view.hand {
        *seen.entry(h.card.index).or_default() += 1;
    }
    let publics = view
        .battlefield
        .iter()
        .chain(&view.stack)
        .chain(view.command.iter().flatten())
        .chain(view.graveyards.iter().flatten())
        .chain(view.exile.iter().flatten());
    for o in publics {
        if o.owner == seat
            && o.token.is_none()
            && let Some(card) = &o.card
        {
            *seen.entry(card.index).or_default() += 1;
        }
    }
    let mut list: BTreeMap<CardIndex, i64> = BTreeMap::new();
    for (card, count) in deck {
        *list.entry(*card).or_default() += i64::from(*count);
    }
    let mut cards = Vec::new();
    let mut rows = Vec::new();
    for (card, count) in list.into_iter().take(MAX_DECK) {
        cards.push((card.get() + 1) as i32);
        let left = (count - seen.get(&card).copied().unwrap_or(0)).max(0);
        rows.push([clamp(count), clamp(left)]);
    }
    (cards, rows)
}

/// Encodes the decision `view` shows with `pending` asked, `picked` picked so
/// far by an answer given one pick at a time.
#[must_use]
#[allow(clippy::too_many_lines)] // one column per line
pub fn encode(view: &PlayerView, pending: &Pending, picked: &Picked, table: &Table<'_>) -> Encoded {
    let seat = view.seat;
    let seats = view.seats.len();
    let (kind, options) = question(pending, view.hand.len());
    let offers = offers(pending);
    let hand: Vec<ObjectId> = view.hand.iter().map(|h| h.id).collect();
    let offered_ids = policy::offered_objects(pending, &hand);

    let mut rows = entities(view, &offers, &offered_ids);
    let dropped: usize = rows
        .iter()
        .skip(MAX_ENTITIES)
        .map(|e| e.members.len())
        .sum();
    let offered_dropped = rows
        .iter()
        .skip(MAX_ENTITIES)
        .flat_map(|e| &e.members)
        .filter(|m| offered_ids.contains(m))
        .count();
    rows.truncate(MAX_ENTITIES);
    // An object can be seen twice (looked at while on the stack); its first
    // row is the one others point at.
    let mut slot: BTreeMap<ObjectId, i16> = BTreeMap::new();
    for (i, e) in rows.iter().enumerate() {
        for m in &e.members {
            slot.entry(*m).or_insert(i as i16);
        }
    }
    let row_of = |id: ObjectId| slot.get(&id).copied().unwrap_or(-1);
    let attacking: BTreeMap<ObjectId, &baylee_view::AttackerView> = view
        .combat
        .attackers
        .iter()
        .map(|a| (a.creature, a))
        .collect();
    let blocking: BTreeMap<ObjectId, ObjectId> = view
        .combat
        .blockers
        .iter()
        .map(|b| (b.blocker, b.attacker))
        .collect();

    let mut out = Encoded {
        dropped,
        offered_dropped,
        kind,
        options,
        ..Encoded::default()
    };
    for e in &rows {
        let mut r = [0_i16; ENT_COLS.len()];
        let id = e.members[0];
        let picked_here = e
            .members
            .iter()
            .filter(|m| picked.objects.contains(m))
            .count();
        r[0] = e.zone;
        r[29] = offers.get(&id).copied().unwrap_or(0)
            | if picked_here > 0 { offered::PICKED } else { 0 };
        for col in [21, 22, 23, 24, 25, 27, 28] {
            r[col] = -1;
        }
        r[38] = clamp(e.members.len() as i64);
        r[39] = clamp(picked_here as i64);
        match &e.seen {
            Seen::Hand(h) => {
                out.cards.push((h.card.index.get() + 1) as i32);
                r[4] = h.types.bits() as i16;
                r[6] = i16::from(h.colors.bits());
                r[7] = clamp(i64::from(h.mana_value));
                r[12] = flag::KNOWN | if h.commander { flag::COMMANDER } else { 0 };
            }
            Seen::Public(o, pos) => {
                let (card, known) = public_id(o);
                out.cards.push(card);
                r[1] = rel(seat, o.controller, seats);
                r[2] = rel(seat, o.owner, seats);
                r[3] = i16::from(o.status.bits());
                r[4] = o.types.bits() as i16;
                r[5] = i16::from(o.supertypes.bits());
                r[6] = i16::from(o.colors.bits());
                r[7] = clamp(i64::from(o.mana_value));
                r[8] = o.power.unwrap_or(0);
                r[9] = o.toughness.unwrap_or(0);
                r[10] = o.base_power.unwrap_or(0);
                r[11] = o.base_toughness.unwrap_or(0);
                let mut flags = 0;
                if o.power.is_some() || o.toughness.is_some() {
                    flags |= flag::HAS_PT;
                }
                if o.summoning_sick {
                    flags |= flag::SICK;
                }
                if o.token.is_some() || (o.card.is_none() && known) {
                    flags |= flag::TOKEN;
                }
                if o.commander {
                    flags |= flag::COMMANDER;
                }
                if o.suspended {
                    flags |= flag::SUSPENDED;
                }
                if known {
                    flags |= flag::KNOWN;
                }
                if let Some(a) = attacking.get(&id) {
                    flags |= flag::ATTACKING;
                    if a.blocked {
                        flags |= flag::BLOCKED;
                    }
                    match a.defending {
                        Defender::Player(p) => r[22] = rel(seat, p, seats),
                        Defender::Planeswalker(pw) => r[23] = row_of(pw),
                    }
                }
                if let Some(attacker) = blocking.get(&id) {
                    flags |= flag::BLOCKING;
                    r[24] = row_of(*attacker);
                }
                r[12] = flags;
                r[13] = clamp(i64::from(o.loyalty.unwrap_or(0)));
                r[14] = clamp(i64::from(o.damage));
                for c in &o.counters {
                    let at = match c.kind {
                        CounterKind::Plus { .. } => 15,
                        CounterKind::Minus { .. } => 16,
                        CounterKind::Lore => 17,
                        CounterKind::Time => 18,
                        CounterKind::Charge => 19,
                        _ => 20,
                    };
                    r[at] = clamp(i64::from(r[at]) + i64::from(c.count));
                }
                r[21] = o.attached_to.map_or(-1, row_of);
                if e.zone == zone::STACK {
                    r[25] = clamp(*pos as i64);
                    r[26] = match o.stack_item {
                        Some(StackItem::Spell) => 1,
                        Some(_) => 2,
                        None => 0,
                    };
                }
                for t in &o.targets {
                    match t {
                        TargetRef::Object(target) if r[27] < 0 => r[27] = row_of(*target),
                        TargetRef::Player(p) if r[28] < 0 => r[28] = rel(seat, *p, seats),
                        _ => {}
                    }
                }
                for (k, col) in (30..38).enumerate() {
                    r[col] = (o.keywords >> (16 * k)) as u16 as i16;
                }
            }
        }
        out.rows.push(r);
        out.members.push(e.members.clone());
    }

    out.seats = seat_rows(view, table.teams);
    let sides: BTreeSet<i64> = view
        .seats
        .iter()
        .map(|sv| {
            table
                .teams
                .get(usize::from(sv.player.get()))
                .copied()
                .flatten()
                .map_or(1000 + i64::from(sv.player.get()), i64::from)
        })
        .collect();
    out.globals = vec![
        clamp(i64::from(view.turn)),
        view.phase as i16,
        view.step as i16,
        i16::from(view.awaiting == Some(seat)),
        clamp(seats as i64),
        clamp(sides.len() as i64),
        i16::from(view.priority_held),
        match view.day_night {
            None => 0,
            Some(baylee_view::DayNight::Day) => 1,
            Some(baylee_view::DayNight::Night) => 2,
        },
        kind,
        clamp(i64::from(options)),
    ];
    (out.deck_cards, out.deck_rows) = deck_rows(view, table.deck);
    out.slots = slot;
    out
}

/// The answers `pending` offers at this step, one per row an answer can
/// name: of several members of one pile, the lowest unpicked one stands for
/// the pile. Each with the (head, a, b) triple the policy scores.
///
/// A blocker assigned to a pile of identical attackers blocks one that still
/// needs blockers to reach its least (menace), else the one with the fewest
/// blockers so far, so blocks spread over the pile. The price: of identical
/// attackers without menace, the net cannot double-block one and leave
/// another. "Done" is offered only where the question's own check
/// (`Pending::answer_fault`) passes the answer so far.
#[must_use]
pub fn options(
    encoded: &Encoded,
    view: &PlayerView,
    pending: &Pending,
    picked: &Picked,
) -> Vec<(Choice, Opt)> {
    let hand: Vec<ObjectId> = view.hand.iter().map(|h| h.id).collect();
    let seats = view.seats.len();
    let row_of = |id| encoded.slots.get(&id).copied();
    let rel_of = |p| rel(view.seat, p, seats);
    let mut choices = policy::options(pending, &hand, picked).unwrap_or_default();
    // "Done" only where the question's own check passes the answer so far,
    // unless nothing else is left: then the answer is refused as stated.
    if choices.len() > 1
        && choices.contains(&Choice::Fixed(policy::fixed::DONE))
        && !policy::done_allowed(pending, picked)
    {
        choices.retain(|c| *c != Choice::Fixed(policy::fixed::DONE));
    }
    // An attacker blocked by fewer than its least (menace) comes first, so a
    // group is completed before another is started.
    let bounds = policy::blocker_bounds(pending);
    choices.sort();
    choices.sort_by_key(|c| match c {
        Choice::Block(_, attacker) => {
            let has = u32::from(picked.blocked.get(attacker).copied().unwrap_or(0));
            let least = bounds.get(attacker).map_or(1, |b| b.0);
            if has > 0 && has < least { 0 } else { 1 + has }
        }
        _ => 0,
    });
    let mut seen: BTreeSet<(i16, i16, i16)> = BTreeSet::new();
    let mut out = Vec::new();
    for c in choices {
        if let Some(o) = policy::opt(c, &row_of, &rel_of)
            && seen.insert((o.head, o.a, o.b))
        {
            out.push((c, o));
        }
    }
    out
}

/// What the seat's view hides, for training only: per other seat its hand,
/// the top of every library, and every seat's deck list with what is left of
/// it in its library.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Omni {
    /// Hidden cards: ids.
    pub cards: Vec<i32>,
    /// Per hidden card: `seat_rel * 16 + what`, `what` 1 = hand, 2 = library
    /// top (then `position` counts from the top).
    pub tags: Vec<i16>,
    /// Per hidden card: its library depth (0 in a hand).
    pub positions: Vec<i16>,
    /// Deck lists: per distinct card of a seat's list, its id.
    pub deck_cards: Vec<i32>,
    /// Per such card: the seat (relative), how many the list has, how many
    /// are in its library now.
    pub deck_rows: Vec<[i16; 3]>,
}

/// The hidden part of the game `engine` is in, as `seat` would not see it;
/// `decks` is every seat's list, in seat order.
#[must_use]
pub fn omniscient(
    engine: &Engine<RegistryLookup>,
    seat: PlayerId,
    decks: &[Vec<(CardIndex, u32)>],
) -> Omni {
    let v2 = crate::features::omniscient(engine, seat);
    let state = engine.state();
    let seats = state.players.len();
    let mut omni = Omni {
        cards: v2.cards,
        tags: v2.tags,
        positions: v2.positions,
        ..Omni::default()
    };
    for (p, deck) in decks.iter().enumerate().take(seats) {
        let player = PlayerId::new(p as u8);
        let r = rel(seat, player, seats);
        let mut in_library: BTreeMap<CardIndex, i64> = BTreeMap::new();
        for id in state.zones.list(ZoneLocation::Library(player)) {
            if let Some(card) = state.object(*id).and_then(|o| o.card) {
                *in_library.entry(card.index).or_default() += 1;
            }
        }
        let mut list: BTreeMap<CardIndex, i64> = BTreeMap::new();
        for (card, count) in deck {
            *list.entry(*card).or_default() += i64::from(*count);
        }
        for (card, count) in list.into_iter().take(MAX_DECK) {
            omni.deck_cards.push((card.get() + 1) as i32);
            omni.deck_rows.push([
                r,
                clamp(count),
                clamp(in_library.get(&card).copied().unwrap_or(0)),
            ]);
        }
    }
    omni
}

/// Every pool card's and registry token's structural features
/// ([`crate::cardwalk`]): the ids and, row by row, `cardwalk::WIDTH` values
/// each. A dataset writes it once; the net reads it beside the id embedding.
#[must_use]
pub fn card_table() -> (Vec<i32>, Vec<f32>) {
    let mut ids = Vec::new();
    let mut values = Vec::new();
    for def in baylee_cards::all() {
        ids.push((def.index.get() + 1) as i32);
        values.extend(crate::cardwalk::features(def));
    }
    for (i, token) in baylee_cards::tokens::ALL.iter().enumerate() {
        ids.push((TOKEN_BASE + i as u32) as i32);
        values.extend(crate::cardwalk::token_features(token));
    }
    (ids, values)
}

/// A seat's deck list from a preset seat: its deck and its commanders.
#[must_use]
pub fn deck_list(seat: &baylee_core::preset::SeatSpec) -> Vec<(CardIndex, u32)> {
    let mut counts: BTreeMap<CardIndex, u32> = BTreeMap::new();
    for e in seat.deck.iter().chain(&seat.commanders) {
        *counts.entry(e.card).or_default() += 1;
    }
    counts.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::seat_view;
    use baylee_core::preset::{AIProfile, DeckEntry};
    use baylee_engine::choice::PlayerAction;

    /// Deals `preset`, keeps every hand, and returns the engine at its first
    /// question after the mulligans.
    fn dealt(preset: &baylee_core::preset::GamePreset) -> Engine<RegistryLookup> {
        let mut engine = Engine::new(preset, RegistryLookup).expect("the preset builds");
        for _ in 0..16 {
            let Pending::Mulligan { player, .. } = engine.pending().clone() else {
                break;
            };
            engine
                .apply(player, PlayerAction::MulliganKeep)
                .expect("keeping is an answer");
        }
        engine
    }

    fn house(key: &str) -> crate::housedeck::HouseDeck {
        crate::housedeck::HouseDeck::named(key).unwrap()
    }

    /// 300 Forests offered for mana are one row, and no offered object is
    /// dropped; v2 dropped 44 of them.
    #[test]
    fn a_swarm_of_identical_offered_objects_is_one_row_and_nothing_is_dropped() {
        let (a, b) = (house("allytifact"), house("victory"));
        let mut preset = crate::selfplay::table(1, &a, &b, [AIProfile::default(); 2]);
        let forest = baylee_cards::decks::by_name("Forest").expect("a Forest");
        let print = preset.seats[0].deck[0].print;
        preset.seats[0].starting_battlefield = vec![
            DeckEntry {
                card: forest,
                print
            };
            300
        ];
        let engine = dealt(&preset);
        let seat = PlayerId::new(0);
        let pending = engine.pending_for(seat).expect("seat 0 is asked").clone();
        let Pending::Priority { legal, .. } = &pending else {
            panic!("seat 0 is asked {pending:?}");
        };
        assert!(
            legal.mana_abilities.len() >= 300,
            "{}",
            legal.mana_abilities.len()
        );
        let view = seat_view(&engine, seat, &pending, 0);

        let v2 = crate::features::encode(&view, &pending, &Picked::default());
        assert!(v2.offered_dropped > 0, "v2 drops offered Forests");

        let deck = deck_list(&preset.seats[0]);
        let table = Table {
            teams: &[],
            deck: &deck,
        };
        let enc = encode(&view, &pending, &Picked::default(), &table);
        assert_eq!(enc.offered_dropped, 0);
        assert_eq!(enc.dropped, 0);
        let forests: Vec<usize> = (0..enc.rows.len())
            .filter(|&i| enc.rows[i][38] >= 300)
            .collect();
        assert_eq!(forests.len(), 1, "one pile of Forests");
        assert_eq!(enc.rows.len(), enc.cards.len());
        assert_eq!(
            enc.members[forests[0]].len(),
            usize::from(enc.rows[forests[0]][38] as u16)
        );
        // One option stands for the pile: its lowest member.
        let opts = options(&enc, &view, &pending, &Picked::default());
        let pile = &enc.members[forests[0]];
        let standing: Vec<&Choice> = opts
            .iter()
            .map(|(c, _)| c)
            .filter(|c| matches!(c, Choice::Entity(o, _) if pile.contains(o)))
            .collect();
        assert_eq!(standing.len(), 1);
        assert!(matches!(standing[0], Choice::Entity(o, _) if *o == pile[0]));
    }

    /// Two swarms of 300 identical creatures in combat: the attackers are one
    /// row and the blockers one row, nothing offered is dropped, and two
    /// blockers assigned into the attacking pile block two of its members.
    #[test]
    fn a_swarm_in_combat_is_one_row_per_side() {
        let (a, b) = (house("allytifact"), house("victory"));
        let mut preset = crate::selfplay::table(5, &a, &b, [AIProfile::default(); 2]);
        let creature = [
            "Grizzly Bears",
            "Savannah Lions",
            "Llanowar Elves",
            "Elvish Mystic",
        ]
        .iter()
        .find_map(|n| baylee_cards::decks::by_name(n))
        .expect("a small creature in the pool");
        for seat in &mut preset.seats {
            let print = seat.deck[0].print;
            seat.starting_battlefield = vec![
                DeckEntry {
                    card: creature,
                    print
                };
                300
            ];
        }
        let mut engine = dealt(&preset);
        let mut checked = false;
        for _ in 0..400 {
            match engine.pending().clone() {
                Pending::Priority { player, .. } => {
                    engine.apply(player, PlayerAction::PassPriority).unwrap();
                }
                Pending::ChooseAttackers {
                    player, attackers, ..
                } => {
                    let defender = PlayerId::new(1 - player.get());
                    let all = attackers
                        .iter()
                        .map(|a| (*a, Defender::Player(defender)))
                        .collect();
                    engine
                        .apply(player, PlayerAction::DeclareAttackers { attackers: all })
                        .unwrap();
                }
                Pending::ChooseBlockers { player, .. } => {
                    let pending = engine.pending().clone();
                    let view = seat_view(&engine, player, &pending, 0);
                    assert!(view.combat.attackers.len() >= 300);
                    let deck = deck_list(&preset.seats[usize::from(player.get())]);
                    let table = Table {
                        teams: &[],
                        deck: &deck,
                    };
                    let enc = encode(&view, &pending, &Picked::default(), &table);
                    assert_eq!(enc.offered_dropped, 0);
                    assert!(enc.rows.len() < 20, "{} rows", enc.rows.len());
                    let mut picked = Picked::default();
                    let mut blocked = Vec::new();
                    for _ in 0..2 {
                        let (choice, _) = options(&enc, &view, &pending, &picked)
                            .into_iter()
                            .find(|(c, _)| matches!(c, Choice::Block(..)))
                            .expect("a block is offered");
                        if let Choice::Block(_, attacker) = choice {
                            blocked.push(attacker);
                        }
                        picked.add(choice);
                    }
                    assert_ne!(blocked[0], blocked[1], "blocks spread over the pile");
                    checked = true;
                    break;
                }
                other => panic!("unexpected question {other:?}"),
            }
        }
        assert!(checked, "the game reached a declaration of blockers");
    }

    /// Seats are rows in turn order from the deciding seat, with their side.
    #[test]
    fn seats_are_rows_in_turn_order_with_their_side() {
        let decks = [
            house("allytifact"),
            house("victory"),
            house("schwarzrand"),
            house("weltenbaum"),
        ];
        let refs: Vec<&crate::housedeck::HouseDeck> = decks.iter().collect();
        let preset =
            crate::selfplay::table_for(2, &refs, &[AIProfile::default(); 4], &[1, 2, 1, 2]);
        let engine = dealt(&preset);
        let seat = (0..4)
            .map(PlayerId::new)
            .find(|p| engine.pending_for(*p).is_some())
            .expect("someone is asked");
        let pending = engine.pending_for(seat).unwrap().clone();
        let view = seat_view(&engine, seat, &pending, 0);
        let teams = [Some(1), Some(2), Some(1), Some(2)];
        let deck = deck_list(&preset.seats[usize::from(seat.get())]);
        let enc = encode(
            &view,
            &pending,
            &Picked::default(),
            &Table {
                teams: &teams,
                deck: &deck,
            },
        );
        assert_eq!(enc.seats.len(), 4);
        for (r, row) in enc.seats.iter().enumerate() {
            assert_eq!(row[0], 1, "present");
            assert_eq!(usize::from(row[1] as u16), r, "rel");
            // Two seats on, the partner sits.
            assert_eq!(row[2], i16::from(r % 2 == 0), "same team at {r}");
        }
        assert_eq!(enc.globals[4], 4, "seats");
        assert_eq!(enc.globals[5], 2, "sides");
    }

    /// The deck rows are the seat's own list, less what it has seen: a hand
    /// drawn is not left in the library. No other seat's card is in them.
    #[test]
    fn the_deck_rows_are_the_seats_own_list_less_what_it_saw() {
        let (a, b) = (house("allytifact"), house("victory"));
        let preset = crate::selfplay::table(3, &a, &b, [AIProfile::default(); 2]);
        let engine = dealt(&preset);
        let seat = PlayerId::new(0);
        let pending = engine.pending_for(seat).expect("seat 0 is asked").clone();
        let view = seat_view(&engine, seat, &pending, 0);
        let mine = deck_list(&preset.seats[0]);
        let theirs = deck_list(&preset.seats[1]);
        let enc = encode(
            &view,
            &pending,
            &Picked::default(),
            &Table {
                teams: &[],
                deck: &mine,
            },
        );
        let in_deck: i64 = enc.deck_rows.iter().map(|r| i64::from(r[0])).sum();
        let left: i64 = enc.deck_rows.iter().map(|r| i64::from(r[1])).sum();
        let listed: i64 = mine.iter().map(|(_, n)| i64::from(*n)).sum();
        assert_eq!(in_deck, listed);
        // Seen: the hand, and the cards of the list it owns in public zones
        // (a commander in the command zone).
        let public_own = view
            .battlefield
            .iter()
            .chain(view.command.iter().flatten())
            .chain(view.graveyards.iter().flatten())
            .chain(view.exile.iter().flatten())
            .filter(|o| o.owner == seat && o.token.is_none() && o.card.is_some())
            .count();
        assert_eq!(left, listed - (view.hand.len() + public_own) as i64);
        let own: BTreeSet<i32> = mine.iter().map(|(c, _)| (c.get() + 1) as i32).collect();
        assert!(enc.deck_cards.iter().all(|c| own.contains(c)));
        let only_theirs: Vec<i32> = theirs
            .iter()
            .map(|(c, _)| (c.get() + 1) as i32)
            .filter(|c| !own.contains(c))
            .collect();
        assert!(!only_theirs.is_empty(), "the decks differ");
        assert!(enc.deck_cards.iter().all(|c| !only_theirs.contains(c)));
        // The critic reads both lists, with what is in each library.
        let omni = omniscient(&engine, seat, &[mine.clone(), theirs]);
        assert!(omni.deck_rows.iter().any(|r| r[0] == 1));
        let mine_left: i64 = omni
            .deck_rows
            .iter()
            .filter(|r| r[0] == 0)
            .map(|r| i64::from(r[2]))
            .sum();
        assert_eq!(mine_left, left);
    }

    /// Every pool card and registry token has a row of the card table.
    #[test]
    fn the_card_table_has_a_row_per_card_and_token() {
        let (ids, values) = card_table();
        assert_eq!(values.len(), ids.len() * crate::cardwalk::WIDTH);
        let cards = baylee_cards::all().count();
        assert_eq!(ids.len(), cards + baylee_cards::tokens::ALL.len());
    }
}
