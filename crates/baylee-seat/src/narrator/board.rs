//! The seats and the board, whole, as one seat sees them.

use super::{Table, object_name, tag, words};
use baylee_core::ids::{Defender, ObjectId, PlayerId};
use baylee_core::types::{SubtypeSet, TypeSet};
use baylee_view::{DayNight, HandObject, ManaPoolView, PublicObject, StackItem, Step, TargetRef};
use std::fmt::Write as _;

/// The most cards one graveyard or exile line names; older ones are
/// counted.
const ZONE_NAMES: usize = 12;

/// One line per seat: life, library, hand, graveyard, pool, commanders.
pub(super) fn seats(table: &Table<'_>, out: &mut String) {
    let view = table.view;
    for seat in &view.seats {
        let p = seat.player;
        // No display name: the prefix carries each once, and a name a
        // person chose never stands in a message a model reads each turn.
        let who = if p == table.me() {
            format!("You ({})", Table::player_id(p))
        } else if table.ally(p) {
            format!("{} (teammate)", Table::player_id(p))
        } else {
            format!("{} (opponent)", Table::player_id(p))
        };
        let mut facts = vec![
            format!("{} life", seat.life),
            format!("library {}", seat.library_count),
            format!("hand {}", seat.hand_count),
            format!("graveyard {}", seat.graveyard_count),
        ];
        if seat.poison > 0 {
            facts.push(format!("poison {}", seat.poison));
        }
        if seat.energy > 0 {
            facts.push(format!("energy {}", seat.energy));
        }
        // An empty pool, which a pool almost always is at a decision, is
        // said by saying nothing.
        if !seat.mana_pool.is_empty() {
            facts.push(pool(&seat.mana_pool));
        }
        for commander in &seat.commanders {
            let tax = commander.casts.saturating_mul(2);
            let mut fact = format!(
                "commander {} {} ({}, cast {} from the command zone",
                commander.name,
                tag(commander.object),
                zone(table, commander.object),
                words::count(commander.casts as usize, "time"),
            );
            if tax > 0 {
                let _ = write!(fact, "; tax {{{tax}}}");
            }
            fact.push(')');
            facts.push(fact);
        }
        for damage in &seat.commander_damage {
            facts.push(format!(
                "commander damage taken: {} from {}",
                damage.amount,
                table.named(damage.source)
            ));
        }
        if view.monarch == Some(p) {
            facts.push("the monarch".into());
        }
        if seat.loss.is_some() {
            facts.push("has lost the game".into());
        }
        let _ = writeln!(out, "{who}: {}", facts.join(" · "));
    }
    match view.day_night {
        Some(DayNight::Day) => out.push_str("It is day.\n"),
        Some(DayNight::Night) => out.push_str("It is night.\n"),
        None => {}
    }
}

/// A mana pool that holds something: "pool {R}{R}{G}".
fn pool(pool: &ManaPoolView) -> String {
    let mut out = String::from("pool ");
    for (count, symbol) in [
        (pool.white, "W"),
        (pool.blue, "U"),
        (pool.black, "B"),
        (pool.red, "R"),
        (pool.green, "G"),
        (pool.colorless, "C"),
    ] {
        for _ in 0..count {
            let _ = write!(out, "{{{symbol}}}");
        }
    }
    let restricted = pool.restricted_total();
    if restricted > 0 {
        let _ = write!(out, " (+{restricted} restricted to some spells)");
    }
    out
}

/// Which zone an object is in, in words.
pub(super) fn zone(table: &Table<'_>, id: ObjectId) -> &'static str {
    let view = table.view;
    if view.battlefield.iter().any(|o| o.id == id) {
        "on the battlefield"
    } else if view.stack.iter().any(|o| o.id == id) {
        "on the stack"
    } else if view.command.iter().flatten().any(|o| o.id == id) {
        "in the command zone"
    } else if view.graveyards.iter().flatten().any(|o| o.id == id) {
        "in a graveyard"
    } else if view.exile.iter().flatten().any(|o| o.id == id) {
        "in exile"
    } else if view.hand.iter().any(|c| c.id == id) {
        "in your hand"
    } else {
        "somewhere you cannot see"
    }
}

/// The battlefield per controller, the stack, the hand, the other zones and
/// the combat.
pub(super) fn board(table: &Table<'_>, out: &mut String) {
    let view = table.view;
    let mut order = vec![table.me()];
    order.extend(view.opponents_in_turn_order());
    for p in order {
        battlefield(table, p, out);
    }
    stack(table, out);
    hand(table, "Your hand", &view.hand, out);
    for shared in &view.shared_hands {
        let title = format!("{} hand (shown to you)", table.whose(shared.player));
        hand(table, &capitalised(&title), &shared.cards, out);
    }
    for controlled in &view.controlled_hands {
        let title = format!(
            "{} hand (you decide for this player)",
            table.whose(controlled.player)
        );
        hand(table, &capitalised(&title), &controlled.cards, out);
    }
    zones(table, out);
    combat(table, out);
}

fn capitalised(text: &str) -> String {
    let mut chars = text.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(chars).collect()
    })
}

