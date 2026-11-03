//! The standing orders, one rail at a time, on views of real cards: what
//! "only makes mana" and "could cast by tapping" mean is read off the pool,
//! so the permanents and the hand here are pool cards, not tokens.

use super::*;
use baylee_client_core::test_support::{ViewBuilder, token};
use baylee_core::ids::{DamageSourceRef, Defender, ObjectId, PrintRef};
use baylee_core::mana::{ManaColor, ManaCost};
use baylee_engine::choice::{BlockOption, TargetPrompt};
use baylee_view::{CardIdentity, HandObject, PublicObject, RulesFace};

mod combat;
mod foretell;
mod phasing;

const ME: PlayerId = PlayerId::new(0);
const THEM: PlayerId = PlayerId::new(1);

const fn o(n: u32) -> ObjectId {
    ObjectId::new(n, 0)
}

fn def(name: &str) -> &'static baylee_cards_dsl::CardDef {
    baylee_cards::all()
        .find(|def| def.name() == name)
        .unwrap_or_else(|| panic!("{name} is not in the pool"))
}

fn identity(name: &str) -> CardIdentity {
    CardIdentity {
        index: def(name).index,
        print: PrintRef::new(0),
        face: 0,
    }
}

/// A pool card on the battlefield under this seat, untapped.
fn permanent(slot: u32, name: &str) -> PublicObject {
    let mut object = token(slot, 0, name, 1, 1);
    object.card = Some(identity(name));
    object.rules = object.card.map(RulesFace::from);
    object.types = def(name).faces[0].types;
    object.colors = def(name).faces[0].mana_cost.colors();
    // A basic land's mana is read off its land type (CR 305.6).
    object.subtypes = baylee_core::types::SubtypeSet::from_slice(def(name).faces[0].subtypes);
    object
}

/// A pool card in this seat's hand.
fn in_hand(slot: u32, name: &str) -> HandObject {
    let face = &def(name).faces[0];
    HandObject {
        id: o(slot),
        card: identity(name),
        name: name.to_string(),
        // Not read by the orders, which price the printed cost itself.
        mana_value: 0,
        colors: baylee_core::color::ColorSet::default(),
        types: face.types,
        commander: false,
    }
}

/// A board of `permanents`, each offering its tap for mana as the engine
/// offers it, and a hand of `hand`.
///
/// A basic land's mana is the CR 305.6 shortcut and is offered in
/// `mana_abilities`, with no index; a mana ability a card prints (the
/// first ability of every other card used here) is an ordinary entry in
/// `abilities`.
fn table(permanents: &[&str], hand: &[&str]) -> (PlayerView, LegalActions) {
    let mut view = ViewBuilder::new(2)
        .with_battlefield(
            0,
            permanents
                .iter()
                .zip(1..)
                .map(|(name, slot)| permanent(slot, name)),
        )
        .build();
    view.hand = hand
        .iter()
        .zip(20..)
        .map(|(name, slot)| in_hand(slot, name))
        .collect();
    let basic = |name: &&str| ["Forest", "Island", "Mountain", "Plains", "Swamp"].contains(name);
    let mut legal = LegalActions {
        can_pass: true,
        ..LegalActions::default()
    };
    for (name, slot) in permanents.iter().zip(1..) {
        if basic(name) {
            legal.mana_abilities.push(o(slot));
        } else {
            legal.abilities.push((o(slot), 0));
        }
    }
    (view, legal)
}

fn priority(legal: LegalActions) -> Pending {
    Pending::Priority {
        player: ME,
        legal: Box::new(legal),
    }
}

/// Moves the view to a step of the other seat's turn.
fn their(view: &mut PlayerView, phase: Phase, step: Step) {
    view.active = THEM;
    view.phase = phase;
    view.step = step;
}

fn judge(view: &PlayerView, legal: LegalActions) -> Verdict {
    WakeFilter::default().judge(view, &priority(legal), &[])
}

