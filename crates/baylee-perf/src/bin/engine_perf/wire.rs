//! What a table's frames come to on the wire, per seat and per message kind,
//! and what the same views would come to as patches
//! (`docs/proposals/view-patches.md`, `docs/perf-baseline.md`).
//!
//! Everything here reads frames as a socket receives them: a seat's view is the
//! `StateDelta` it was sent, decoded, and not what a session could have told it.

#![allow(clippy::cast_precision_loss)] // percentages and means in a report

use std::collections::BTreeMap;
use std::time::Instant;

use baylee_core::ids::PlayerId;
use baylee_engine::choice::Pending;
use baylee_gamehost::PlayerView;
use baylee_protocol::v1::Envelope;
use baylee_protocol::v1::envelope::Msg;
use baylee_view::{ViewPatch, ViewReader, Zone, ZoneEdit, apply, diff, digest};
use prost::Message as _;
use serde::Serialize;
use serde_json::Value;

/// Frames and the bytes they came to.
#[derive(Clone, Copy, Default)]
struct Count {
    frames: u64,
    bytes: u64,
}

impl Count {
    const fn add(&mut self, bytes: usize) {
        self.frames += 1;
        self.bytes += bytes as u64;
    }

    const fn merge(&mut self, other: Self) {
        self.frames += other.frames;
        self.bytes += other.bytes;
    }
}

/// Objects seen in the views a seat was sent, against the view before each.
#[derive(Clone, Copy, Default)]
struct Churn {
    /// Objects in the views, each view counted.
    objects: u64,
    /// Those that are the object of the last view, field for field.
    same: u64,
    /// Those whose id the zone held and that differ now.
    changed: u64,
    /// Those the zone did not hold.
    new: u64,
    /// Objects that left a zone.
    removed: u64,
    /// Views whose zones and hand did not change at all.
    still: u64,
    /// Views in which only `seq` differs.
    empty: u64,
}

/// Which parts of a patch its bytes went to, as JSON.
#[derive(Clone, Copy, Default)]
struct PatchParts {
    /// Whole objects that are new to their zone.
    new_objects: u64,
    /// Whole objects that changed in their zone.
    changed_objects: u64,
    /// The same objects, with only the fields that changed.
    changed_fields: u64,
    /// Ids removed and orders.
    ids: u64,
    /// Seat lines.
    seats: u64,
    /// The other fields of the view.
    fields: u64,
}

/// Nanoseconds spent, by what on, to compare the two paths. Wall time on a
/// machine that may be busy: read the ratios.
#[derive(Clone, Copy, Default)]
struct Costs {
    write_view: u128,
    read_view: u128,
    diff: u128,
    write_patch: u128,
    read_patch: u128,
    apply: u128,
    digest: u128,
}

/// One seat's frames.
#[derive(Default)]
struct SeatWire {
    /// Every frame.
    all: Count,
    /// `StateDelta`.
    state: Count,
    /// `StateDelta`s that carry only the rest of a long log.
    repeats: u64,
    /// What the views came to whole, as JSON.
    view_bytes: u64,
    /// `log_json` of those.
    log_bytes: u64,
    /// `ChoiceRequest`.
    question: Count,
    /// `Priority` questions, and the JSON of their `LegalActions`.
    priority: Count,
    /// `Priority` questions whose `LegalActions` were the last one's.
    legal_same: u64,
    /// The `GameStatic` payload.
    statics: Count,
    /// Anything else.
    other: Count,
    /// How many view frames came before each question since the last.
    between: BTreeMap<u64, u64>,
    since_question: u64,
    /// Of those, how many changed something a table draws.
    between_moved: BTreeMap<u64, u64>,
    moved_since_question: u64,
    churn: Churn,
    /// What the views after the first come to as patches, JSON.
    patches: Count,
    /// The first view: what a snapshot costs.
    snapshot: u64,
    parts: PatchParts,
    /// Each other field of the view, by name: how often it changed, and what
    /// that came to.
    by_field: BTreeMap<String, Count>,
    costs: Costs,
    last_view: Option<PlayerView>,
    reader: ViewReader,
    /// What the view frames came to on the wire.
    frame_bytes: u64,
    last_legal: Option<Vec<u8>>,
}

fn json_len<T: Serialize + ?Sized>(value: &T) -> u64 {
    serde_json::to_vec(value).map_or(0, |json| json.len() as u64)
}

