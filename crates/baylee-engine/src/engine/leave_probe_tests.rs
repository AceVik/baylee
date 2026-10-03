//! The leave-the-battlefield probe: does a permanent's ability stop when
//! the permanent is no longer in play? (Verification rung L4 for the cards a
//! trainer plays with; `docs/verification-hooks.md` §"L4: the leave probe".)
//!
//! Every card of the pool whose front face is a permanent and whose coverage
//! is `Implemented` is seated under seat 0 on a small, symmetric board and
//! taken away along seven routes, and an Aura, Equipment or Fortification
//! along an eighth. The route is resolved by
//! [`crate::resolve::run`], the function every resolving spell and ability
//! runs, with the effect a card would print for it:
//!
//! | route       | effect                                   | controlled by |
//! |-------------|------------------------------------------|---------------|
//! | `destroy`   | `Effect::Destroy`, the card its target    | seat 1        |
//! | `exile`     | `Effect::Exile`                           | seat 1        |
//! | `bounce`    | `Effect::ReturnToHand` (its owner's hand) | seat 1        |
//! | `library`   | `Effect::PutSourceOnTopOfLibrary`         | seat 0        |
//! | `sacrifice` | `Effect::SacrificeSelf`                   | seat 0        |
//! | `phase_out` | `Effect::PhaseOut` (CR 702.26b)           | seat 0        |
//! | `phase_host`| `Effect::PhaseOut` on what the card is    | seat 0        |
//! |             | attached to (CR 702.26g)                  |               |
//! | `control`   | `Effect::ChangeController`, which is      | seat 1        |
//! |             | `resolve::gain_control`'s layer-2 effect  |               |
//!
//! After a route the engine settles by itself (a priority pass runs the
//! machine: statics synced, state-based actions, triggers), and then:
//!
//! - **Residue.** No continuous effect from the card with origin `Static`
//!   is in the table, none that lasts while its source is on the battlefield
//!   (CR 611.2b, whatever its origin), and no replacement entry. The layer
//!   projection is a function of that table and the board, so a table
//!   without them and a fresh projection mean nothing of the card is applied
//!   to another object. An effect a resolution made that lasts a turn is
//!   left alone (CR 611.2a), as are one-shot results.
//! - **Activation.** Seat 0 is offered none of the card's battlefield
//!   abilities from where it is now (a cycling ability from a hand is).
//! - **Triggers.** A turn cycle of ordinary events follows ([`battery`]),
//!   and none of the card's own triggered abilities may trigger in it.
//! - **Fresh.** [`Engine::projection_is_fresh`] after settling and at the end.
//! - **Unaffected** (the phasing routes). While the card is phased out,
//!   seat 1 resolves an effect that fixes the set it affects as it begins:
//!   every permanent gets +0/+0 for the rest of the game (CR 611.2c). The
//!   set leaves the card out (CR 702.26e).
//!
//! `phase_out` asks the same of a card that stays on the battlefield
//! phased out (but for a resolution's effect lasting while it is on the
//! battlefield: see [`residue`]), and then that its statics and
//! replacements are back once it has phased in at its controller's next
//! untap step. `phase_host` phases out the permanent the card is attached
//! to instead: the card must phase out with it (CR 702.26g) and phase in
//! with it, no sooner, still attached, and then everything `phase_out` asks
//! holds as well. `control` asks the
//! opposite: every static and replacement of the card answers to seat 1 now
//! (CR 109.5), seat 0 is offered none of its abilities, what it triggers is
//! seat 1's, and on a card that is attached to nothing its statics touch the
//! two sides' identical bystanders the other way round ([`swap`]).
//!
//! The sweep is `#[ignore]`d and writes `leave.jsonl` into
//! `BAYLEE_LEAVE_LOG` when that is set. The test the gate runs is
//! [`representative_cards_leave_cleanly_by_every_route`].

use super::testkit::{
    Duel, RegistryLookup, Rest, answer_one, basic_forest, card_index, is_permanent, mine,
    quiet_creature, spawn_named, walk_to_own_main,
};
use super::*;
use crate::effects::{EffectFilter, EffectOrigin};
use crate::object::{Characteristics, Status};
use baylee_cards_dsl::{AbilityDef, ActivationZone, CardDef, Duration, Effect, Filter, TargetSpec};
use baylee_core::ids::{AbilityRef, CardIndex, Defender};
use std::fmt::Write as _;

/// The environment variable naming the directory `leave.jsonl` goes into.
pub(super) const VAR: &str = "BAYLEE_LEAVE_LOG";

/// The seed every board is built at. Seat 0 takes the first turn at it,
/// which [`representative_cards_leave_cleanly_by_every_route`] pins.
const SEED: u64 = 4_211;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// The ways off the battlefield (and the one way to another side of it).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Route {
    Destroy,
    Exile,
    Bounce,
    Library,
    Sacrifice,
    PhaseOut,
    PhaseHost,
    Control,
}

impl Route {
    const ALL: [Self; 8] = [
        Self::Destroy,
        Self::Exile,
        Self::Bounce,
        Self::Library,
        Self::Sacrifice,
        Self::PhaseOut,
        Self::PhaseHost,
        Self::Control,
    ];

    /// Whether the route is asked of `def`: `phase_host` only of what
    /// attaches to a permanent (CR 702.26g names the three).
    fn applies(self, def: &CardDef) -> bool {
        use baylee_core::generated::subtypes::{artifact, enchantment};
        self != Self::PhaseHost
            || def.faces[0].subtypes.iter().any(|s| {
                [
                    enchantment::AURA,
                    artifact::EQUIPMENT,
                    artifact::FORTIFICATION,
                ]
                .contains(s)
            })
    }

    /// Whether the card stays on the battlefield phased out.
    const fn phases(self) -> bool {
        matches!(self, Self::PhaseOut | Self::PhaseHost)
    }