fn passes(why: Standing) -> Verdict {
    standing(PlayerAction::PassPriority, why)
}

#[test]
fn the_rail_stops_where_the_design_does_and_nowhere_else() {
    let filter = WakeFilter::default();
    for side in RailSide::BOTH {
        for row in RAIL_ROWS.into_iter().filter(|row| row.grants_priority()) {
            assert_eq!(
                filter.stops_at(side, row),
                MIND_STOPS.contains(&(side, row)),
                "{side:?} {row:?}"
            );
        }
    }
}

#[test]
fn lands_and_mana_creatures_with_nothing_to_spend_on_are_nothing_to_do() {
    let (view, legal) = table(&["Forest", "Llanowar Elves"], &[]);
    // The Elves' tap is an activated ability like any other in the engine's
    // list, so by that list this is not "nothing but passing", and the
    // client's rule would stop in this main phase.
    assert!(!legal.nothing_but_passing());
    assert_eq!(judge(&view, legal), passes(Standing::NothingToDo));
}

#[test]
fn a_mana_ability_that_costs_a_sacrifice_is_something_to_do() {
    let (view, legal) = table(&["Lotus Petal"], &[]);
    assert_eq!(judge(&view, legal), Verdict::Wake(Why::RailStop));
}

#[test]
fn lands_that_could_pay_for_a_card_in_hand_are_something_to_do() {
    let (view, legal) = table(&["Forest"], &["Llanowar Elves"]);
    assert!(
        legal.castable.is_empty(),
        "the engine counts only mana floating"
    );
    assert_eq!(judge(&view, legal), Verdict::Wake(Why::RailStop));
    // The same card over the wrong colour is nothing to do.
    let (view, legal) = table(&["Island"], &["Llanowar Elves"]);
    assert_eq!(judge(&view, legal), passes(Standing::NothingToDo));
}

#[test]
fn x_is_counted_at_its_least() {
    // Curse of the Swine is {X}{U}{U}: castable over two Islands with X = 0.
    let (view, legal) = table(&["Island", "Island"], &["Curse of the Swine"]);
    assert_eq!(judge(&view, legal), Verdict::Wake(Why::RailStop));
    let (view, legal) = table(&["Island"], &["Curse of the Swine"]);
    assert_eq!(judge(&view, legal), passes(Standing::NothingToDo));
}

/// Mana Drain's "target spell" has nothing to target on an empty stack
/// (CR 601.2c, 601.2e): no tap makes it castable, so two open Islands are
/// no reach. Found by the gateway e2e, where the menu offered it and the
/// cast was refused after the Islands were tapped.
#[test]
fn a_counterspell_with_nothing_to_counter_is_out_of_reach() {
    let (mut view, legal) = table(&["Island", "Island"], &["Mana Drain"]);
    their(&mut view, Phase::Ending, Step::End);
    assert!(reachable(&view, &legal).is_empty());
    assert_eq!(judge(&view, legal.clone()), passes(Standing::NothingToDo));
    // A spell of theirs on the stack is something to counter.
    view.stack.push(token(90, 1, "TEST spell", 0, 0));
    let reach = reachable(&view, &legal);
    assert_eq!(reach.len(), 1, "{reach:?}");
    assert_eq!(reach[0].plan.steps.len(), 2);
}

/// A card whose cost is already in the pool and that the engine does not
/// list is kept from being cast by something other than mana, which no
/// plan of taps gives it.
#[test]
fn a_card_the_pool_pays_and_the_engine_does_not_list_is_out_of_reach() {
    let (mut view, legal) = table(&["Forest"], &["Llanowar Elves"]);
    assert_eq!(reachable(&view, &legal).len(), 1, "one Forest to tap");
    view.seats[0].mana_pool.green = 1;
    assert!(reachable(&view, &legal).is_empty());
}