fn battlefield(table: &Table<'_>, p: PlayerId, out: &mut String) {
    let title = capitalised(&format!("{} battlefield", table.whose(p)));
    let objects: Vec<&PublicObject> = table
        .view
        .battlefield
        .iter()
        .filter(|o| o.controller == p)
        .collect();
    if objects.is_empty() {
        let _ = writeln!(out, "{title}: empty");
        return;
    }
    let _ = writeln!(out, "{title}:");
    let (lands, others): (Vec<&PublicObject>, Vec<&PublicObject>) =
        objects.into_iter().partition(|o| {
            o.types.contains(TypeSet::LAND)
                && !o.types.contains(TypeSet::CREATURE)
                && !o.status.is_face_down()
        });
    if !lands.is_empty() {
        let listed: Vec<String> = lands.iter().map(|o| land(table, o)).collect();
        let _ = writeln!(out, "  lands: {}", listed.join(", "));
    }
    for object in others {
        let _ = writeln!(out, "  {}", permanent(table, object));
    }
}

/// A land on the compact line: "Forest #21 (tapped)".
fn land(table: &Table<'_>, object: &PublicObject) -> String {
    let mut out = format!("{} {}", object_name(object), tag(object.id));
    let mut notes = Vec::new();
    if object.status.is_tapped() {
        notes.push("tapped".to_string());
    }
    notes.extend(details(table, object));
    if !notes.is_empty() {
        let _ = write!(out, " ({})", notes.join(", "));
    }
    out
}

/// A nonland permanent's line: "#45 Serra Angel · Creature — Angel · 4/4 ·
/// flying, vigilance · tapped".
pub(super) fn permanent(table: &Table<'_>, object: &PublicObject) -> String {
    let mut parts = vec![format!("{} {}", tag(object.id), object_name(object))];
    parts.push(type_line(object));
    if let Some(stats) = stats(object) {
        parts.push(stats);
    }
    let keywords = words::keywords(object.keywords);
    if !keywords.is_empty() {
        parts.push(keywords.join(", "));
    }
    let mut status = Vec::new();
    if object.status.is_tapped() {
        status.push("tapped".to_string());
    }
    if object.summoning_sick && object.types.contains(TypeSet::CREATURE) {
        status.push("summoning sick".into());
    }
    status.extend(details(table, object));
    if !status.is_empty() {
        parts.push(status.join(", "));
    }
    parts.join(" · ")
}

/// An object's type line as the view projects it: "Legendary Creature —
/// Goblin Warrior". Spelt as printed while the projection is the printed
/// face's (the view keeps subtypes as a set, which has no printed order); a
/// face-down permanent is what CR 708.2a makes it, and not a token.
pub(super) fn type_line(object: &PublicObject) -> String {
    if let Some(rules) = object.rules
        && !object.status.is_face_down()
        && let Some(face) = baylee_cards::by_index(rules.card)
            .and_then(|def| def.faces.get(usize::from(rules.face)))
        && face.types == object.types
        && face.supertypes == object.supertypes
        && SubtypeSet::from_slice(face.subtypes) == object.subtypes
    {
        return baylee_cards::pool::type_line(face).clone();
    }
    let mut line: Vec<&str> = object.supertypes.words().collect();
    line.extend(object.types.words());
    let mut out = line.join(" ");
    let subtypes = words::subtypes(object.subtypes);
    if !subtypes.is_empty() {
        let _ = write!(out, " — {subtypes}");
    }
    let face_down = object.status.is_face_down();
    if object.token.is_some() || (object.card.is_none() && object.rules.is_none() && !face_down) {
        out.push_str(" token");
    }
    out
}

/// Power and toughness, or loyalty.
pub(super) fn stats(object: &PublicObject) -> Option<String> {
    match (object.power, object.toughness, object.loyalty) {
        (Some(p), Some(t), _) if object.types.contains(TypeSet::CREATURE) => {
            Some(format!("{p}/{t}"))
        }
        (_, _, Some(loyalty)) => Some(format!("loyalty {loyalty}")),
        _ => None,
    }
}