    /// The name the output file uses.
    const fn name(self) -> &'static str {
        match self {
            Self::Destroy => "destroy",
            Self::Exile => "exile",
            Self::Bounce => "bounce",
            Self::Library => "library",
            Self::Sacrifice => "sacrifice",
            Self::PhaseOut => "phase_out",
            Self::PhaseHost => "phase_host",
            Self::Control => "control",
        }
    }

    /// Whether the card is meant to be gone from the battlefield.
    const fn leaves(self) -> bool {
        !matches!(self, Self::PhaseOut | Self::PhaseHost | Self::Control)
    }

    /// Who controls the effect, whether it names the card as its target
    /// (rather than as its source), and what it does.
    const fn effect(self) -> (PlayerId, bool, &'static [Effect]) {
        match self {
            Self::Destroy => (P1, true, DESTROY),
            Self::Exile => (P1, true, EXILE),
            Self::Bounce => (P1, true, BOUNCE),
            Self::Library => (P0, false, LIBRARY),
            Self::Sacrifice => (P0, false, SACRIFICE),
            Self::PhaseOut | Self::PhaseHost => (P0, false, PHASE_OUT),
            Self::Control => (P1, true, CONTROL),
        }
    }
}

static DESTROY: &[Effect] = &[Effect::Destroy {
    target: TargetSpec::Object(&Filter::Any),
    no_regen: false,
}];
static EXILE: &[Effect] = &[Effect::Exile {
    target: TargetSpec::Object(&Filter::Any),
}];
static BOUNCE: &[Effect] = &[Effect::ReturnToHand {
    target: TargetSpec::Object(&Filter::Any),
}];
static LIBRARY: &[Effect] = &[Effect::PutSourceOnTopOfLibrary];
static SACRIFICE: &[Effect] = &[Effect::SacrificeSelf];
static PHASE_OUT: &[Effect] = &[Effect::PhaseOut { target: None }];
/// The unaffected check's effect: +0/+0 to every permanent for the rest of
/// the game, a set fixed as the effect begins (CR 611.2c) that changes no
/// number the rest of the probe reads.
static EVERY_PERMANENT_GETS_NOTHING: &[Effect] = &[Effect::CreateContinuousEffect {
    layer: baylee_cards_dsl::Layer::PtModify,
    filter: &Filter::Any,
    modifier: baylee_cards_dsl::Modifier::ModifyPT(0, 0),
    duration: Duration::Indefinitely,
}];
static CONTROL: &[Effect] = &[Effect::ChangeController {
    new_controller: baylee_cards_dsl::PlayerRel::You,
}];

/// What the probe concluded about one (card, route).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Verdict {
    Ok,
    Lingers,
    Stale,
    Skipped,
}

impl Verdict {
    const fn name(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Lingers => "lingers",
            Self::Stale => "stale",
            Self::Skipped => "skipped",
        }
    }
}

/// One line of `leave.jsonl`.
#[derive(Clone, Debug)]
struct Line {
    card: CardIndex,
    route: Route,
    verdict: Verdict,
    detail: String,
    /// Where the trigger walk stopped short, if it did. Counted, never
    /// written: `detail` is empty when the verdict is `ok`.
    short: Option<String>,
    /// What the trigger walk made happen.
    walk: Walk,
}

impl Line {
    fn json(&self) -> String {
        format!(
            "{{\"card\": {}, \"route\": \"{}\", \"verdict\": \"{}\", \"detail\": {}}}",
            self.card.get(),
            self.route.name(),
            self.verdict.name(),
            crate::ability_log::json_str(&self.detail)
        )
    }
}

/// What failed, as it was found.
#[derive(Default)]
struct Findings {
    lingers: Vec<String>,
    stale: Vec<String>,
    short: Option<String>,
    walk: Walk,
}

impl Findings {
    fn fresh(&mut self, engine: &Engine<RegistryLookup>, when: &str) {
        if !engine.projection_is_fresh() {
            self.stale.push(format!("projection stale {when}"));
        }
    }

    fn line(self, card: CardIndex, route: Route) -> Line {
        let (verdict, mut detail) = if !self.lingers.is_empty() {
            let mut all = self.lingers;
            all.extend(self.stale);
            (Verdict::Lingers, all.join("; "))
        } else if !self.stale.is_empty() {
            (Verdict::Stale, self.stale.join("; "))
        } else {
            (Verdict::Ok, String::new())
        };
        shorten(&mut detail);
        Line {
            card,
            route,
            verdict,
            detail,
            short: self.short,
            walk: self.walk,
        }
    }
}

fn skipped(card: CardIndex, route: Route, why: impl Into<String>) -> Line {
    let mut detail = why.into();
    shorten(&mut detail);
    Line {
        card,
        route,
        verdict: Verdict::Skipped,
        detail,
        short: None,
        walk: Walk::default(),
    }
}

/// "short reason": a line stays readable when a card lingers ten ways.
fn shorten(detail: &mut String) {
    const MAX: usize = 400;
    if detail.len() > MAX {
        let mut cut = MAX;
        while !detail.is_char_boundary(cut) {
            cut -= 1;
        }
        detail.truncate(cut);
        detail.push('…');
    }
}

/// A `Debug` spelling cut to its variant's name: `ModifyPT(1, 1)` is
/// `ModifyPT`.
fn variant(debug: &str) -> &str {
    let end = debug.find([' ', '(', '{']).unwrap_or(debug.len());
    &debug[..end]
}

// --- the board -------------------------------------------------------------

/// The two basic lands each side has: a Forest (the Elf in hand costs
/// `{G}`) and the next basic type, so a static reading land types has two.
fn board_lands() -> [CardIndex; 2] {
    let forest = basic_forest();
    let other = baylee_cards::decks::basic_lands()
        .into_iter()
        .flatten()
        .find(|&basic| basic != forest)
        .unwrap_or(forest);
    [forest, other]
}

/// A seated card and the bystanders around it.
struct Table {
    engine: Engine<RegistryLookup>,
    /// The card under probe. Its id is stable across zones (CR 400.7 bumps
    /// the version instead), so it also names "its old object".
    card: ObjectId,
    /// Each side's Elf and two lands, in that order, by owner.
    sides: [[Option<ObjectId>; 3]; 2],
    /// What the card is attached to (an Aura, an Equipment). The trigger
    /// walk spares it: a host that dies takes a phased-out Aura with it
    /// when the Aura phases in (CR 704.5m), which says nothing about the
    /// Aura's own statics.
    host: Option<ObjectId>,
}