#[test]
fn an_instant_over_open_mana_is_woken_at_their_end_step_and_not_at_their_upkeep() {
    let (mut view, legal) = table(&["Forest"], &["Giant Growth"]);
    their(&mut view, Phase::Ending, Step::End);
    assert_eq!(judge(&view, legal.clone()), Verdict::Wake(Why::RailStop));
    their(&mut view, Phase::Beginning, Step::Upkeep);
    assert_eq!(judge(&view, legal), passes(Standing::QuietWindow));
}

#[test]
fn a_creature_in_hand_is_nothing_to_do_on_their_turn() {
    let (mut view, legal) = table(&["Forest"], &["Llanowar Elves"]);
    their(&mut view, Phase::Ending, Step::End);
    assert_eq!(judge(&view, legal), passes(Standing::NothingToDo));
}

#[test]
fn their_stack_wakes_in_any_window_and_a_teammates_does_not() {
    let (mut view, legal) = table(&["Forest"], &["Giant Growth"]);
    their(&mut view, Phase::Beginning, Step::Upkeep);
    view.stack = vec![token(40, 1, "Shock", 0, 0)];
    assert_eq!(
        judge(&view, legal.clone()),
        Verdict::Wake(Why::OpposingStack)
    );
    // At a table where both seats play for one team, the other seat's turn
    // is this side's, its upkeep a row the rail passes, and its spell no
    // opponent's.
    let teams = [Some(1), Some(1)];
    assert_eq!(
        WakeFilter::default().judge(&view, &priority(legal), &teams),
        passes(Standing::QuietWindow)
    );
}

#[test]
fn nothing_to_do_passes_even_their_stack() {
    let (mut view, legal) = table(&["Forest"], &[]);
    their(&mut view, Phase::Beginning, Step::Upkeep);
    view.stack = vec![token(40, 1, "Shock", 0, 0)];
    assert_eq!(judge(&view, legal), passes(Standing::NothingToDo));
}

#[test]
fn a_cleanup_window_wakes() {
    let (mut view, legal) = table(&["Forest"], &["Giant Growth"]);
    view.phase = Phase::Ending;
    view.step = Step::Cleanup;
    assert_eq!(judge(&view, legal), Verdict::Wake(Why::Cleanup));
}

#[test]
fn an_owed_payment_wakes_whatever_else_is_offered() {
    let (mut view, legal) = table(&["Forest"], &[]);
    view.owed = Some(baylee_core::mana::ManaPayment::Fixed(
        "{1}".parse::<ManaCost>().unwrap(),
    ));
    assert_eq!(judge(&view, legal), Verdict::Wake(Why::Owed));
}

#[test]
fn a_declaration_is_made_only_when_there_is_nothing_to_declare() {
    let view = ViewBuilder::new(2).build();
    let mut filter = WakeFilter::default();
    let attack = |attackers| Pending::ChooseAttackers {
        alone: Vec::new(),
        minimum_required: None,
        player: ME,
        attackers,
        defenders: vec![Defender::Player(THEM)],
        required: Vec::new(),
        limits: Vec::new(),
    };
    assert_eq!(
        filter.judge(&view, &attack(Vec::new()), &[]),
        standing(
            PlayerAction::DeclareAttackers {
                attackers: Vec::new()
            },
            Standing::NoAttackers
        )
    );
    assert_eq!(
        filter.judge(&view, &attack(vec![o(5)]), &[]),
        Verdict::Wake(Why::Declaration)
    );
    let block = |blockers| Pending::ChooseBlockers {
        demands: Vec::new(),
        player: ME,
        attacker: THEM,
        blockers,
        bounds: Vec::new(),
        capacity: Vec::new(),
        obeying: Vec::new(),
    };
    assert_eq!(
        filter.judge(&view, &block(Vec::new()), &[]),
        standing(
            PlayerAction::DeclareBlockers {
                blockers: Vec::new()
            },
            Standing::NoBlockers
        )
    );
    let blocker = BlockOption {
        blocker: o(6),
        attackers: vec![o(9)],
    };
    assert_eq!(
        filter.judge(&view, &block(vec![blocker]), &[]),
        Verdict::Wake(Why::Declaration)
    );
}