/// What an object's changed fields come to as JSON: each changed or dropped
/// key with its value, and the id that says whose they are.
fn field_bytes<T: Serialize>(old: &T, new: &T) -> u64 {
    let (Ok(Value::Object(old)), Ok(Value::Object(new))) =
        (serde_json::to_value(old), serde_json::to_value(new))
    else {
        return 0;
    };
    let written = new
        .iter()
        .filter(|(key, value)| old.get(*key) != Some(value))
        .map(|(key, value)| json_len(key) + 1 + json_len(value) + 1);
    let dropped = old
        .keys()
        .filter(|key| !new.contains_key(*key))
        .map(|key| json_len(key) + 1);
    written.chain(dropped).sum::<u64>() + 12
}

/// `old`'s object of `id` in the zone the patch names.
fn zone_of(view: &PlayerView, zone: Zone) -> &[baylee_view::PublicObject] {
    let seat = |zones: &'_ [Vec<baylee_view::PublicObject>], at: u8| -> usize {
        usize::from(at).min(zones.len().saturating_sub(1))
    };
    match zone {
        Zone::Battlefield => &view.battlefield,
        Zone::Stack => &view.stack,
        Zone::Graveyard(at) => &view.graveyards[seat(&view.graveyards, at)],
        Zone::Exile(at) => &view.exile[seat(&view.exile, at)],
        Zone::Command(at) => &view.command[seat(&view.command, at)],
        Zone::LookingAt => &view.looking_at,
        Zone::LibraryTops => &view.library_tops,
    }
}

fn objects_in(view: &PlayerView) -> u64 {
    let zones = view.battlefield.len()
        + view.stack.len()
        + view.graveyards.iter().map(Vec::len).sum::<usize>()
        + view.exile.iter().map(Vec::len).sum::<usize>()
        + view.command.iter().map(Vec::len).sum::<usize>()
        + view.looking_at.len()
        + view.library_tops.len()
        + view.hand.len();
    zones as u64
}

impl SeatWire {
    fn state(&mut self, frame: &[u8], log_json: &[u8], bytes: usize) {
        self.state.add(bytes);
        self.frame_bytes += frame.len() as u64;
        self.log_bytes += log_json.len() as u64;
        if frame.is_empty() {
            self.repeats += 1;
            return;
        }
        let started = Instant::now();
        let view = self
            .reader
            .read(frame)
            .expect("a seat's frame places")
            .expect("a seat holds a view")
            .clone();
        self.costs.read_view += started.elapsed().as_nanos();
        let started = Instant::now();
        let whole = serde_json::to_vec(&view).expect("a view serializes").len() as u64;
        self.costs.write_view += started.elapsed().as_nanos();
        self.view_bytes += whole;
        let started = Instant::now();
        let fingerprint = digest(&view);
        self.costs.digest += started.elapsed().as_nanos();
        std::hint::black_box(fingerprint);
        self.since_question += 1;
        let moved = if let Some(last) = self.last_view.take() {
            self.patch(&last, &view)
        } else {
            self.snapshot = whole;
            true
        };
        self.moved_since_question += u64::from(moved);
        self.last_view = Some(view);
    }

    /// Counts `new` as a patch on `last`; whether it changed more than `seq`.
    fn patch(&mut self, last: &PlayerView, new: &PlayerView) -> bool {
        let started = Instant::now();
        let patch = diff(last, new);
        self.costs.diff += started.elapsed().as_nanos();
        let started = Instant::now();
        let json = serde_json::to_vec(&patch).expect("a patch serializes");
        self.costs.write_patch += started.elapsed().as_nanos();
        let started = Instant::now();
        let read: ViewPatch = serde_json::from_slice(&json).expect("a patch reads back");
        self.costs.read_patch += started.elapsed().as_nanos();
        let started = Instant::now();
        let rebuilt = apply(last, &read).expect("a patch applies to its base");
        self.costs.apply += started.elapsed().as_nanos();
        assert_eq!(&rebuilt, new, "a patch leaves its view");
        self.patches.add(json.len());
        self.count_patch(last, new, &patch);
        !patch.is_empty()
    }