/// Seats `def` under seat 0 and walks to seat 0's first main phase.
fn seat(def: &'static CardDef) -> Result<Table, String> {
    let elf = quiet_creature();
    let [a, b] = board_lands();
    let mut engine = Duel::new(SEED, basic_forest())
        .battlefield(0, &[def.index, elf, a, b])
        .battlefield(1, &[elf, a, b])
        .hand(0, &[elf, a])
        .hand(1, &[elf, a])
        .start();
    let card = *mine(&engine, P0, def.index, Zone::Battlefield)
        .first()
        .ok_or("setup: the card was not seated")?;
    let sides = [P0, P1].map(|seat| {
        [elf, a, b].map(|index| {
            engine
                .state()
                .zones
                .list(ZoneLocation::Battlefield)
                .iter()
                .copied()
                .find(|&id| {
                    id != card
                        && engine.state().object(id).is_some_and(|o| {
                            o.owner == seat && o.card.is_some_and(|c| c.index == index)
                        })
                })
        })
    });
    let host = fix_up(&mut engine, def, card, &sides)?;
    if !walk_to_own_main(&mut engine, P0) && !walk_answering(&mut engine) {
        return Err(format!(
            "setup: stopped before seat 0's main phase at {}",
            variant(&format!("{:?}", engine.pending()))
        ));
    }
    drain_stack(&mut engine)?;
    let seated = engine.state().object(card).is_some_and(|o| {
        o.zone == Zone::Battlefield && o.controller == P0 && !o.status.contains(Status::PHASED_OUT)
    });
    if !seated {
        let zone = engine.state().object(card).map(|o| o.zone);
        return Err(format!(
            "setup: not on the battlefield under seat 0 after setup (in {zone:?})"
        ));
    }
    Ok(Table {
        engine,
        card,
        sides,
        host,
    })
}

/// What `starting_battlefield` cannot say about a permanent: what an Aura
/// enchants, what an Equipment equips, a planeswalker's loyalty (#304).
/// Before the first state-based check, which would bin all three.
fn fix_up(
    engine: &mut Engine<RegistryLookup>,
    def: &'static CardDef,
    card: ObjectId,
    sides: &[[Option<ObjectId>; 3]; 2],
) -> Result<Option<ObjectId>, String> {
    use baylee_core::generated::subtypes::{artifact, enchantment};
    let face = &def.faces[0];
    let aura = face.subtypes.contains(&enchantment::AURA);
    let equipment = face.subtypes.contains(&artifact::EQUIPMENT);
    let state = engine
        .dev_state_mut(P0)
        .ok_or("setup: the harness may not set the board up")?;
    let mut attached = None;
    if aura {
        let restriction = def.abilities_for_face(0).iter().find_map(|a| match a {
            AbilityDef::Spell { effects, .. } => effects.iter().find_map(|e| match e {
                Effect::AttachSelf {
                    target: TargetSpec::Object(filter),
                } => Some(*filter),
                _ => None,
            }),
            _ => None,
        });
        // Seat 0's Elf first, then seat 1's, then the lands: the first
        // bystander the Aura may enchant (CR 303.4c, as the attachment
        // state-based action reads it).
        let order = [
            sides[0][0],
            sides[1][0],
            sides[0][1],
            sides[0][2],
            sides[1][1],
            sides[1][2],
        ];
        let host = order.into_iter().flatten().find(|&host| {
            state.object(host).is_some_and(|h| {
                restriction.is_none_or(|filter| crate::eval::matches(filter, state, h, P0, card))
            })
        });
        let Some(host) = host else {
            return Err("setup: an Aura with nothing on the probe board to enchant".into());
        };
        if let Some(obj) = state.object_mut(card) {
            obj.attached_to = Some(host);
        }
        attached = Some(host);
    } else if equipment && let Some(elf) = sides[0][0] {
        if let Some(obj) = state.object_mut(card) {
            obj.attached_to = Some(elf);
        }
        attached = Some(elf);
    }
    if let Some(loyalty) = face.loyalty
        && let Some(obj) = state.object_mut(card)
    {
        obj.counters
            .set(baylee_cards_dsl::CounterKind::Loyalty, loyalty);
    }
    state.invalidate_projections();
    Ok(attached)
}

/// The fallback walker: keeps opening hands and answers everything else
/// with the kit's default answers until seat 0 holds priority in its first
/// main phase.
fn walk_answering(engine: &mut Engine<RegistryLookup>) -> bool {
    for _ in 0..300 {
        if matches!(engine.state().turn.phase, Phase::FirstMain)
            && engine.state().turn.active == P0
            && matches!(engine.pending(), Pending::Priority { player, .. } if *player == P0)
        {
            return true;
        }
        let (player, action) = match engine.pending().clone() {
            Pending::Mulligan { player, .. } => (player, PlayerAction::MulliganKeep),
            _ => match answer_one(engine) {
                Ok(pair) => pair,
                Err(_) => return false,
            },
        };
        if engine.apply(player, action).is_err() {
            return false;
        }
    }
    false
}

/// Answers until the stack is empty and somebody holds priority.
fn drain_stack(engine: &mut Engine<RegistryLookup>) -> Result<(), String> {
    for _ in 0..300 {
        if engine.state().zones.stack_is_empty()
            && matches!(engine.pending(), Pending::Priority { .. })
        {
            return Ok(());
        }
        let (player, action) = answer_one(engine).map_err(|rest| rest_name(&rest))?;
        engine
            .apply(player, action)
            .map_err(|e| format!("an answer out of the question was refused: {e:?}"))?;
    }
    Err("the stack never emptied".into())
}

fn rest_name(rest: &Rest) -> String {
    match rest {
        Rest::Reached => "reached".into(),
        Rest::Over => "the game ended".into(),
        Rest::Unanswered(what) => format!("unanswered {what}"),
        Rest::Stalled => "stalled".into(),
        Rest::Refused(why) => format!("refused: {why}"),
    }
}

// --- what the card has registered ------------------------------------------

/// The card's live statics and replacements, by name, sorted.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Registered {
    statics: Vec<String>,
    replacements: Vec<String>,
}

fn registered(state: &GameState, card: ObjectId) -> Registered {
    let mut statics: Vec<String> = state
        .effects
        .iter()
        .filter(|fx| fx.source == Some(card) && fx.origin == EffectOrigin::Static)
        .map(|fx| format!("{:?}", fx.modifier))
        .collect();
    let mut replacements: Vec<String> = state
        .replacement_rules
        .iter()
        .filter(|r| r.source == card)
        .map(|r| format!("{:?}", r.rule))
        .collect();
    statics.sort();
    replacements.sort();
    Registered {
        statics,
        replacements,
    }
}