#[test]
fn every_other_question_goes_to_the_mind_and_another_seats_to_nobody() {
    let view = ViewBuilder::new(2).build();
    let mut filter = WakeFilter::default();
    let targets = |player| Pending::ChooseTargets {
        player,
        options: vec![o(4)],
        player_options: Vec::new(),
        min: 1,
        max: 1,
        reason: TargetPrompt::Targets,
    };
    assert_eq!(
        filter.judge(&view, &targets(ME), &[]),
        Verdict::Wake(Why::Question)
    );
    assert_eq!(
        filter.judge(&view, &targets(THEM), &[]),
        Verdict::Wake(Why::NotAsked)
    );
}

#[test]
fn a_payment_under_way_is_the_minds_next_step_until_it_ends() {
    let (mut view, legal) = table(&["Forest", "Llanowar Elves"], &[]);
    let mut filter = WakeFilter::default();
    filter.heard(&view, &PlayerAction::ActivateManaAbility { source: o(1) });
    view.seats[0].mana_pool.green = 1;
    // Passing now would throw the mana away, and nothing is castable yet by
    // the engine's list: the orders would call this nothing to do.
    assert_eq!(
        filter.judge(&view, &priority(legal.clone()), &[]),
        Verdict::Continue
    );
    // What the next tap asks on its way is part of the payment too.
    let colour = Pending::ChooseColor {
        player: ME,
        options: vec![ManaColor::Green, ManaColor::Red],
    };
    assert_eq!(filter.judge(&view, &colour, &[]), Verdict::Continue);
    // A cast ends it.
    filter.heard(&view, &PlayerAction::CastSpell { card: o(20) });
    assert_eq!(
        filter.judge(&view, &priority(legal), &[]),
        passes(Standing::NothingToDo)
    );
}

#[test]
fn a_payment_does_not_outlive_its_step_or_its_mana() {
    let (mut view, legal) = table(&["Forest"], &[]);
    let mut filter = WakeFilter::default();
    filter.heard(&view, &PlayerAction::ActivateManaAbility { source: o(1) });
    // The pool is empty (the mana was spent by an ability that asked
    // nothing): an ordinary round, and the payment is over.
    assert_eq!(
        filter.judge(&view, &priority(legal.clone()), &[]),
        passes(Standing::NothingToDo)
    );
    view.seats[0].mana_pool.green = 1;
    assert_eq!(
        filter.judge(&view, &priority(legal.clone()), &[]),
        passes(Standing::NothingToDo)
    );
    // Begun in one step, it is not continued in the next.
    filter.heard(&view, &PlayerAction::ActivateManaAbility { source: o(1) });
    view.phase = Phase::Combat;
    view.step = Step::CombatBegin;
    assert_eq!(
        filter.judge(&view, &priority(legal), &[]),
        passes(Standing::NothingToDo)
    );
}

#[test]
fn an_answer_on_the_way_keeps_the_payment_and_one_that_ends_it_does_not() {
    let (mut view, legal) = table(&["Forest"], &[]);
    view.seats[0].mana_pool.green = 1;
    let mut filter = WakeFilter::default();
    filter.heard(&view, &PlayerAction::ActivateManaAbility { source: o(1) });
    filter.heard(&view, &PlayerAction::ChooseColor(ManaColor::Green));
    assert_eq!(
        filter.judge(&view, &priority(legal.clone()), &[]),
        Verdict::Continue
    );
    filter.heard(&view, &PlayerAction::PassPriority);
    assert_eq!(
        filter.judge(&view, &priority(legal), &[]),
        passes(Standing::NothingToDo)
    );
}

/// A spell of theirs on the stack, aimed at `target`.
fn aimed(view: &mut PlayerView, target: TargetRef) {
    let mut shock = token(40, 1, "Shock", 0, 0);
    shock.targets = vec![target];
    view.stack = vec![shock];
}