    fn count_patch(&mut self, last: &PlayerView, new: &PlayerView, patch: &ViewPatch) {
        let churn = &mut self.churn;
        let parts = &mut self.parts;
        churn.objects += objects_in(new);
        let (mut changed, mut fresh, mut removed) = (0, 0, 0);
        for (zone, edit) in &patch.zones {
            let held = zone_of(last, *zone);
            count_edit(held, edit, parts, &mut changed, &mut fresh, &mut removed);
        }
        if let Some(edit) = &patch.hand {
            count_edit(
                &last.hand,
                edit,
                parts,
                &mut changed,
                &mut fresh,
                &mut removed,
            );
        }
        churn.changed += changed;
        churn.new += fresh;
        churn.removed += removed;
        churn.same += objects_in(new).saturating_sub(changed + fresh);
        churn.still += u64::from(patch.zones.is_empty() && patch.hand.is_none());
        churn.empty += u64::from(patch.is_empty());
        parts.seats += json_len(&patch.seats);
        parts.fields += json_len(&patch.fields);
        for field in &patch.fields {
            if let Ok(Value::Object(field)) = serde_json::to_value(field)
                && let Some((name, value)) = field.iter().next()
            {
                let tally = self.by_field.entry(name.clone()).or_default();
                tally.add(json_len(value) as usize);
            }
        }
    }

    fn question(&mut self, pending_json: &[u8], bytes: usize) {
        self.question.add(bytes);
        *self.between.entry(self.since_question).or_default() += 1;
        *self
            .between_moved
            .entry(self.moved_since_question)
            .or_default() += 1;
        self.since_question = 0;
        self.moved_since_question = 0;
        let pending: Pending = serde_json::from_slice(pending_json).expect("a question reads");
        if let Pending::Priority { legal, .. } = pending {
            self.priority.add(json_len(&legal) as usize);
            self.legal_same += u64::from(
                self.last_legal.as_deref()
                    == Some(&serde_json::to_vec(&legal).unwrap_or_default()[..]),
            );
            self.last_legal = serde_json::to_vec(&legal).ok();
        }
    }

    fn merge(&mut self, other: &Self) {
        self.all.merge(other.all);
        self.state.merge(other.state);
        self.repeats += other.repeats;
        self.view_bytes += other.view_bytes;
        self.frame_bytes += other.frame_bytes;
        self.log_bytes += other.log_bytes;
        self.question.merge(other.question);
        self.priority.merge(other.priority);
        self.legal_same += other.legal_same;
        self.statics.merge(other.statics);
        self.other.merge(other.other);
        for (gap, times) in &other.between {
            *self.between.entry(*gap).or_default() += times;
        }
        for (gap, times) in &other.between_moved {
            *self.between_moved.entry(*gap).or_default() += times;
        }
        self.churn.objects += other.churn.objects;
        self.churn.same += other.churn.same;
        self.churn.changed += other.churn.changed;
        self.churn.new += other.churn.new;
        self.churn.removed += other.churn.removed;
        self.churn.still += other.churn.still;
        self.churn.empty += other.churn.empty;
        self.patches.merge(other.patches);
        self.snapshot += other.snapshot;
        self.parts.new_objects += other.parts.new_objects;
        self.parts.changed_objects += other.parts.changed_objects;
        self.parts.changed_fields += other.parts.changed_fields;
        self.parts.ids += other.parts.ids;
        self.parts.seats += other.parts.seats;
        self.parts.fields += other.parts.fields;
        for (name, count) in &other.by_field {
            self.by_field.entry(name.clone()).or_default().merge(*count);
        }
        self.costs.write_view += other.costs.write_view;
        self.costs.read_view += other.costs.read_view;
        self.costs.diff += other.costs.diff;
        self.costs.write_patch += other.costs.write_patch;
        self.costs.read_patch += other.costs.read_patch;
        self.costs.apply += other.costs.apply;
        self.costs.digest += other.costs.digest;
    }
}

/// Counts one zone's edit against what the zone held.
fn count_edit<T: Serialize + PartialEq + Keyed>(
    held: &[T],
    edit: &ZoneEdit<T>,
    parts: &mut PatchParts,
    changed: &mut u64,
    fresh: &mut u64,
    removed: &mut u64,
) {
    *removed += edit.removed.len() as u64;
    parts.ids += json_len(&edit.removed) + json_len(&edit.order);
    for object in &edit.upserts {
        if let Some(old) = held.iter().find(|old| old.handle() == object.handle()) {
            *changed += 1;
            parts.changed_objects += json_len(object);
            parts.changed_fields += field_bytes(old, object);
        } else {
            *fresh += 1;
            parts.new_objects += json_len(object);
        }
    }
}

/// What an object is told apart by, here.
trait Keyed {
    fn handle(&self) -> baylee_core::ids::ObjectId;
}

impl Keyed for baylee_view::PublicObject {
    fn handle(&self) -> baylee_core::ids::ObjectId {
        self.id
    }
}