/// What of the card is still in force although it is not in play.
///
/// An effect a resolution made that lasts while its source is on the
/// battlefield is residue once the card has `left`. A phased-out card has
/// not: CR 702.26f ends only a "for as long as" duration that tracks it,
/// and the table does not tell one of those from an effect that lasts
/// indefinitely on the permanent itself ("~ gains …"), so the probe asks
/// that of the leaving routes only.
fn residue(state: &GameState, card: ObjectId, left: bool, found: &mut Findings, when: &str) {
    for fx in state.effects.iter().filter(|fx| fx.source == Some(card)) {
        let modifier = format!("{:?}", fx.modifier);
        if fx.origin == EffectOrigin::Static {
            found.lingers.push(format!(
                "static {} still applies {when}",
                variant(&modifier)
            ));
        } else if left && fx.duration == Duration::WhileSourceOnBattlefield {
            found.lingers.push(format!(
                "{} lasting while its source is on the battlefield still applies {when}",
                variant(&modifier)
            ));
        }
    }
    for entry in state.replacement_rules.iter().filter(|r| r.source == card) {
        found.lingers.push(format!(
            "replacement {} still registered {when}",
            variant(&format!("{:?}", entry.rule))
        ));
    }
}

/// Seat 0 offered one of the card's battlefield abilities while the card is
/// somewhere else, or phased out, or seat 1's.
fn offered(engine: &Engine<RegistryLookup>, card: ObjectId, found: &mut Findings, when: &str) {
    let Some(obj) = engine.state().object(card) else {
        return;
    };
    let legal = engine.compute_legal(P0);
    if legal.mana_abilities.contains(&card) {
        found
            .lingers
            .push(format!("its mana ability is offered to seat 0 {when}"));
    }
    let list = obj.ability_list(&RegistryLookup).abilities;
    for &(_, index) in legal.abilities.iter().filter(|(source, _)| *source == card) {
        let on_the_battlefield = match list.get(index as usize) {
            Some(
                AbilityDef::Activated { zone, .. } | AbilityDef::ActivatedConditional { zone, .. },
            ) => *zone == ActivationZone::Battlefield,
            Some(AbilityDef::Loyalty { .. }) => true,
            _ => false,
        };
        if on_the_battlefield {
            found
                .lingers
                .push(format!("its ability {index} is offered to seat 0 {when}"));
        }
    }
}

// --- the routes --------------------------------------------------------------

/// Resolves the route's effect, then lets the engine settle.
fn take_away(table: &mut Table, route: Route) -> Result<(), String> {
    let (controller, targeted, effects) = route.effect();
    let card = table.card;
    let source = match route {
        Route::PhaseHost => table.host.ok_or("route: attached to nothing")?,
        _ if controller == P0 => card,
        _ => seat_one_source(table)?,
    };
    resolve(table, controller, source, targeted.then_some(card), effects)?;
    settle(&mut table.engine)
}

/// Seat 1's effect needs a source seat 1 controls: the first of its
/// bystanders still standing.
fn seat_one_source(table: &Table) -> Result<ObjectId, String> {
    table.sides[1]
        .iter()
        .flatten()
        .copied()
        .find(|&id| {
            table
                .engine
                .state()
                .object(id)
                .is_some_and(|o| o.zone == Zone::Battlefield)
        })
        .ok_or_else(|| "route: seat 1 has nothing left to be the effect's source".into())
}

/// Runs `effects` through [`crate::resolve::run`] as `controller`'s, from
/// `source`, targeting `target` when there is one.
fn resolve(
    table: &mut Table,
    controller: PlayerId,
    source: ObjectId,
    target: Option<ObjectId>,
    effects: &[Effect],
) -> Result<(), String> {
    let state = table
        .engine
        .dev_state_mut(controller)
        .ok_or("route: the harness may not touch the board")?;
    let mut res = crate::resolve::Resolution {
        source,
        on_stack: source,
        controller,
        effects: effects.to_vec(),
        pc: 0,
        targets: target.into_iter().collect(),
        second_targets: smallvec::SmallVec::new(),
        x: None,
        chosen_player: None,
        target_players: baylee_core::ids::SeatSet::new(),
        event_object: None,
        awaiting: None,
        targeted: false,
        mana_ability: false,
        countered_source: None,
        target_lki: None,
        subject: crate::resolve::SubjectContext::default(),
        event_mana: None,
        retarget_left: None,
    };
    if let crate::resolve::Flow::Wait(pending) = crate::resolve::run(state, &mut res) {
        return Err(format!(
            "route: the effect asked {}",
            variant(&format!("{pending:?}"))
        ));
    }
    Ok(())
}

/// The unaffected check: seat 1 resolves [`EVERY_PERMANENT_GETS_NOTHING`]
/// while the card is phased out, and the set it fixes leaves the card out
/// (CR 702.26e). A bystander of seat 1's that is phased in is in it, or the
/// check proved nothing.
fn unaffected(table: &mut Table, found: &mut Findings) -> Result<(), String> {
    let naming = |table: &Table, id: ObjectId| {
        table
            .engine
            .state()
            .effects
            .iter()
            .filter(|fx| matches!(fx.filter, EffectFilter::ObjectIs(named, _) if named == id))
            .count()
    };
    let control = table.sides[1]
        .iter()
        .flatten()
        .copied()
        .find(|&id| {
            table
                .engine
                .state()
                .battlefield_seen()
                .any(|seen| seen == id)
        })
        .ok_or("unaffected: seat 1 has no bystander phased in")?;
    let (card_before, control_before) = (naming(table, table.card), naming(table, control));
    let source = seat_one_source(table)?;
    resolve(table, P1, source, None, EVERY_PERMANENT_GETS_NOTHING)?;
    if naming(table, control) == control_before {
        return Err("unaffected: the board-wide effect left out a bystander phased in".into());
    }
    if naming(table, table.card) != card_before {
        found.lingers.push(
            "an effect that began while it was phased out took it into its set (CR 702.26e)".into(),
        );
    }
    Ok(())
}

/// One pass of priority runs the machine (statics, state-based actions,
/// triggers, CR 117.5); then everything asked is answered until the stack
/// is empty and somebody holds priority again.
fn settle(engine: &mut Engine<RegistryLookup>) -> Result<(), String> {
    engine.refresh_offer();
    let Pending::Priority { player, .. } = engine.pending().clone() else {
        return Err(format!(
            "settle: no priority to pass, {}",
            variant(&format!("{:?}", engine.pending()))
        ));
    };
    engine
        .apply(player, PlayerAction::PassPriority)
        .map_err(|e| format!("settle: pass refused: {e:?}"))?;
    drain_stack(engine)
}