/// What else a permanent carries: counters, damage, what it is attached
/// to, combat, a controller who is not its owner, choices made for it.
fn details(table: &Table<'_>, object: &PublicObject) -> Vec<String> {
    let mut out = Vec::new();
    if object.status.is_phased_out() {
        out.push("phased out".into());
    }
    if object.commander {
        out.push(format!("{} commander", table.whose(object.owner)));
    }
    for counter in &object.counters {
        if counter.kind == baylee_view::CounterKind::Loyalty {
            continue;
        }
        let noun = format!("{} counter", words::counter(counter.kind));
        out.push(words::count(usize::from(counter.count), &noun));
    }
    if object.damage > 0 {
        out.push(format!("{} damage marked", object.damage));
    }
    if let Some(host) = object.attached_to {
        out.push(format!("attached to {}", table.named(host)));
    }
    let combat = &table.view.combat;
    if let Some(attack) = combat.attackers.iter().find(|a| a.creature == object.id) {
        out.push(format!("attacking {}", defender(table, attack.defending)));
        if attack.blocked {
            out.push("blocked".into());
        }
    }
    for block in combat.blockers.iter().filter(|b| b.blocker == object.id) {
        out.push(format!("blocking {}", table.named(block.attacker)));
    }
    if object.controller != object.owner {
        out.push(format!("owned by {}", table.player(object.owner)));
    }
    if let Some(subtype) = object.chosen_subtype
        && let Some(name) = baylee_core::generated::subtypes::name(subtype)
    {
        out.push(format!("chosen type {name}"));
    }
    if let Some(named) = object.chosen_name
        && let Some(def) = baylee_cards::by_index(named.card)
        && let Some(face) = def.faces.get(usize::from(named.face))
    {
        out.push(format!("chosen name {}", face.name));
    }
    if let Some([left, right]) = object.unlocked_doors {
        let open: Vec<&str> = [(left, "left"), (right, "right")]
            .into_iter()
            .filter_map(|(open, side)| open.then_some(side))
            .collect();
        if !open.is_empty() {
            out.push(format!("unlocked: {}", open.join(" and ")));
        }
    }
    out
}

/// What an attack is aimed at: "you", "P2", "Jace #90".
pub(super) fn defender(table: &Table<'_>, defending: Defender) -> String {
    match defending {
        Defender::Player(p) => table.player(p),
        Defender::Planeswalker(id) => table.named(id),
    }
}

fn stack(table: &Table<'_>, out: &mut String) {
    let view = table.view;
    if view.stack.is_empty() {
        out.push_str("Stack: empty\n");
        return;
    }
    out.push_str("Stack (top first; the top resolves next):\n");
    for object in view.stack.iter().rev() {
        let _ = writeln!(out, "  {}", stack_line(table, object));
    }
}

/// A stack object: a spell with its targets, or an ability with its text.
pub(super) fn stack_line(table: &Table<'_>, object: &PublicObject) -> String {
    let owner = table.whose(object.controller);
    let mut line = if let Some(StackItem::Ability {
        source,
        ability,
        text,
        rules,
        ..
    }) = object.stack_item
    {
        let from = if table.visible(source) {
            table.named(source)
        } else {
            object_name(object)
        };
        let mut line = format!("{} {owner} ability of {from}", tag(object.id));
        let sentence = rules.and_then(|rules| {
            text.and_then(|t| {
                baylee_cards::oracle::sentence(rules.card, usize::from(t.face), t.line)
            })
            .or_else(|| {
                let index = ability?.index;
                let at =
                    baylee_cards::lines::ability_line(rules.card, usize::from(rules.face), index)?;
                baylee_cards::oracle::sentence(rules.card, usize::from(rules.face), at.line)
            })
        });
        if let Some(sentence) = sentence {
            let _ = write!(line, ": \"{sentence}\"");
        }
        line
    } else {
        let kind = words::types(object.types, baylee_core::types::SupertypeSet::EMPTY);
        format!(
            "{} {} · {owner} {kind} spell",
            tag(object.id),
            object_name(object)
        )
    };
    if !object.targets.is_empty() {
        let targets: Vec<String> = object
            .targets
            .iter()
            .map(|target| match *target {
                TargetRef::Object(source) => table.named_target(source),
                TargetRef::Player(p) => table.player(p),
            })
            .collect();
        let _ = write!(line, " · targets {}", targets.join(", "));
    }
    line
}

fn hand(table: &Table<'_>, title: &str, cards: &[HandObject], out: &mut String) {
    if cards.is_empty() {
        let _ = writeln!(out, "{title}: empty");
        return;
    }
    let _ = writeln!(out, "{title}:");
    for card in cards {
        let _ = writeln!(out, "  {}", hand_line(table, card));
    }
}