/// An object as a target.
const fn at(n: u32) -> TargetRef {
    TargetRef::Object(DamageSourceRef {
        object: o(n),
        version: 0,
    })
}

/// A filter holding `until`, with `react`, written in `view`.
fn holding(until: Until, react: React, view: &PlayerView) -> WakeFilter {
    let mut filter = WakeFilter::default();
    let orders = Orders {
        until: Some(until),
        react,
    };
    filter.apply_orders(None, Some(orders), 0, view, &[]);
    filter
}

#[test]
fn react_targets_me_passes_a_spell_aimed_elsewhere_and_wakes_at_one_aimed_at_me() {
    let (mut view, legal) = table(&["Forest"], &["Giant Growth"]);
    their(&mut view, Phase::Beginning, Step::Upkeep);
    let mut filter = holding(Until::MyTurn, React::TargetsMe, &view);
    // At their own creature: not this seat's business.
    aimed(&mut view, at(70));
    assert_eq!(
        filter.judge(&view, &priority(legal.clone()), &[]),
        passes(Standing::QuietWindow)
    );
    // At this seat's Forest: a permanent it controls.
    aimed(&mut view, at(1));
    assert_eq!(
        filter.clone().judge(&view, &priority(legal.clone()), &[]),
        Verdict::Wake(Why::OpposingStack)
    );
    // At this seat's commander, wherever it is and whoever controls it.
    view.seats[0].commanders = vec![baylee_view::CommanderView {
        object: o(80),
        card: None,
        name: "Atraxa".into(),
        casts: 1,
    }];
    aimed(&mut view, at(80));
    assert_eq!(
        filter.clone().judge(&view, &priority(legal.clone()), &[]),
        Verdict::Wake(Why::OpposingStack)
    );
    // At this seat itself; the wake ends the `until`, and says how.
    aimed(&mut view, TargetRef::Player(ME));
    assert_eq!(
        filter.judge(&view, &priority(legal.clone()), &[]),
        Verdict::Wake(Why::OpposingStack)
    );
    assert_eq!(
        filter.take_held(),
        Some(Held {
            until: Until::MyTurn,
            windows: 1,
            woke: Some(Why::OpposingStack),
        })
    );
    assert_eq!(filter.take_held(), None, "told once");
    // Ended: the next spell of theirs wakes, whatever it aims at.
    aimed(&mut view, at(70));
    assert_eq!(
        filter.judge(&view, &priority(legal), &[]),
        Verdict::Wake(Why::OpposingStack)
    );
}

#[test]
fn react_none_passes_their_stack_and_react_all_wakes_at_it() {
    let (mut view, legal) = table(&["Forest"], &["Giant Growth"]);
    their(&mut view, Phase::Beginning, Step::Upkeep);
    let mut quiet = holding(Until::EndOfTurn, React::None, &view);
    let mut all = holding(Until::EndOfTurn, React::All, &view);
    aimed(&mut view, TargetRef::Player(ME));
    assert_eq!(
        quiet.judge(&view, &priority(legal.clone()), &[]),
        passes(Standing::QuietWindow)
    );
    assert_eq!(
        all.judge(&view, &priority(legal), &[]),
        Verdict::Wake(Why::OpposingStack)
    );
}

#[test]
fn an_until_never_answers_a_block_there_is_something_to_declare_in() {
    let (mut view, _) = table(&["Forest"], &[]);
    their(&mut view, Phase::Combat, Step::DeclareBlockers);
    let mut filter = holding(Until::MyTurn, React::None, &view);
    let block = Pending::ChooseBlockers {
        demands: Vec::new(),
        player: ME,
        attacker: THEM,
        blockers: vec![BlockOption {
            blocker: o(6),
            attackers: vec![o(9)],
        }],
        bounds: Vec::new(),
        capacity: Vec::new(),
        obeying: Vec::new(),
    };
    assert_eq!(
        filter.judge(&view, &block, &[]),
        Verdict::Wake(Why::Declaration)
    );
    // Nothing passed, and a wake ended it: the mind hears why.
    assert_eq!(
        filter.take_held().map(|held| held.woke),
        Some(Some(Why::Declaration))
    );
}