// --- the trigger walk --------------------------------------------------------

/// A turn cycle of ordinary events after the card has gone: both players
/// play a land and cast a creature in their main phase (a spell cast, a
/// land and a creature entering, mana tapped), seat 1 attacks with what it
/// may, both original Elves (but the card's host) die in seat 1's second
/// main phase, and every step of both turns begins (upkeep, draw, combat,
/// end), up to seat 0's next first main phase.
///
/// Returns the journal position it started at, and where it stopped short
/// if it did.
fn battery(table: &mut Table) -> (u64, Option<String>) {
    let mark = table.engine.state().journal.last_seq();
    let start = table.engine.state().turn.number;
    let mut acted: Vec<(u32, PlayerId)> = Vec::new();
    let mut killed = false;
    for _ in 0..800 {
        let engine = &mut table.engine;
        let turn = engine.state().turn;
        let quiet = engine.state().zones.stack_is_empty();
        if turn.number >= start + 2
            && turn.active == P0
            && turn.phase == Phase::FirstMain
            && quiet
            && matches!(engine.pending(), Pending::Priority { .. })
        {
            return (mark, None);
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. }
                if quiet
                    && player == turn.active
                    && matches!(turn.phase, Phase::FirstMain | Phase::SecondMain)
                    && !acted.contains(&(turn.number, player)) =>
            {
                acted.push((turn.number, player));
                play_and_cast(engine, player, table.card);
            }
            Pending::Priority { player, .. }
                if quiet
                    && player == P1
                    && turn.active == P1
                    && turn.phase == Phase::SecondMain
                    && !killed =>
            {
                killed = true;
                let elves = [table.sides[0][0], table.sides[1][0]];
                if let Some(state) = engine.dev_state_mut(P1) {
                    for elf in elves.into_iter().flatten() {
                        if elf != table.card
                            && Some(elf) != table.host
                            && state
                                .object(elf)
                                .is_some_and(|o| o.zone == Zone::Battlefield)
                        {
                            sba::destroy(state, elf);
                        }
                    }
                }
                engine.refresh_offer();
            }
            Pending::ChooseAttackers {
                player,
                attackers,
                defenders,
                ..
            } if player == P1 => {
                let at = defenders
                    .iter()
                    .copied()
                    .find(|d| matches!(d, Defender::Player(_)))
                    .or_else(|| defenders.first().copied());
                let declared = at
                    .map(|at| attackers.iter().map(|&a| (a, at)).collect())
                    .unwrap_or_default();
                if engine
                    .apply(
                        player,
                        PlayerAction::DeclareAttackers {
                            attackers: declared,
                        },
                    )
                    .is_err()
                {
                    return (mark, Some("seat 1's attack was refused".into()));
                }
            }
            _ => {
                let (player, action) = match answer_one(engine) {
                    Ok(pair) => pair,
                    Err(rest) => return (mark, Some(rest_name(&rest))),
                };
                if let Err(e) = engine.apply(player, action) {
                    return (mark, Some(format!("refused: {e:?}")));
                }
            }
        }
    }
    (mark, Some("the turn cycle never finished".into()))
}

/// `seat` plays a land and casts an Elf from its hand, neither of them the
/// card. Best effort: what the board does not allow is not done.
fn play_and_cast(engine: &mut Engine<RegistryLookup>, seat: PlayerId, card: ObjectId) {
    let elf = quiet_creature();
    let forest = basic_forest();
    if let Pending::Priority { legal, .. } = engine.pending().clone()
        && let Some(&land) = legal.lands.iter().find(|&&l| l != card)
    {
        let _ = engine.apply(seat, PlayerAction::PlayLand { card: land });
    }
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        return;
    };
    let green = legal.mana_abilities.iter().copied().find(|&source| {
        source != card
            && engine
                .state()
                .object(source)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == forest))
    });
    let spell = engine
        .state()
        .zones
        .list(ZoneLocation::Hand(seat))
        .iter()
        .copied()
        .find(|&id| {
            id != card
                && engine
                    .state()
                    .object(id)
                    .is_some_and(|o| o.card.is_some_and(|c| c.index == elf))
        });
    if let (Some(green), Some(spell)) = (green, spell)
        && engine
            .apply(seat, PlayerAction::ActivateManaAbility { source: green })
            .is_ok()
        && matches!(engine.pending(), Pending::Priority { .. })
    {
        let _ = engine.apply(seat, PlayerAction::CastSpell { card: spell });
    }
}

/// What happened in a trigger walk, counted off the journal: the floor the
/// sweep prints and the representative test asserts, because a walk that
/// did nothing would find no trigger and look clean.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Walk {
    lands: u16,
    spells: u16,
    entered: u16,
    died: u16,
    attacks: u16,
    upkeeps: u16,
    draws: u16,
    ends: u16,
}

impl Walk {
    fn read(entries: &[crate::event::JournalEntry]) -> Self {
        use crate::event::GameEvent as E;
        use crate::turn::Step;
        let mut walk = Self::default();
        for entry in entries {
            match entry.event {
                E::LandPlayed { .. } => walk.lands += 1,
                E::SpellCast { .. } => walk.spells += 1,
                E::ZoneChanged {
                    to: Zone::Battlefield,
                    ..
                } => walk.entered += 1,
                E::ZoneChanged {
                    from: Zone::Battlefield,
                    to: Zone::Graveyard,
                    ..
                } => walk.died += 1,
                E::BecameAttacker { .. } => walk.attacks += 1,
                E::StepChanged {
                    step: Step::Upkeep, ..
                } => walk.upkeeps += 1,
                E::CardsDrawn { .. } => walk.draws += 1,
                E::StepChanged {
                    step: Step::End, ..
                } => walk.ends += 1,
                _ => {}
            }
        }
        walk
    }

    /// Everything [`battery`] sets out to do, done at least once per side
    /// where it is per side.
    const fn complete(self) -> bool {
        self.lands >= 2
            && self.spells >= 2
            && self.entered >= 4
            && self.died >= 1
            && self.attacks >= 1
            && self.upkeeps >= 2
            && self.draws >= 2
            && self.ends >= 2
    }
}