impl Keyed for baylee_view::HandObject {
    fn handle(&self) -> baylee_core::ids::ObjectId {
        self.id
    }
}

/// Every seat's frames in one game, or in many, merged.
#[derive(Default)]
pub struct Wire {
    seats: BTreeMap<PlayerId, SeatWire>,
    /// Games that sent any frame.
    games: u64,
    /// Seats that were sent any frame, each game counted.
    seat_games: u64,
}

fn share(part: u64, whole: u64) -> f64 {
    if whole == 0 {
        0.0
    } else {
        100.0 * part as f64 / whole as f64
    }
}

/// `part / whole`, for a mean.
fn per(part: u64, whole: u64) -> f64 {
    if whole == 0 {
        0.0
    } else {
        part as f64 / whole as f64
    }
}

/// The share of gaps at each length, and their mean and percentiles.
fn gaps(between: &BTreeMap<u64, u64>) -> String {
    let times: u64 = between.values().sum();
    let frames: u64 = between.iter().map(|(gap, times)| gap * times).sum();
    let at = |wanted: u64| {
        let mut seen = 0;
        between
            .iter()
            .find(|(_, times)| {
                seen += **times;
                seen * 100 >= times_of(wanted, between)
            })
            .map_or(0, |(gap, _)| *gap)
    };
    format!(
        "questions={times} mean={:.2} p50={} p90={} p99={} max={} none={:.1}% one={:.1}% more={:.1}%",
        per(frames, times),
        at(50),
        at(90),
        at(99),
        between.keys().next_back().copied().unwrap_or(0),
        share(between.get(&0).copied().unwrap_or(0), times),
        share(between.get(&1).copied().unwrap_or(0), times),
        share(
            between
                .iter()
                .filter(|(gap, _)| **gap > 1)
                .map(|(_, t)| *t)
                .sum(),
            times
        ),
    )
}

fn times_of(percent: u64, between: &BTreeMap<u64, u64>) -> u64 {
    percent * between.values().sum::<u64>()
}

impl Wire {
    /// Counts one frame a seat was sent.
    pub fn frame(&mut self, seat: PlayerId, frame: &Envelope) {
        let seat = self.seats.entry(seat).or_default();
        let bytes = frame.encoded_len();
        seat.all.add(bytes);
        match &frame.msg {
            Some(Msg::StateDelta(delta)) => {
                seat.state(&delta.view_frame, &delta.log_json, bytes);
            }
            Some(Msg::ChoiceRequest(choice)) => seat.question(&choice.pending_json, bytes),
            Some(Msg::GameStatic(_)) => seat.statics.add(bytes),
            _ => seat.other.add(bytes),
        }
    }

    /// Adds another game's frames.
    pub fn merge(&mut self, other: &Self) {
        self.games += u64::from(!other.seats.is_empty());
        self.seat_games += other.seats.len() as u64;
        for (seat, wire) in &other.seats {
            self.seats.entry(*seat).or_default().merge(wire);
        }
    }

    /// Every seat as one, for the totals.
    fn all(&self) -> SeatWire {
        let mut all = SeatWire::default();
        for wire in self.seats.values() {
            all.merge(wire);
        }
        all
    }

    /// One line per seat, for a game's report.
    pub fn seat_lines(&self, game: &str) {
        for (seat, wire) in &self.seats {
            println!(
                "wire {game} seat={} frames={} bytes={} view={}/{} log={} question={}/{} legal={} static={}/{} other={}/{} patch={}",
                seat.get(),
                wire.all.frames,
                wire.all.bytes,
                wire.state.frames,
                wire.view_bytes,
                wire.log_bytes,
                wire.question.frames,
                wire.question.bytes,
                wire.priority.bytes,
                wire.statics.frames,
                wire.statics.bytes,
                wire.other.frames,
                wire.other.bytes,
                wire.patches.bytes,
            );
        }
    }