#[test]
fn my_main2_ends_at_the_seats_second_main_phase_and_not_before() {
    let (mut view, legal) = table(&["Forest"], &["Giant Growth"]);
    view.turn = 3;
    view.phase = Phase::SecondMain;
    view.step = Step::Main;
    let fresh = |view: &PlayerView| judge(view, legal.clone());
    // Written in the second main phase, it means the next one.
    let mut filter = holding(Until::MyMain2, React::All, &view);
    assert_eq!(fresh(&view), Verdict::Wake(Why::RailStop));
    assert_eq!(
        filter.judge(&view, &priority(legal.clone()), &[]),
        passes(Standing::QuietWindow)
    );
    // Their end step and this seat's first main phase: windows the rail
    // wakes in, which the `until` passes.
    view.turn = 4;
    their(&mut view, Phase::Ending, Step::End);
    assert_eq!(fresh(&view), Verdict::Wake(Why::RailStop));
    assert_eq!(
        filter.judge(&view, &priority(legal.clone()), &[]),
        passes(Standing::QuietWindow)
    );
    view.turn = 5;
    view.active = ME;
    view.phase = Phase::FirstMain;
    view.step = Step::Main;
    assert_eq!(fresh(&view), Verdict::Wake(Why::RailStop));
    assert_eq!(
        filter.judge(&view, &priority(legal.clone()), &[]),
        passes(Standing::QuietWindow)
    );
    assert_eq!(filter.take_held(), None, "still holding");
    view.phase = Phase::SecondMain;
    assert_eq!(
        filter.judge(&view, &priority(legal), &[]),
        Verdict::Wake(Why::RailStop)
    );
    assert_eq!(
        filter.take_held(),
        Some(Held {
            until: Until::MyMain2,
            windows: 3,
            woke: None,
        })
    );
}

#[test]
fn a_plan_is_asked_every_question_of_its_turn_and_none_after() {
    let (mut view, legal) = table(&["Forest"], &[]);
    view.turn = 3;
    let mut filter = WakeFilter::default();
    filter.apply_orders(None, None, 2, &view, &[]);
    // Even what the orders would answer at once is the plan's.
    assert_eq!(
        filter.judge(&view, &priority(legal.clone()), &[]),
        Verdict::Planned
    );
    view.turn = 4;
    their(&mut view, Phase::Beginning, Step::Upkeep);
    assert_eq!(
        filter.judge(&view, &priority(legal.clone()), &[]),
        passes(Standing::NothingToDo)
    );
    // An answer with no steps left ends it within its own turn.
    view.turn = 5;
    filter.apply_orders(None, None, 1, &view, &[]);
    filter.apply_orders(None, None, 0, &view, &[]);
    assert_eq!(
        filter.judge(&view, &priority(legal), &[]),
        passes(Standing::NothingToDo)
    );
}

#[test]
fn unpaid_abilities_payable_from_untapped_lands_are_reachable_and_wake() {
    let (mut view, mut legal) = table(&["Forest", "Forest"], &[]);
    view.battlefield.push(permanent(10, "Grizzly Bears"));
    legal
        .unpaid_abilities
        .push((o(10), 0, "{1}{G}".parse::<ManaCost>().unwrap()));
    let reach = reachable_abilities(&view, &legal);
    assert_eq!(reach.len(), 1);
    assert_eq!(reach[0].plan.steps.len(), 2);
    assert_eq!(judge(&view, legal), Verdict::Wake(Why::RailStop));
}