/// The card's own triggered abilities that triggered in `entries`.
fn fired(
    entries: &[crate::event::JournalEntry],
    card: ObjectId,
) -> impl Iterator<Item = (u64, u32, PlayerId)> + '_ {
    entries.iter().filter_map(move |entry| match entry.event {
        crate::event::GameEvent::AbilityTriggered {
            source,
            ability_index,
            controller,
            ..
        } if source == card && ability_index < AbilityRef::FIRST_RESERVED => {
            Some((entry.seq, ability_index, controller))
        }
        _ => None,
    })
}

/// The seq at which the card phased back in after `mark`, if it did.
fn phased_in_at(entries: &[crate::event::JournalEntry], card: ObjectId) -> Option<u64> {
    entries.iter().find_map(|entry| match entry.event {
        crate::event::GameEvent::PhaseChanged {
            object,
            phased_out: false,
        } if object == card => Some(entry.seq),
        _ => None,
    })
}

// --- control: which side the statics touch ------------------------------------

/// What the card's statics do to each bystander: its characteristics with
/// them against a refresh of the same state without them. Differences only,
/// and power and toughness as a delta, so a counter one Elf carries and the
/// other does not cancels out.
fn contributions(
    state: &GameState,
    card: ObjectId,
    sides: &[[Option<ObjectId>; 3]; 2],
) -> [[Option<String>; 3]; 2] {
    let mut without = state.clone();
    without
        .effects
        .remove_where(|fx| fx.source == Some(card) && fx.origin == EffectOrigin::Static);
    without.invalidate_projections();
    without.refresh_characteristics();
    sides.map(|side| {
        side.map(|id| {
            let id = id?;
            let with = state.object(id).filter(|o| o.zone == Zone::Battlefield)?;
            let wo = without.object(id)?;
            Some(difference(with.characteristics(), wo.characteristics()))
        })
    })
}

fn difference(with: &Characteristics, without: &Characteristics) -> String {
    let mut out = String::new();
    let (p, t) = (
        with.power.unwrap_or(0) - without.power.unwrap_or(0),
        with.toughness.unwrap_or(0) - without.toughness.unwrap_or(0),
    );
    if (p, t) != (0, 0) {
        let _ = write!(out, "P/T {p:+}/{t:+} ");
    }
    if with.types != without.types {
        let _ = write!(out, "types {:?} ", with.types);
    }
    if with.subtypes != without.subtypes {
        let _ = write!(out, "subtypes {:?} ", with.subtypes);
    }
    if with.supertypes != without.supertypes {
        let _ = write!(out, "supertypes {:?} ", with.supertypes);
    }
    if with.colors != without.colors {
        let _ = write!(out, "colors {:?} ", with.colors);
    }
    if with.keywords != without.keywords {
        let _ = write!(out, "keywords {:?} ", with.keywords);
    }
    if with.abilities_lost != without.abilities_lost {
        out.push_str("abilities lost ");
    }
    out
}

/// After the card changed sides, what its statics did to seat 0's
/// bystanders they now do to seat 1's, and the other way round.
fn swap(before: &[[Option<String>; 3]; 2], after: &[[Option<String>; 3]; 2], found: &mut Findings) {
    const NAMES: [&str; 3] = ["Elf", "first land", "second land"];
    for (slot, name) in NAMES.iter().enumerate() {
        for (from, to) in [(0, 1), (1, 0)] {
            if let (Some(was), Some(now)) = (&before[from][slot], &after[to][slot])
                && was != now
            {
                found.lingers.push(format!(
                    "after the steal seat {to}'s {name} gets `{}` where seat {from}'s got `{}`",
                    now.trim(),
                    was.trim()
                ));
            }
        }
    }
}

// --- one probe -----------------------------------------------------------------

/// One card, one route.
fn probe(def: &'static CardDef, route: Route) -> Line {
    let mut table = match seat(def) {
        Ok(table) => table,
        Err(why) => return skipped(def.index, route, why),
    };
    let card = table.card;
    let stale_before = !table.engine.projection_is_fresh();
    let before = registered(table.engine.state(), card);
    let contributed = (route == Route::Control && table.host.is_none())
        .then(|| contributions(table.engine.state(), card, &table.sides));
    let version = table.engine.state().object(card).map(|o| o.version);
    if let Err(why) = take_away(&mut table, route) {
        return skipped(def.index, route, why);
    }
    if let Some(line) = missed(&table, def, route, version) {
        return line;
    }
    let mut found = Findings::default();
    let when = match route {
        Route::PhaseOut => "while phased out",
        Route::PhaseHost => "while phased out with its host",
        Route::Control => "after the steal",
        _ => "after it left",
    };
    found.fresh(&table.engine, "after settling");
    if stale_before && let Some(last) = found.stale.last_mut() {
        last.push_str(" (already stale before the route)");
    }
    if route == Route::Control {
        stolen(&table, &before, contributed.as_ref(), &mut found);
    } else {
        residue(table.engine.state(), card, route.leaves(), &mut found, when);
    }
    offered(&table.engine, card, &mut found, when);
    if route.phases()
        && let Err(why) = unaffected(&mut table, &mut found)
    {
        return skipped(def.index, route, why);
    }

    let (mark, short) = battery(&mut table);
    found.short = short;
    let entries = &table.engine.state().journal.entries()[mark as usize..];
    found.walk = Walk::read(entries);
    let window_end = if route.phases() {
        phased_in_at(entries, card).unwrap_or(u64::MAX)
    } else {
        u64::MAX
    };
    for (seq, index, by) in fired(entries, card) {
        let lingering = match route {
            Route::Control => by != P1,
            _ => seq < window_end,
        };
        if lingering {
            found.lingers.push(if route == Route::Control {
                format!(
                    "its ability {index} triggered for seat {} after the steal",
                    by.get()
                )
            } else {
                format!("its ability {index} triggered {when}")
            });
            break;
        }
    }
    let state = table.engine.state();
    match route {
        Route::PhaseOut | Route::PhaseHost if found.short.is_none() => {
            let host = (route == Route::PhaseHost)
                .then_some(table.host)
                .flatten()
                .map(|host| (host, phased_in_at(entries, host)));
            phased_back(&table, def, &before, window_end, host, &mut found);
        }
        Route::PhaseOut | Route::PhaseHost | Route::Control => {
            found.fresh(&table.engine, "after the turn cycle");
        }
        _ => {
            residue(state, card, true, &mut found, "a turn cycle later");
            found.fresh(&table.engine, "after the turn cycle");
        }
    }
    found.line(def.index, route)
}