    /// The totals over every game and seat.
    pub fn report(&self) {
        let all = self.all();
        println!(
            "== wire: {} games, {} seats told over all of them",
            self.games, self.seat_games
        );
        println!(
            "per seat and game: {:.0} frames, {:.1} KB",
            per(all.all.frames, self.seat_games),
            per(all.all.bytes, self.seat_games) / 1000.0,
        );
        for (name, count) in [
            ("StateDelta", all.state),
            ("ChoiceRequest", all.question),
            ("GameStatic", all.statics),
            ("other", all.other),
        ] {
            println!(
                "  {name:<14} {:>9} frames {:>6.1}%  {:>13} bytes {:>6.1}%  mean {:>8.0}",
                count.frames,
                share(count.frames, all.all.frames),
                count.bytes,
                share(count.bytes, all.all.bytes),
                per(count.bytes, count.frames),
            );
        }
        println!(
            "  in StateDelta: view frames {} bytes ({:.1}% of all), whole views would be {}, log {} bytes ({:.1}%), {} frames carry only a log",
            all.frame_bytes,
            share(all.frame_bytes, all.all.bytes),
            all.view_bytes,
            all.log_bytes,
            share(all.log_bytes, all.all.bytes),
            all.repeats,
        );
        println!(
            "  LegalActions: {} priority questions, {} bytes, {:.1}% of the question bytes, {:.1}% of all bytes, mean {:.0}; {:.1}% equal the seat's last",
            all.priority.frames,
            all.priority.bytes,
            share(all.priority.bytes, all.question.bytes),
            share(all.priority.bytes, all.all.bytes),
            per(all.priority.bytes, all.priority.frames),
            share(all.legal_same, all.priority.frames),
        );
        println!(
            "  view frames between two questions: {}",
            gaps(&all.between)
        );
        println!(
            "  … that changed more than seq:     {}",
            gaps(&all.between_moved)
        );
        Self::report_patches(&all);
    }

    /// How far the views came to patches, and where the bytes went.
    fn report_patches(all: &SeatWire) {
        let churn = all.churn;
        println!(
            "  objects over {} patched views: {} seen, {:.1} a view; same {:.2}%, changed {:.2}%, new to their zone {:.2}%; removed {:.2} a view",
            all.patches.frames,
            churn.objects,
            per(churn.objects, all.patches.frames),
            share(churn.same, churn.objects),
            share(churn.changed, churn.objects),
            share(churn.new, churn.objects),
            per(churn.removed, all.patches.frames),
        );
        println!(
            "  views with no zone changed: {:.1}%; nothing but seq: {:.1}%",
            share(churn.still, all.patches.frames),
            share(churn.empty, all.patches.frames),
        );
        let whole = all.view_bytes.saturating_sub(all.snapshot);
        println!(
            "  as patches, JSON: {} bytes, mean {:.0}, against {} whole views after the first, mean {:.0}: {:.1}x fewer, {:.1}% of the view bytes",
            all.patches.bytes,
            per(all.patches.bytes, all.patches.frames),
            whole,
            per(whole, all.patches.frames),
            per(whole, all.patches.bytes),
            share(all.patches.bytes, whole),
        );
        let parts = all.parts;
        println!(
            "  a patch's bytes: new objects {:.1}%, changed objects {:.1}%, ids and orders {:.1}%, seat lines {:.1}%, other fields {:.1}%",
            share(parts.new_objects, all.patches.bytes),
            share(parts.changed_objects, all.patches.bytes),
            share(parts.ids, all.patches.bytes),
            share(parts.seats, all.patches.bytes),
            share(parts.fields, all.patches.bytes),
        );
        println!(
            "  changed objects by field instead: {} bytes against {}, {:.1}% of them",
            parts.changed_fields,
            parts.changed_objects,
            share(parts.changed_fields, parts.changed_objects),
        );
        let mut heavy: Vec<_> = all.by_field.iter().collect();
        heavy.sort_by_key(|(_, count)| std::cmp::Reverse(count.bytes));
        for (name, count) in heavy.iter().take(8) {
            println!(
                "    {name:<20} {:>8} times {:>11} bytes {:>5.1}% of patches, mean {:.0}",
                count.frames,
                count.bytes,
                share(count.bytes, all.patches.bytes),
                per(count.bytes, count.frames),
            );
        }
        let frames = all.state.frames.max(1);
        let patches = all.patches.frames.max(1);
        let costs = all.costs;
        let micros = |nanos: u128, of: u64| nanos as f64 / 1000.0 / of as f64;
        println!(
            "  µs each, wall: write view {:.1}, read view {:.1}, digest {:.1}, diff {:.1}, write patch {:.1}, read patch {:.1}, apply {:.1}",
            micros(costs.write_view, frames),
            micros(costs.read_view, frames),
            micros(costs.digest, frames),
            micros(costs.diff, patches),
            micros(costs.write_patch, patches),
            micros(costs.read_patch, patches),
            micros(costs.apply, patches),
        );
    }
}