#[test]
fn wild_growth_and_mana_flare_add_bonus_mana_to_land_taps() {
    let (mut view, legal) = table(&["Forest"], &["Grizzly Bears"]);
    let mut wg = permanent(10, "Wild Growth");
    wg.attached_to = Some(o(1));
    view.battlefield.push(wg);
    let reach = reachable(&view, &legal);
    assert_eq!(
        reach.len(),
        1,
        "one Forest enchanted with Wild Growth pays {{1}}{{G}}"
    );
    assert_eq!(reach[0].plan.steps.len(), 1);
}

#[test]
fn psychic_venom_and_blight_deprioritize_cursed_land() {
    let (mut view, legal) = table(&["Forest", "Forest"], &["Llanowar Elves"]);
    let mut pv = permanent(10, "Psychic Venom");
    pv.controller = THEM;
    pv.attached_to = Some(o(1));
    view.battlefield.push(pv);
    let reach = reachable(&view, &legal);
    assert_eq!(reach.len(), 1);
    assert_eq!(
        reach[0].plan.steps[0].source,
        o(2),
        "untapped uncursed Forest #2 must be tapped before Psychic Venom-enchanted Forest #1"
    );
}

#[test]
fn sirens_call_is_only_reachable_on_opponents_turn_before_attackers() {
    let (mut view, legal) = table(&["Island"], &["Siren's Call"]);
    assert!(
        reachable(&view, &legal).is_empty(),
        "Siren's Call cannot be cast on your own turn"
    );
    their(&mut view, Phase::Beginning, Step::Upkeep);
    assert_eq!(
        reachable(&view, &legal).len(),
        1,
        "Siren's Call is reachable on opponent's upkeep"
    );
    their(&mut view, Phase::SecondMain, Step::Main);
    assert!(
        reachable(&view, &legal).is_empty(),
        "Siren's Call is not reachable after declare attackers"
    );
}

#[test]
fn combat_begin_stop_accepts_beginning_combat_aliases() {
    let (view, _) = table(&[], &[]);
    let mut filter = WakeFilter::default();
    let stops = crate::narrator::Stops {
        mine: vec!["beginning_combat".into()],
        theirs: vec!["begin_combat".into()],
    };
    filter.apply_orders(Some(&stops), None, 0, &view, &[]);
    assert!(filter.stops_at(RailSide::Mine, RailRow::CombatBegin));
    assert!(filter.stops_at(RailSide::Theirs, RailRow::CombatBegin));
}

#[test]
fn convoke_reduces_generic_and_matching_colored_pips_without_double_counting_mana_dorks() {
    // Chord of Calling is {X}{G}{G}{G} with Convoke (CR 702.51a).
    // Two Forests + one white creature (Eager Cadet) cannot pay the third {G}.
    let (mut view, legal) = table(&["Forest", "Forest"], &["Chord of Calling"]);
    view.battlefield.push(permanent(10, "Eager Cadet"));
    assert!(
        reachable(&view, &legal).is_empty(),
        "a white creature cannot convoke a green pip on Chord of Calling"
    );

    // Replacing Eager Cadet with a green creature (Grizzly Bears) convokes the third {G}.
    view.battlefield.pop();
    view.battlefield.push(permanent(10, "Grizzly Bears"));
    let reach = reachable(&view, &legal);
    assert_eq!(
        reach.len(),
        1,
        "two Forests + one green creature pay {{G}}{{G}}{{G}} via Convoke"
    );
    assert_eq!(reach[0].plan.steps.len(), 2);

    // Clever Concealment is {2}{W}{W} with Convoke.
    // Plains + active Llanowar Elves + one white creature make only three
    // mana/convoke payments: the Elves cannot both make mana and convoke.
    let (mut view, legal) = table(&["Plains", "Llanowar Elves"], &["Clever Concealment"]);
    view.battlefield.push(permanent(10, "Eager Cadet"));
    assert!(
        reachable(&view, &legal).is_empty(),
        "an active mana dork must not be double-counted as both a mana source and a convoke tap"
    );
    // The second white creature supplies the fourth payment.
    view.battlefield.push(permanent(11, "Eager Cadet"));
    assert_eq!(reachable(&view, &legal).len(), 1);
}