/// A card in a hand: "#50 Lightning Bolt {R} · Instant".
pub(super) fn hand_line(table: &Table<'_>, card: &HandObject) -> String {
    let printed = baylee_cards::by_index(card.card.index)
        .and_then(|def| def.faces.get(usize::from(card.card.face)));
    let mut line = format!("{} {}", tag(card.id), card.name);
    let type_line = printed.map_or_else(
        || words::types(card.types, baylee_core::types::SupertypeSet::EMPTY),
        baylee_cards::pool::type_line,
    );
    if let Some(face) = printed {
        let cost = face.mana_cost.to_string();
        if !cost.is_empty() {
            let _ = write!(line, " {cost}");
        }
    }
    let _ = write!(line, " · {type_line}");
    if let Some(stats) = printed.and_then(baylee_cards::pool::stats) {
        let _ = write!(line, " · {stats}");
    }
    if card.commander {
        let _ = write!(line, " · {} commander", table.whose(table.me()));
    }
    line
}

/// Graveyards, exile, the command zone, and cards being looked at.
fn zones(table: &Table<'_>, out: &mut String) {
    let view = table.view;
    for (seat, cards) in view.graveyards.iter().enumerate() {
        if cards.is_empty() {
            continue;
        }
        let p = PlayerId::new(u8::try_from(seat).unwrap_or(u8::MAX));
        let title = capitalised(&format!("{} graveyard", table.whose(p)));
        zone_line(table, &title, cards, out);
    }
    for (seat, cards) in view.exile.iter().enumerate() {
        if cards.is_empty() {
            continue;
        }
        let p = PlayerId::new(u8::try_from(seat).unwrap_or(u8::MAX));
        let title = format!("Exiled cards {} owns", table.player(p)).replace("you owns", "you own");
        zone_line(table, &title, cards, out);
    }
    for (seat, cards) in view.command.iter().enumerate() {
        if cards.is_empty() {
            continue;
        }
        let p = PlayerId::new(u8::try_from(seat).unwrap_or(u8::MAX));
        let title = capitalised(&format!("{} command zone", table.whose(p)));
        zone_line(table, &title, cards, out);
    }
    if !view.looking_at.is_empty() {
        zone_line(table, "You are looking at", &view.looking_at, out);
    }
    if !view.library_tops.is_empty() {
        zone_line(table, "Known top of a library", &view.library_tops, out);
    }
}

/// One zone on one line, newest last: "Your graveyard (3): Shock #12, …".
fn zone_line(table: &Table<'_>, title: &str, cards: &[PublicObject], out: &mut String) {
    let older = cards.len().saturating_sub(ZONE_NAMES);
    let mut named: Vec<String> = cards[older..]
        .iter()
        .map(|object| {
            let mut name = format!("{} {}", object_name(object), tag(object.id));
            let mut notes = Vec::new();
            if let Some(cost) = object.flashback {
                notes.push(format!("flashback {cost}"));
            }
            if object.suspended {
                let time = object.counter_count(baylee_view::CounterKind::Time);
                notes.push(format!(
                    "suspended, {}",
                    words::count(usize::from(time), "time counter")
                ));
            }
            if object.commander {
                notes.push(format!("{} commander", table.whose(object.owner)));
            }
            if !notes.is_empty() {
                let _ = write!(name, " ({})", notes.join(", "));
            }
            name
        })
        .collect();
    if older > 0 {
        named.insert(0, format!("{older} older"));
    }
    let _ = writeln!(out, "{title} ({}): {}", cards.len(), named.join(", "));
}

fn combat(table: &Table<'_>, out: &mut String) {
    let combat = &table.view.combat;
    if !combat.is_active() {
        return;
    }
    out.push_str("Combat:\n");
    for attack in &combat.attackers {
        let blockers: Vec<String> = combat
            .blockers_of(attack.creature)
            .map(|id| table.named(id))
            .collect();
        // "Not blocked" only once blocks are declared: before that it reads
        // as a fact about a block nobody has chosen yet.
        let declared = matches!(
            table.view.step,
            Step::CombatDamageFirst | Step::CombatDamage | Step::CombatEnd
        );
        let state = if !blockers.is_empty() {
            format!(" · blocked by {}", blockers.join(", "))
        } else if attack.blocked {
            " · blocked".into()
        } else if declared {
            " · not blocked".into()
        } else {
            String::new()
        };
        let _ = writeln!(
            out,
            "  {} attacks {}{state}",
            table.named(attack.creature),
            defender(table, attack.defending)
        );
    }
}