/// Whether the route happened to the card, which had `version` before it:
/// `Some` is the line written instead of probing on, a skip, or
/// `phase_host`'s finding that the card stayed phased in.
fn missed(table: &Table, def: &CardDef, route: Route, version: Option<u32>) -> Option<Line> {
    let Some(obj) = table.engine.state().object(table.card) else {
        return Some(skipped(def.index, route, "the card's object is gone"));
    };
    let phased = obj.status.contains(Status::PHASED_OUT);
    match route {
        _ if route.leaves() && obj.zone == Zone::Battlefield => {
            let why = if Some(obj.version) == version {
                "did not leave the battlefield"
            } else {
                "came back to the battlefield"
            };
            Some(skipped(
                def.index,
                route,
                format!("{why} ({})", route.name()),
            ))
        }
        Route::PhaseOut if !phased => Some(skipped(def.index, route, "did not phase out")),
        Route::PhaseHost if !phased => {
            let host_phased = table.host.is_some_and(|host| {
                table
                    .engine
                    .state()
                    .object(host)
                    .is_some_and(|o| o.status.contains(Status::PHASED_OUT))
            });
            if !host_phased {
                return Some(skipped(def.index, route, "its host did not phase out"));
            }
            let mut found = Findings::default();
            found.lingers.push(
                "stayed phased in when the permanent it is attached to phased out \
                 (CR 702.26g)"
                    .into(),
            );
            Some(found.line(def.index, route))
        }
        Route::Control if obj.controller != P1 => {
            Some(skipped(def.index, route, "control did not change"))
        }
        _ => None,
    }
}

/// `control`, right after the steal: every static and replacement of the
/// card answers to seat 1 (CR 109.5), and what its statics do to the
/// bystanders has changed sides with it.
fn stolen(
    table: &Table,
    before: &Registered,
    contributed: Option<&[[Option<String>; 3]; 2]>,
    found: &mut Findings,
) {
    let (state, card) = (table.engine.state(), table.card);
    for fx in state.effects.iter().filter(|fx| {
        fx.source == Some(card) && fx.origin == EffectOrigin::Static && fx.controller != P1
    }) {
        found.lingers.push(format!(
            "static {} still answers to seat {}",
            variant(&format!("{:?}", fx.modifier)),
            fx.controller.get()
        ));
    }
    for entry in state
        .replacement_rules
        .iter()
        .filter(|r| r.source == card && r.controller != P1)
    {
        found.lingers.push(format!(
            "replacement {} still answers to seat {}",
            variant(&format!("{:?}", entry.rule)),
            entry.controller.get()
        ));
    }
    if let Some(before_side) = contributed
        && registered(state, card) == *before
    {
        let after_side = contributions(state, card, &table.sides);
        swap(before_side, &after_side, found);
    }
}

/// `phase_out` and `phase_host`, after the walk: the card is back and so is
/// what it registered, unless it phased in (at journal seq `phased_in`) and
/// then left by its own rules. For `phase_host`, `host` is the permanent it
/// phased out with and when that phased in: the card phases in with it and
/// not before (CR 702.26g), still attached to it (CR 702.26d).
fn phased_back(
    table: &Table,
    def: &'static CardDef,
    before: &Registered,
    phased_in: u64,
    host: Option<(ObjectId, Option<u64>)>,
    found: &mut Findings,
) {
    let (state, card) = (table.engine.state(), table.card);
    if let Some((host, host_in)) = host
        && phased_in != u64::MAX
    {
        if host_in.is_none_or(|at| phased_in < at) {
            found.lingers.push(
                "phased in by itself, before the permanent it phased out with (CR 702.26g)".into(),
            );
        } else if state.object(card).is_some_and(|o| {
            o.zone == Zone::Battlefield
                && !o.status.contains(Status::PHASED_OUT)
                && o.attached_to != Some(host)
        }) {
            found
                .lingers
                .push("phased in no longer attached to its host (CR 702.26d)".into());
        }
    }
    let phased_in = phased_in != u64::MAX;
    let back = state
        .object(card)
        .is_some_and(|o| o.zone == Zone::Battlefield && !o.status.contains(Status::PHASED_OUT));
    if back {
        let after = registered(state, card);
        let unconditional = unconditional_statics(def);
        for missing in before
            .statics
            .iter()
            .filter(|s| unconditional.contains(*s) && !after.statics.contains(*s))
        {
            found.lingers.push(format!(
                "static {} did not come back after phasing in",
                variant(missing)
            ));
        }
        for missing in before
            .replacements
            .iter()
            .filter(|r| !after.replacements.contains(*r))
        {
            found.lingers.push(format!(
                "replacement {} did not come back after phasing in",
                variant(missing)
            ));
        }
        found.fresh(&table.engine, "after phasing in");
    } else if phased_in
        && state
            .object(card)
            .is_none_or(|o| o.zone != Zone::Battlefield)
    {
        // It phased in and then left by its own rules (a Saga's last
        // chapter, an upkeep's "sacrifice unless"): nothing to see come
        // back, and what it left must be gone.
        residue(state, card, true, found, "after it phased in and left");
        found.fresh(&table.engine, "after it phased in and left");
    } else {
        found.lingers.push(if host.is_some() {
            "did not phase back in with its host".into()
        } else {
            "did not phase back in at seat 0's untap step".into()
        });
    }
}

/// The modifiers of the card's printed statics that hold under no
/// condition, which are the ones that must be back whatever the board.
fn unconditional_statics(def: &CardDef) -> Vec<String> {
    def.abilities_for_face(0)
        .iter()
        .filter_map(|a| match a {
            AbilityDef::Static(sa) if sa.condition.is_none() => Some(format!("{:?}", sa.modifier)),
            _ => None,
        })
        .collect()
}

/// [`probe`], with a panic turned into a skip that says so.
fn probe_caught(def: &'static CardDef, route: Route) -> Line {
    std::panic::catch_unwind(|| probe(def, route)).unwrap_or_else(|panic| {
        let text = panic
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| panic.downcast_ref::<&str>().copied())
            .unwrap_or("?");
        skipped(def.index, route, format!("panic: {text}"))
    })
}

/// Every card the probe is for: implemented, a permanent on its front face.
fn pool() -> Vec<&'static CardDef> {
    baylee_cards::all()
        .filter(|def| def.is_implemented() && is_permanent(def))
        .collect()
}