#[test]
fn self_tapping_unpaid_ability_cannot_tap_its_own_source_for_mana() {
    // Eden, Seat of the Sanctum has 0: {T}: Add {C} and 1: {5}, {T}: Mill two cards...
    // With 4 Forests + Eden itself untapped (5 mana sources total, including Eden),
    // Eden's ability 1 ({5}, {T}) is NOT reachable because Eden cannot tap itself for {C}
    // and also tap itself to pay {T} of ability 1.
    let (mut view, mut legal) = table(
        &[
            "Forest",
            "Forest",
            "Forest",
            "Forest",
            "Eden, Seat of the Sanctum",
        ],
        &[],
    );
    legal
        .unpaid_abilities
        .push((o(5), 1, "{5}".parse::<ManaCost>().unwrap()));
    assert!(
        reachable_abilities(&view, &legal).is_empty(),
        "Eden cannot tap itself for mana to pay for its own {{5}}, {{T}} ability"
    );
    // Adding a fifth Forest makes Eden's ability 1 reachable without tapping Eden for mana.
    view.battlefield.push(permanent(6, "Forest"));
    legal.mana_abilities.push(o(6));
    let reach = reachable_abilities(&view, &legal);
    assert_eq!(reach.len(), 1);
    assert!(
        reach[0].plan.steps.iter().all(|s| s.source != o(5)),
        "the mana plan for Eden's {{T}} ability must not tap Eden"
    );
}

#[test]
fn mdfc_spell_pitch_spell_and_graveyard_exile_permissions_are_reachable() {
    // 1. MDFC spell front face (Shatterskull Smashing {X}{R}{R}) remains reachable as a spell
    // even when its back land face puts its ObjectId in legal.lands.
    let (view, mut legal) = table(&["Mountain", "Mountain"], &["Shatterskull Smashing"]);
    legal.lands.push(o(20));
    let reach = reachable(&view, &legal);
    assert_eq!(reach.len(), 1);
    assert_eq!(reach[0].object, o(20));
    assert_eq!(reach[0].from, ReachFrom::Hand);

    // 2. Pitch spell (Force of Will {3}{U}{U}) in legal.castable (because it can be pitch-cast)
    // is still offered in reachable when lands can pay its mana cost and the pool does not yet hold {3}{U}{U}.
    let (mut view, mut legal) = table(
        &["Island", "Island", "Island", "Island", "Island"],
        &["Force of Will"],
    );
    view.stack.push(token(90, 1, "Shock", 0, 0));
    legal.castable.push(o(20));
    let reach = reachable(&view, &legal);
    assert_eq!(
        reach.len(),
        1,
        "pitch spell in legal.castable should still have a tap plan for its normal mana cost"
    );
    // Once {3}{U}{U} is already floating in the pool, no tap plan is needed.
    view.seats[0].mana_pool.blue = 5;
    assert!(reachable(&view, &legal).is_empty());

    // 3. Graveyard flashback and Exile payable permissions.
    let (mut view, mut legal) = table(&["Forest", "Forest"], &[]);
    let mut gy_card = permanent(70, "Grizzly Bears");
    gy_card.flashback = Some("{1}{G}".parse::<ManaCost>().unwrap());
    view.graveyards[0].push(gy_card);
    let ex_card = permanent(71, "Grizzly Bears");
    view.exile[0].push(ex_card);
    legal.payable.push(o(71));
    let reach = reachable(&view, &legal);
    assert_eq!(reach.len(), 2);
    assert!(
        reach
            .iter()
            .any(|r| r.object == o(70) && r.from == ReachFrom::Graveyard)
    );
    assert!(
        reach
            .iter()
            .any(|r| r.object == o(71) && r.from == ReachFrom::Exile)
    );
}