/// Every (card, route) of `cards`, one chunk per core.
fn sweep(cards: &[&'static CardDef]) -> Vec<Line> {
    let threads = std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get);
    let chunk = cards.len().div_ceil(threads).max(1);
    let mut lines = Vec::new();
    std::thread::scope(|scope| {
        let handles: Vec<_> = cards
            .chunks(chunk)
            .map(|slice| {
                spawn_named(scope, move || {
                    slice
                        .iter()
                        .flat_map(|def| {
                            Route::ALL
                                .into_iter()
                                .filter(|route| route.applies(def))
                                .map(|route| probe_caught(def, route))
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        for handle in handles {
            lines.extend(handle.join().expect("a chunk does not panic"));
        }
    });
    lines.sort_by_key(|line| (line.card.get(), line.route));
    lines
}

/// The directory `BAYLEE_LEAVE_LOG` names; empty is the same as unset.
fn log_dir() -> Option<std::path::PathBuf> {
    std::env::var_os(VAR)
        .filter(|v| !v.is_empty())
        .map(std::path::PathBuf::from)
}

/// Totals, printed by the sweep.
fn totals(lines: &[Line]) -> String {
    let count = |v: Verdict| lines.iter().filter(|l| l.verdict == v).count();
    let cards = {
        let mut ids: Vec<u32> = lines.iter().map(|l| l.card.get()).collect();
        ids.dedup();
        ids.len()
    };
    format!(
        "{cards} cards, {} lines ({} of them `phase_host`): {} ok, {} lingers, {} stale, \
         {} skipped; of the {} probed, {} trigger walks stopped short and {} did less than a \
         full walk",
        lines.len(),
        lines.iter().filter(|l| l.route == Route::PhaseHost).count(),
        count(Verdict::Ok),
        count(Verdict::Lingers),
        count(Verdict::Stale),
        count(Verdict::Skipped),
        lines.len() - count(Verdict::Skipped),
        lines.iter().filter(|l| l.short.is_some()).count(),
        lines
            .iter()
            .filter(|l| l.verdict != Verdict::Skipped && !l.walk.complete())
            .count()
    )
}

/// The pool-wide sweep. Asserts nothing about the pool; with
/// `BAYLEE_LEAVE_LOG` set, writes `leave.jsonl` there.
#[test]
#[ignore = "pool-wide; run with BAYLEE_LEAVE_LOG=<dir> (docs/verification-hooks.md)"]
fn leave_probe_sweep() {
    let cards = pool();
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let lines = sweep(&cards);
    std::panic::set_hook(hook);
    println!("{}", totals(&lines));
    // Which trigger walks were thin, so a clean line with a thin walk can
    // be told from a clean one.
    for line in lines
        .iter()
        .filter(|l| l.short.is_some() || (l.verdict != Verdict::Skipped && !l.walk.complete()))
    {
        println!(
            "walk: card {} {}: {} {:?}",
            line.card.get(),
            line.route.name(),
            line.short.as_deref().unwrap_or("less than a full walk"),
            line.walk
        );
    }
    let Some(dir) = log_dir() else {
        return;
    };
    std::fs::create_dir_all(&dir)
        .unwrap_or_else(|e| panic!("{VAR}={}: cannot create it: {e}", dir.display()));
    let mut out = String::new();
    for line in &lines {
        out.push_str(&line.json());
        out.push('\n');
    }
    let path = dir.join("leave.jsonl");
    std::fs::write(&path, out).unwrap_or_else(|e| panic!("cannot write {}: {e}", path.display()));
}

/// The cards the gate runs the probe on, one of each kind of ability that
/// can outlive its permanent.
///
/// The pool has no static that makes spells cost less: `Primal Amulet` is
/// `Partial` because no `Modifier` reduces a cost, and Training Grounds is a
/// stub. Exploration stands in, a static that changes what its controller
/// may play rather than what an object is.
const REPRESENTATIVE: &[(&str, &str)] = &[
    // "Creatures you control get +1/+1."
    ("Glorious Anthem", "e3886fe8-9b76-4613-8891-4ec74657c087"),
    // Two replacement effects: tokens and counters, doubled.
    ("Doubling Season", "01546b7d-a233-4176-8843-d732074dc5b6"),
    // "Whenever another creature enters, you gain 1 life."
    ("Soul Warden", "f3fad295-1af2-4ecc-8546-b121ad6be27b"),
    // "You may play an additional land on each of your turns."
    ("Exploration", "0c2841bb-038c-4fbf-8360-bc0a1522b58d"),
    // "Other creatures you control get +1/+1": whose they are is the point.
    (
        "Kongming, \"Sleeping Dragon\"",
        "21e9e1a9-5d6d-473e-adab-6a1e8e2b0ebd",
    ),
    // An Aura and an Equipment, which phase out with what they are on.
    ("Holy Strength", "9357de36-f8be-4f49-b2c8-9fe9eaf82b07"),
    ("Bonesplitter", "452e3f5f-ce17-4682-966b-5cc100210aee"),
];

/// Every representative card leaves cleanly by every route, and the board
/// they are probed on is the one the probe describes.
#[test]
fn representative_cards_leave_cleanly_by_every_route() {
    let cards: Vec<&'static CardDef> = REPRESENTATIVE
        .iter()
        .map(|(name, oracle)| {
            let def = baylee_cards::by_index(card_index(oracle)).expect("in the pool");
            assert_eq!(def.name(), *name, "{oracle} is not {name}");
            assert!(
                def.is_implemented() && is_permanent(def),
                "{name} is one the sweep probes"
            );
            def
        })
        .collect();
    let table = seat(cards[0]).expect("the Anthem seats");
    assert_eq!(
        table.engine.state().turn.number,
        1,
        "at this seed seat 0 takes the first turn"
    );
    assert!(
        table.sides.iter().flatten().all(Option::is_some),
        "both sides have an Elf and two lands"
    );
    let lines = sweep(&cards);
    let wrong: Vec<String> = lines
        .iter()
        .filter(|l| l.verdict != Verdict::Ok || l.short.is_some() || !l.walk.complete())
        .map(|l| {
            format!(
                "{} {}: {} {} {} {:?}",
                baylee_cards::by_index(l.card).map_or("?", CardDef::name),
                l.route.name(),
                l.verdict.name(),
                l.detail,
                l.short.as_deref().unwrap_or(""),
                l.walk
            )
        })
        .collect();
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
