//! Forked simulations (`Engine::fork_for`): a fork continues the game it
//! was taken from, never reaches back into it, and holds nothing of the
//! real hidden cards or the real random stream.
//!
//! The fixtures are reached through ordinary `Engine::apply`, never built:
//! a position assembled by hand would be a position no driver ever stood in,
//! and the point is that the fork carries the driver's continuation.

use super::testkit::*;
use super::*;
use crate::engine::{DeckKnowledge, ForkRefusal};
use crate::text_changes::TextOrigin;
use baylee_core::ids::{CardIndex, ObjectId, PrintRef};
use baylee_core::preset::{
    AIProfile, DeckEntry, Finish, FormatId, GamePreset, HouseRules, PrintInfo, SeatController,
    SeatSpec,
};

const ME: PlayerId = PlayerId::new(0);
const THEM: PlayerId = PlayerId::new(1);

fn named(name: &str) -> CardIndex {
    baylee_cards::decks::by_name(name).expect("the registry holds the card")
}

fn forest() -> CardIndex {
    named("Forest")
}

fn elves() -> CardIndex {
    quiet_creature()
}

fn entry(card: CardIndex) -> DeckEntry {
    DeckEntry {
        rule_print: None,
        choices: baylee_core::card_metadata::DeckChoices::default(),
        card,
        print: PrintRef::new(0),
    }
}

/// Forty cards cycling through `cards`, in that order.
fn deck(cards: &[CardIndex]) -> Vec<DeckEntry> {
    (0..40).map(|i| entry(cards[i % cards.len()])).collect()
}

/// My deck: Forests, Islands and Elves.
fn my_deck() -> Vec<DeckEntry> {
    deck(&[forest(), named("Island"), elves(), forest()])
}

/// Their deck: Mountains, Swamps, Elves and Sol Rings, unless a test asks
/// for another.
fn their_deck() -> Vec<DeckEntry> {
    deck(&[named("Mountain"), named("Swamp"), elves(), quiet_artifact()])
}

/// A duel I start, holding two Forests and an Elves; by default they draw
/// their seven from their own shuffled library and nobody has a battlefield
/// or a sideboard. `seed` shuffles both libraries, so two seeds are two
/// games I cannot tell apart until a hidden card shows.
struct Table {
    seed: u64,
    theirs: Vec<DeckEntry>,
    their_hand: Option<Vec<DeckEntry>>,
    their_field: Vec<DeckEntry>,
    their_side: Vec<DeckEntry>,
    my_field: Vec<DeckEntry>,
    my_side: Vec<DeckEntry>,
}

impl Table {
    fn new(seed: u64, theirs: Vec<DeckEntry>) -> Self {
        Self {
            seed,
            theirs,
            their_hand: None,
            their_field: vec![],
            their_side: vec![],
            my_field: vec![],
            my_side: vec![],
        }
    }

    fn preset(&self) -> GamePreset {
        let seat = |deck: Vec<DeckEntry>,
                    hand: Option<Vec<DeckEntry>>,
                    field: Vec<DeckEntry>,
                    side: Vec<DeckEntry>| SeatSpec {
            controller: SeatController::Ai(AIProfile::default()),
            capabilities: baylee_core::preset::SeatCapabilities {
                dev_commands: true,
                see_hidden: false,
            },
            deck,
            sideboard: side,
            commanders: vec![],
            starting_life: None,
            starting_hand: hand,
            starting_battlefield: field,
            emblems: vec![],
            team: None,
        };
        GamePreset {
            rules: baylee_core::card_metadata::RulesMetadata::default(),
            format: FormatId::Freeform,
            seed: self.seed,
            house_rules: HouseRules {
                starting_player: Some(0),
                ..HouseRules::default()
            },
            modifiers: vec![],
            prints: vec![PrintInfo {
                scryfall_id: uuid::Uuid::nil(),
                lang: "EN".into(),
                finish: Finish::Normal,
            }],
            seats: vec![
                seat(
                    my_deck(),
                    Some(vec![entry(forest()), entry(forest()), entry(elves())]),
                    self.my_field.clone(),
                    self.my_side.clone(),
                ),
                seat(
                    self.theirs.clone(),
                    self.their_hand.clone(),
                    self.their_field.clone(),
                    self.their_side.clone(),
                ),
            ],
        }
    }

    /// My first main phase, a Forest played: settled priority, empty stack.
    fn settled_main(&self) -> Engine<RegistryLookup> {
        let mut engine = Engine::new(&self.preset(), RegistryLookup).expect("the duel starts");
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, ME);
        let Pending::Priority { legal, .. } = engine.pending().clone() else {
            panic!("priority in my main phase, got {:?}", engine.pending());
        };
        let land = *legal.lands.first().expect("a Forest to play");
        engine
            .apply(ME, PlayerAction::PlayLand { card: land })
            .expect("the land is played");
        assert!(stack_is_empty(&engine));
        engine
    }
}

fn preset(seed: u64, theirs: Vec<DeckEntry>) -> GamePreset {
    Table::new(seed, theirs).preset()
}

/// My first main phase, a Forest played: settled priority, empty stack.
fn settled_main(seed: u64, theirs: Vec<DeckEntry>) -> Engine<RegistryLookup> {
    Table::new(seed, theirs).settled_main()
}

/// The Elves cast off the Forest and passed over: they hold priority with
/// one pass already made.
fn elves_passed_over(engine: &mut Engine<RegistryLookup>) {
    cast_from_hand(engine, ME, elves());
    assert!(!stack_is_empty(engine), "the Elves are on the stack");
    engine.apply(ME, PlayerAction::PassPriority).unwrap();
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == THEM),
        "they hold priority over the Elves: {:?}",
        engine.pending()
    );
    assert_eq!(engine.passes, 1, "one pass made");
}

/// The hidden zones of a game as I would deal them: each card as its id and
/// its rules identity, zone by zone.
fn hidden_zones(engine: &Engine<RegistryLookup>) -> Vec<Vec<(ObjectId, CardIndex)>> {
    [
        ZoneLocation::Hand(THEM),
        ZoneLocation::Library(THEM),
        ZoneLocation::Library(ME),
    ]
    .into_iter()
    .map(|loc| {
        engine
            .state()
            .zones
            .list(loc)
            .iter()
            .map(|&id| {
                let card = engine
                    .state()
                    .object(id)
                    .and_then(|o| o.card)
                    .expect("a hidden zone holds cards");
                (id, card.index)
            })
            .collect()
    })
    .collect()
}

fn identities(zones: &[Vec<(ObjectId, CardIndex)>]) -> Vec<Vec<CardIndex>> {
    zones
        .iter()
        .map(|zone| zone.iter().map(|&(_, card)| card).collect())
        .collect()
}

/// A multiset, as a sorted list.
fn sorted(mut cards: Vec<CardIndex>) -> Vec<CardIndex> {
    cards.sort_unstable();
    cards
}

/// What everybody at the table sees: the turn, the life totals, the public
/// zones in order, the sizes of the hidden ones, and who is being asked.
fn public_facts(engine: &Engine<RegistryLookup>) -> String {
    let s = engine.state();
    let sizes: Vec<(usize, usize)> = s
        .players
        .iter()
        .map(|p| {
            (
                s.zones.list(ZoneLocation::Hand(p.id)).len(),
                s.zones.list(ZoneLocation::Library(p.id)).len(),
            )
        })
        .collect();
    let lives: Vec<i32> = s.players.iter().map(|p| p.life).collect();
    format!(
        "{:?}",
        (
            (s.turn.number, s.turn.active, s.turn.phase, s.turn.step),
            lives,
            s.zones.list(ZoneLocation::Battlefield),
            s.zones.list(ZoneLocation::Stack),
            s.zones.list(ZoneLocation::Graveyard(ME)),
            s.zones.list(ZoneLocation::Graveyard(THEM)),
            sizes,
            engine.decision_actor(),
            engine.passes,
            engine.priority_holder,
        )
    )
}

/// Passes, and declares no attackers or blockers, for whoever is asked;
/// any other question (a discard to hand size) gets the kit's first legal
/// answer, which depends on nothing but the question. Whoever answers is
/// the decision's actor: the controller, while a player's turn is
/// controlled (CR 722.1).
fn step(engine: &mut Engine<RegistryLookup>) {
    let actor = engine.decision_actor().expect("somebody is asked");
    let action = match engine.pending() {
        Pending::Priority { .. } => PlayerAction::PassPriority,
        Pending::ChooseAttackers { .. } => PlayerAction::DeclareAttackers { attackers: vec![] },
        Pending::ChooseBlockers { .. } => PlayerAction::DeclareBlockers { blockers: vec![] },
        _ => {
            answer_one(engine)
                .unwrap_or_else(|rest| panic!("unanswerable: {rest:?}"))
                .1
        }
    };
    engine.apply(actor, action).expect("the answer is legal");
}

/// Two engines nobody can tell apart: every field of both, the complete
/// comparison (`Engine::fingerprint`), with the first difference of each
/// differing field shown.
#[track_caller]
fn assert_indistinguishable(a: &Engine<RegistryLookup>, b: &Engine<RegistryLookup>) {
    let (fa, fb) = (a.fingerprint(), b.fingerprint());
    let differing = fa.differing(&fb);
    let excerpts: Vec<String> = differing
        .iter()
        .map(|&field| {
            let print = |f: &crate::engine::Fingerprint| {
                f.0.iter()
                    .find(|(name, _)| *name == field)
                    .map(|(_, value)| value.clone())
                    .unwrap_or_default()
            };
            let (x, y) = (print(&fa), print(&fb));
            let at = x
                .char_indices()
                .zip(y.chars())
                .find(|((_, p), q)| p != q)
                .map_or(0, |((i, _), _)| i);
            let excerpt = |s: &str| {
                s.get(at.saturating_sub(120)..(at + 240).min(s.len()))
                    .unwrap_or(s)
                    .to_owned()
            };
            format!("{field}:\n  {}\n  {}", excerpt(&x), excerpt(&y))
        })
        .collect();
    assert!(
        differing.is_empty(),
        "the engines differ in {differing:?}\n{}",
        excerpts.join("\n")
    );
}

/// The source's own record of itself: both hashes and the random stream's
/// seed, position and call count.
fn record(engine: &Engine<RegistryLookup>) -> (u64, u64, [u8; 32], u128, u64) {
    let rng = &engine.state().rng;
    (
        engine.snapshot_hash(),
        engine.state().snapshot_hash(),
        rng.seed(),
        rng.word_pos(),
        rng.calls(),
    )
}

fn next_draws(engine: &Engine<RegistryLookup>) -> Vec<u64> {
    let mut rng = engine.state().rng.clone();
    (0..8).map(|_| rng.next_u64()).collect()
}

fn fork(engine: &Engine<RegistryLookup>, policy_seed: u64) -> Engine<RegistryLookup> {
    engine
        .fork_for(ME, DeckKnowledge::Known, policy_seed)
        .unwrap_or_else(|refusal| panic!("a settled priority forks: {refusal}"))
}

/// Forking reads the source and nothing more, and the fork owns everything
/// it changes: after a fork has been taken, played on past two draw steps,
/// and made to shuffle, the source's hashes, its stream's seed, position
/// and call count, and the next numbers its stream gives are what they were
/// — and what an untouched twin's are.
#[test]
fn forking_leaves_the_source_unchanged() {
    let control = settled_main(31, their_deck());
    let source = settled_main(31, their_deck());
    let before = record(&source);
    assert_eq!(before, record(&control), "the twins start equal");

    let mut fork = fork(&source, 5);
    for _ in 0..40 {
        if fork.state().turn.number >= 3 {
            break;
        }
        step(&mut fork);
    }
    assert!(fork.state().turn.number >= 3, "the fork played on");
    fork.state.shuffle_library(THEM);
    fork.state.shuffle_library(ME);

    assert_eq!(record(&source), before, "the source is untouched");
    assert_eq!(record(&source), record(&control));
    assert_eq!(next_draws(&source), next_draws(&control));
    assert_indistinguishable(&source, &control);
    assert_eq!(source.journal().entries(), control.journal().entries());
}

/// A table with more to deal: their deck has a Plague Rats, which defines
/// its own power and toughness everywhere (CR 604.3), and an Anger, whose
/// static works from a graveyard, and both of us have sideboards.
fn rich(seed: u64) -> Table {
    let rats = named("Plague Rats");
    let anger = named("Anger");
    let mut table = Table::new(
        seed,
        deck(&[named("Mountain"), named("Swamp"), elves(), rats, anger]),
    );
    table.their_side = vec![entry(named("Lightning Bolt")), entry(rats), entry(anger)];
    table.my_side = vec![entry(named("Unsummon"))];
    table
}

/// Their first main phase: they have untapped, drawn a card I did not see,
/// and hold priority. Everything they did is passing, so the two games of
/// two seeds still look the same to me.
fn their_main(seed: u64) -> Engine<RegistryLookup> {
    let mut engine = rich(seed).settled_main();
    pass_until(&mut engine, |e| {
        e.state().turn.active == THEM
            && e.state().turn.phase == Phase::FirstMain
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == THEM)
    });
    engine
}

/// Two games I cannot tell apart — different seeds, so their libraries and
/// their opening hand differ, and so does my own library's order — fork
/// into the same game: every field of the two forks is equal (the complete
/// comparison, journal and offers included), and stays equal as both are
/// played on through draws. Checked where I have just played a land and
/// again after they have drawn on their own turn, whose draw leaves notes
/// of the card drawn in several places.
#[test]
fn hidden_arrangement_does_not_reach_the_fork() {
    let pairs = vec![
        (settled_main(1, their_deck()), settled_main(2, their_deck())),
        (rich(1).settled_main(), rich(2).settled_main()),
        (their_main(1), their_main(2)),
    ];
    for (one, two) in &pairs {
        assert_eq!(public_facts(one), public_facts(two), "I see the same game");
        let (real_one, real_two) = (hidden_zones(one), hidden_zones(two));
        assert_ne!(
            identities(&real_one)[0],
            identities(&real_two)[0],
            "their hands differ"
        );
        assert_ne!(
            identities(&real_one)[2],
            identities(&real_two)[2],
            "my library's order differs"
        );

        let (mut a, mut b) = (fork(one, 7), fork(two, 7));
        assert_eq!(hidden_zones(&a), hidden_zones(&b), "the same cards dealt");
        assert_eq!(next_draws(&a), next_draws(&b));
        let start = a.state().turn.number;
        for _ in 0..60 {
            assert_indistinguishable(&a, &b);
            assert_eq!(a.snapshot_hash(), b.snapshot_hash());
            // The fingerprint prints the journal's length and last entry
            // only: the whole of it is compared here.
            assert_eq!(a.journal().entries(), b.journal().entries());
            if a.state().turn.number >= start + 2 {
                break;
            }
            step(&mut a);
            step(&mut b);
        }
        assert!(
            a.state().turn.number >= start + 2,
            "both forks drew and played on"
        );
    }
}

/// A hidden card's id says nothing about which card it is. Ids are handed
/// out in deck-list order, so a real hidden card's id is the card to anyone
/// holding the list: none of them survives into the fork, nothing in the
/// fork's journal names one, and the id each dealt card gets is fixed by
/// what I can see and the policy seed alone.
#[test]
fn hidden_ids_carry_no_identity() {
    let one = settled_main(1, their_deck());
    let two = settled_main(2, their_deck());
    let (a, b) = (fork(&one, 3), fork(&two, 3));
    let real: Vec<ObjectId> = hidden_zones(&one)
        .into_iter()
        .flatten()
        .map(|(id, _)| id)
        .collect();
    for &id in &real {
        assert!(a.state().object(id).is_none(), "{id:?} is retired");
    }
    let dealt: Vec<ObjectId> = hidden_zones(&a)
        .into_iter()
        .flatten()
        .map(|(id, _)| id)
        .collect();
    assert!(dealt.iter().all(|id| !real.contains(id)), "all ids are new");
    assert_eq!(hidden_zones(&a), hidden_zones(&b), "id → card is the same");
    assert_names_none(&a, &retired_ids(&one));
    assert_names_none(&b, &retired_ids(&two));
    assert!(
        a.journal().entries().iter().all(|entry| !matches!(
            entry.event,
            GameEvent::GameStarted { seed, .. } if seed != 0
        )),
        "the seed is gone"
    );
}

/// What the hidden cards are made of is the deck, and the fork deals what
/// that deck hides: two games apart only in their deck fork apart, each
/// into its own cards, when I know the decks — and when I do not, they are
/// not forked at all.
#[test]
fn a_known_deck_is_dealt_and_an_unknown_one_refused() {
    let swamps = settled_main(9, their_deck());
    let islands = settled_main(
        9,
        deck(&[
            named("Mountain"),
            named("Island"),
            elves(),
            quiet_artifact(),
        ]),
    );
    assert_eq!(public_facts(&swamps), public_facts(&islands));
    let theirs = |engine: &Engine<RegistryLookup>| -> Vec<CardIndex> {
        let zones = identities(&hidden_zones(engine));
        sorted(zones[0].iter().chain(&zones[1]).copied().collect())
    };
    let (a, b) = (fork(&swamps, 4), fork(&islands, 4));
    assert_eq!(theirs(&a), theirs(&swamps), "their own cards, dealt");
    assert_eq!(theirs(&b), theirs(&islands));
    assert!(theirs(&a).contains(&named("Swamp")) && !theirs(&a).contains(&named("Island")));
    assert!(theirs(&b).contains(&named("Island")) && !theirs(&b).contains(&named("Swamp")));

    for engine in [&swamps, &islands] {
        assert_eq!(
            engine.fork_for(ME, DeckKnowledge::Unknown, 4).err(),
            Some(ForkRefusal::UnknownDecks)
        );
    }
}

/// Two policy seeds deal two arrangements, each of them a whole one: every
/// zone keeps its size, every pool its cards, and the game goes on from
/// either.
#[test]
fn policy_seeds_deal_different_arrangements() {
    let source = settled_main(17, their_deck());
    let real = identities(&hidden_zones(&source));
    let (mut a, mut b) = (fork(&source, 1), fork(&source, 2));
    let (dealt_a, dealt_b) = (identities(&hidden_zones(&a)), identities(&hidden_zones(&b)));
    assert_ne!(dealt_a, dealt_b, "two seeds, two deals");
    for dealt in [&dealt_a, &dealt_b] {
        let sizes = |zones: &[Vec<CardIndex>]| zones.iter().map(Vec::len).collect::<Vec<_>>();
        assert_eq!(sizes(dealt), sizes(&real), "every zone keeps its size");
        assert_eq!(
            sorted(dealt[0].iter().chain(&dealt[1]).copied().collect()),
            sorted(real[0].iter().chain(&real[1]).copied().collect()),
            "their hand and library hold the same cards between them"
        );
        assert_eq!(sorted(dealt[2].clone()), sorted(real[2].clone()));
    }
    assert_eq!(
        fork(&source, 1).snapshot_hash(),
        a.snapshot_hash(),
        "one seed, one deal"
    );
    for engine in [&mut a, &mut b] {
        for _ in 0..40 {
            if engine.state().turn.number >= 3 {
                break;
            }
            step(engine);
        }
        assert!(engine.state().turn.number >= 3, "the game goes on");
    }
}

/// A fork stands where its source stands and moves as it moves. At a
/// settled main phase it offers me what the game offers me, and the same
/// actions — tap, cast, pass, resolve — take both to the same board.
#[test]
fn the_fork_continues_the_game() {
    let mut source = settled_main(23, their_deck());
    let mut fork = fork(&source, 11);
    assert_eq!(public_facts(&fork), public_facts(&source));
    assert_eq!(
        format!("{:?}", fork.pending()),
        format!("{:?}", source.pending()),
        "the same offer: my hand and the board are mine to see"
    );
    assert!(fork.mulligans.is_none(), "no mulligan reopened");

    elves_passed_over(&mut source);
    elves_passed_over(&mut fork);
    assert_eq!(public_facts(&fork), public_facts(&source));
    for engine in [&mut source, &mut fork] {
        engine.apply(THEM, PlayerAction::PassPriority).unwrap();
    }
    assert!(stack_is_empty(&source) && stack_is_empty(&fork), "resolved");
    assert_eq!(public_facts(&fork), public_facts(&source));
    assert_eq!(
        format!("{:?}", fork.pending()),
        format!("{:?}", source.pending())
    );
}

/// A fork taken while they hold priority over my Elves, one pass made,
/// carries that pass: their pass resolves the Elves in the fork as in the
/// game. A fork that began a fresh round would hand me priority back with
/// the Elves still on the stack.
#[test]
fn a_fork_keeps_the_passes_already_made() {
    let mut source = settled_main(29, their_deck());
    elves_passed_over(&mut source);
    let mut fork = fork(&source, 13);
    assert_eq!(fork.passes, 1);
    assert_eq!(public_facts(&fork), public_facts(&source));
    // Forked for them instead, the seat holding priority: their hand is
    // theirs to see, so their offer is the game's, while my hand and both
    // libraries are dealt.
    let theirs = source
        .fork_for(THEM, DeckKnowledge::Known, 13)
        .expect("their priority forks too");
    assert_eq!(
        format!("{:?}", theirs.pending()),
        format!("{:?}", source.pending())
    );
    assert_eq!(theirs.passes, 1);
    assert_eq!(
        theirs.state().zones.list(ZoneLocation::Hand(THEM)),
        source.state().zones.list(ZoneLocation::Hand(THEM)),
        "their own hand is kept"
    );
    assert_ne!(
        theirs.state().zones.list(ZoneLocation::Hand(ME)),
        source.state().zones.list(ZoneLocation::Hand(ME)),
        "mine is dealt"
    );
    for engine in [&mut source, &mut fork] {
        engine.apply(THEM, PlayerAction::PassPriority).unwrap();
        assert!(stack_is_empty(engine), "their pass resolves the Elves");
        assert!(
            matches!(engine.pending(), Pending::Priority { player, .. } if *player == ME),
            "and I have priority on the resolved board"
        );
    }
    assert_eq!(public_facts(&fork), public_facts(&source));
}

/// Rhystic Study's tax asked of them, then paid into an open window: both
/// are operations half done, and both are refused rather than forked with
/// their continuation reset.
#[test]
fn a_suspended_operation_is_refused() {
    let study = card_index("53236dd7-845a-444c-96d5-f41ed7325d8f");
    let mut engine = Duel::new(7, forest())
        .battlefield(0, &[study])
        .battlefield(1, &[forest(), forest()])
        .hand(1, &[elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, THEM);
    let forests: Vec<ObjectId> = engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|&id| {
            engine.state().object(id).is_some_and(|o| {
                o.controller == THEM && o.card.is_some_and(|c| c.index == forest())
            })
        })
        .collect();
    engine
        .apply(
            THEM,
            PlayerAction::ActivateManaAbility { source: forests[0] },
        )
        .unwrap();
    cast_with_floating(&mut engine, THEM, elves());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    assert_eq!(
        engine.fork_for(ME, DeckKnowledge::Known, 1).err(),
        Some(ForkRefusal::Continuation("resolution")),
        "the tax question is asked mid-resolution"
    );
    engine.apply(THEM, PlayerAction::YesNo(true)).unwrap();
    assert!(engine.payment_window().is_some(), "the window is open");
    assert!(matches!(engine.pending(), Pending::Priority { .. }));
    assert!(matches!(
        engine.fork_for(ME, DeckKnowledge::Known, 1).err(),
        Some(ForkRefusal::Continuation(_))
    ));
}

/// A face-down permanent is a card nobody but its controller may look at
/// (CR 708.5), and a card exiled face down has its own viewers (CR 406.3):
/// the first version deals neither, so it refuses both.
#[test]
fn face_down_and_hidden_exile_are_refused() {
    let mut engine = settled_main(37, their_deck());
    let land = engine.state().zones.list(ZoneLocation::Battlefield)[0];
    engine
        .dev_state_mut(ME)
        .expect("the fixture trusts its seats")
        .object_mut(land)
        .unwrap()
        .status
        .insert(crate::object::Status::FACE_DOWN);
    assert_eq!(
        engine.fork_for(ME, DeckKnowledge::Known, 1).err(),
        Some(ForkRefusal::FaceDown)
    );

    let mut engine = settled_main(37, their_deck());
    let land = engine.state().zones.list(ZoneLocation::Battlefield)[0];
    let state = engine
        .dev_state_mut(ME)
        .expect("the fixture trusts its seats");
    let card = *state
        .zones
        .list(ZoneLocation::Library(THEM))
        .last()
        .unwrap();
    state
        .move_object(
            card,
            ZoneLocation::Exile(THEM),
            ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    let source = state.source_identity(land).unwrap();
    state.hide_exiled_card(card, source, 0, baylee_core::ids::SeatSet::new(), false);
    // The developer door journals its move and the machine has not run
    // since; read it, as the next pass would, so the board is what is
    // refused and not the unread event.
    engine.trigger_scan_seq = engine.state.journal.last_seq();
    assert_eq!(
        engine.fork_for(ME, DeckKnowledge::Known, 1).err(),
        Some(ForkRefusal::HiddenExile)
    );
}

/// The opening hands are decided on streams of the real seed: no fork. Kept,
/// the first upkeep still has their opening draw's miracle check queued,
/// naming the card they drew first (CR 702.94): no fork either, until the
/// engine has offered it.
#[test]
fn the_opening_and_its_miracle_check_are_refused() {
    let mut engine = Engine::new(&preset(3, their_deck()), RegistryLookup).unwrap();
    assert_eq!(
        engine.fork_for(ME, DeckKnowledge::Known, 1).err(),
        Some(ForkRefusal::OpeningHands)
    );
    assert_eq!(
        engine
            .fork_for(PlayerId::new(5), DeckKnowledge::Known, 1)
            .err(),
        Some(ForkRefusal::NoSuchSeat(PlayerId::new(5)))
    );
    keep_mulligans(&mut engine);
    assert!(!engine.state().pending_miracle.is_empty());
    assert_eq!(
        engine.fork_for(ME, DeckKnowledge::Known, 1).err(),
        Some(ForkRefusal::Continuation("pending_miracle"))
    );
    reach_main_phase(&mut engine, ME);
    assert!(engine.fork_for(ME, DeckKnowledge::Known, 1).is_ok());
}

/// Grist, the Hunger Tide is a 1/1 Insect creature card in every zone but
/// the battlefield (CR 113.6c), so its statics are registered wherever it
/// is hidden, and registered again each time it moves between a hand and a
/// library: the effect table's identities and generation move, and a text
/// override names the card's id and incarnation. Forked as it was, a game
/// with Grist in their hand and one with Grist in their library differed in
/// exactly that override — naming Grist's deck-list slot. The first version
/// refuses both, whatever else they hold.
#[test]
fn a_static_that_works_while_hidden_is_refused() {
    let grist = named("Grist, the Hunger Tide");
    let mountain = named("Mountain");
    assert_eq!(their_deck()[4].card, mountain);
    let mut in_library = Table::new(5, their_deck());
    in_library.theirs[4] = entry(grist);
    in_library.their_hand = Some(vec![entry(mountain); 7]);
    let mut in_hand = Table::new(5, their_deck());
    let mut hand = vec![entry(mountain); 7];
    hand[0] = entry(grist);
    in_hand.their_hand = Some(hand);
    let (one, two) = (in_library.settled_main(), in_hand.settled_main());
    assert_eq!(
        public_facts(&one),
        public_facts(&two),
        "I see the same game"
    );
    for engine in [&one, &two] {
        assert_eq!(
            engine.fork_for(ME, DeckKnowledge::Known, 7).err(),
            Some(ForkRefusal::MarkedHiddenCard(
                "a static that works in a hidden zone"
            ))
        );
    }
}

/// Courser of Kruphix plays with the top of their library revealed, and
/// Revelation with every hand revealed: what everybody sees is not dealt.
/// Their top card and their whole hand are the same objects in the fork;
/// the rest of their library is dealt as ever.
#[test]
fn what_a_static_reveals_stays_where_it_is() {
    let mut table = Table::new(41, their_deck());
    table.their_field = vec![entry(named("Courser of Kruphix"))];
    let source = table.settled_main();
    assert!(source.state().library_top_revealed(THEM));
    assert!(!source.state().hand_revealed(THEM));
    let forked = fork(&source, 2);
    let library =
        |e: &Engine<RegistryLookup>| e.state().zones.list(ZoneLocation::Library(THEM)).clone();
    let (real, dealt) = (library(&source), library(&forked));
    assert_eq!(real.len(), dealt.len());
    assert_eq!(real.last(), dealt.last(), "the revealed top is kept");
    assert!(
        dealt[..dealt.len() - 1].iter().all(|id| !real.contains(id)),
        "the rest is dealt"
    );
    assert_ne!(
        source.state().zones.list(ZoneLocation::Hand(THEM)),
        forked.state().zones.list(ZoneLocation::Hand(THEM)),
        "their hand is not revealed, and is dealt"
    );

    let mut table = Table::new(41, their_deck());
    table.my_field = vec![entry(named("Revelation"))];
    let source = table.settled_main();
    assert!(source.state().hand_revealed(THEM));
    let forked = fork(&source, 2);
    assert_eq!(
        source.state().zones.list(ZoneLocation::Hand(THEM)),
        forked.state().zones.list(ZoneLocation::Hand(THEM)),
        "their revealed hand is kept"
    );
    let library = library(&forked);
    assert!(
        library.iter().all(|id| !source
            .state()
            .zones
            .list(ZoneLocation::Library(THEM))
            .contains(id)),
        "their library is dealt"
    );
}

/// They activate Prodigal Sorcerer and the Sorcerer goes back to their hand
/// before the ability resolves: the ability names its source, now a hidden
/// card. A delayed trigger naming a card in their hand is the same. Both
/// are refused rather than dealt out from under what names them.
#[test]
fn what_names_a_hidden_card_is_refused() {
    let sorcerer = named("Prodigal Sorcerer");
    let mut table = Table::new(43, their_deck());
    table.their_field = vec![entry(sorcerer)];
    let mut engine = table.settled_main();
    engine.apply(ME, PlayerAction::PassPriority).unwrap();
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("they have priority: {:?}", engine.pending());
    };
    assert_eq!(player, THEM);
    let &(source, ability_index) = legal
        .abilities
        .iter()
        .find(|&&(id, _)| {
            engine
                .state()
                .object(id)
                .and_then(|o| o.card)
                .is_some_and(|c| c.index == sorcerer)
        })
        .expect("the Sorcerer's ability is offered");
    engine
        .apply(
            THEM,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .unwrap();
    while !matches!(engine.pending(), Pending::Priority { .. }) {
        let (seat, action) = answer_one(&engine).expect("a target to name");
        engine.apply(seat, action).unwrap();
    }
    assert!(!stack_is_empty(&engine), "the ability waits on the stack");
    engine
        .dev_state_mut(ME)
        .expect("the fixture trusts its seats")
        .move_object(
            source,
            ZoneLocation::Hand(THEM),
            ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    // The developer door journals the bounce; read it, as the next pass would.
    engine.trigger_scan_seq = engine.state.journal.last_seq();
    assert_eq!(
        engine.fork_for(ME, DeckKnowledge::Known, 1).err(),
        Some(ForkRefusal::ReferencedHiddenCard("an object"))
    );

    let mut engine = settled_main(43, their_deck());
    let card = engine.state().zones.list(ZoneLocation::Hand(THEM))[0];
    let version = engine.state().object(card).unwrap().version;
    engine
        .dev_state_mut(ME)
        .expect("the fixture trusts its seats")
        .delayed
        .push(crate::state::DelayedTrigger {
            controller: THEM,
            when: crate::state::DelayedWhen::NextEndStep,
            action: crate::state::DelayedAction::ReturnToHandDuringUntap { card, version },
        });
    assert_eq!(
        engine.fork_for(ME, DeckKnowledge::Known, 1).err(),
        Some(ForkRefusal::ReferencedHiddenCard("a delayed trigger"))
    );
}

/// A card in their hand revealed by its own rider (Firestorm Phoenix's
/// return) is a hidden card with rules state of its own; a commander that
/// has gone into their library is a known card in an unknown place; a loop
/// watch that has saved a signature has hashed their real hand; and a
/// standing answer of theirs in the journal is their own setting. None is
/// forked.
#[test]
fn every_refusal_has_its_case() {
    let mut engine = settled_main(47, their_deck());
    let card = engine.state().zones.list(ZoneLocation::Hand(THEM))[0];
    engine
        .dev_state_mut(ME)
        .unwrap()
        .object_mut(card)
        .unwrap()
        .riders
        .push(crate::object::Rider::RevealedInHandCantPlayUntilTurn);
    assert_eq!(
        engine.fork_for(ME, DeckKnowledge::Known, 1).err(),
        Some(ForkRefusal::MarkedHiddenCard("riders"))
    );

    let mut engine = Duel::new(47, forest()).commander(1, &[elves()]).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, ME);
    let commander = engine.state().commanders[1][0].object;
    engine
        .dev_state_mut(ME)
        .unwrap()
        .move_object(
            commander,
            ZoneLocation::Library(THEM),
            ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .unwrap();
    engine.trigger_scan_seq = engine.state.journal.last_seq();
    assert_eq!(
        engine.fork_for(ME, DeckKnowledge::Known, 1).err(),
        Some(ForkRefusal::HiddenCommander)
    );

    let mut engine = settled_main(47, their_deck());
    engine.action_loops.step(Some(0xfeed));
    assert_eq!(
        engine.fork_for(ME, DeckKnowledge::Known, 1).err(),
        Some(ForkRefusal::LoopWatch)
    );
    let mut engine = settled_main(47, their_deck());
    engine.breaking_loop = true;
    assert_eq!(
        engine.fork_for(ME, DeckKnowledge::Known, 1).err(),
        Some(ForkRefusal::LoopWatch)
    );

    let mut engine = settled_main(47, their_deck());
    let answered = |player| GameEvent::AutoAnswered {
        player,
        ability: baylee_core::ids::AbilityRef::new(elves(), 0),
        object: None,
        answer: crate::event::PolicyAnswer::Passed,
    };
    engine.state.journal.record(answered(ME));
    engine.trigger_scan_seq = engine.state.journal.last_seq();
    assert!(
        engine.fork_for(ME, DeckKnowledge::Known, 1).is_ok(),
        "my own standing answers are mine to see"
    );
    engine.state.journal.record(answered(THEM));
    engine.trigger_scan_seq = engine.state.journal.last_seq();
    assert_eq!(
        engine.fork_for(ME, DeckKnowledge::Known, 1).err(),
        Some(ForkRefusal::PrivatePolicy)
    );
}

/// The settle drops only the text overrides of what the deal removed. My
/// Glorious Anthem's static carries one and the Anthem phases out: the
/// static is parked, not gone (CR 702.26b), and the fork keeps the override
/// for when it phases in. An override quoting a card in their hand goes with
/// the card. Both are written by hand: no card in the registry gives a
/// printed static an override and phases, and the audit lets an override
/// quoting a hidden card through only because the settle drops it.
#[test]
fn a_parked_static_keeps_its_override_and_a_dealt_card_takes_its_own() {
    let anthem_card = named("Glorious Anthem");
    let mut table = Table::new(43, their_deck());
    table.my_field = vec![entry(anthem_card)];
    let mut source = table.settled_main();
    let anthem = *source
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .find(|&&id| {
            source
                .state()
                .object(id)
                .and_then(|o| o.card)
                .is_some_and(|c| c.index == anthem_card)
        })
        .expect("the Anthem is on my battlefield");
    let hidden = source.state().zones.list(ZoneLocation::Hand(THEM))[0];
    let state = source
        .dev_state_mut(ME)
        .expect("the fixture trusts its seats");
    let effect = state
        .effects
        .iter()
        .find(|fx| fx.source == Some(anthem))
        .expect("the Anthem's static is registered")
        .id;
    let anthem_text = TextOrigin::Live(state.source_identity(anthem).unwrap());
    let quoted = TextOrigin::Live(state.source_identity(hidden).unwrap());
    let unregistered = baylee_core::ids::EffectId::new(u32::MAX);
    // Read by nothing in either game, and quoting nothing hidden: the fork
    // carries it as the game does, until the end of the turn prunes both.
    let stale = (baylee_core::ids::EffectId::new(u32::MAX - 1), anthem_text);
    state.effect_text_overrides.push((effect, anthem_text));
    state.effect_text_overrides.push((unregistered, quoted));
    state.effect_text_overrides.push(stale);
    state.phase_out(&[anthem]);
    source.sync_static_effects();
    // The phasing is journaled and the machine has not run since; read it,
    // as the next pass would, so the board is what is forked.
    source.trigger_scan_seq = source.state.journal.last_seq();
    assert!(
        source
            .state()
            .effects
            .iter()
            .all(|fx| fx.source != Some(anthem)),
        "the phased-out Anthem's static is parked"
    );

    let forked = fork(&source, 5);
    let overrides = &forked.state().effect_text_overrides;
    assert!(
        overrides.contains(&(effect, anthem_text)),
        "the parked static keeps its override: {overrides:?}"
    );
    assert!(
        !overrides.iter().any(|&(_, origin)| origin == quoted),
        "the override quoting a dealt card goes with it"
    );
    assert!(
        overrides.contains(&stale),
        "nothing else goes, whether its effect is registered or not"
    );
    assert_names_none(&forked, &[hidden]);
}

fn card_of(engine: &Engine<RegistryLookup>, id: ObjectId) -> CardIndex {
    engine
        .state()
        .object(id)
        .and_then(|o| o.card)
        .expect("a card")
        .index
}

/// Their instant, cast in my first main phase once I have passed on my
/// empty stack, off their battlefield's mana; both pass until it has
/// resolved, and every card question it asks them is answered by `choose`.
/// Back in my main phase with the stack empty: settled.
fn their_instant(
    table: &Table,
    spell: CardIndex,
    choose: impl Fn(&Engine<RegistryLookup>, &[ObjectId]) -> Vec<ObjectId>,
) -> Engine<RegistryLookup> {
    let mut engine = table.settled_main();
    engine.apply(ME, PlayerAction::PassPriority).unwrap();
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == THEM),
        "they hold priority in my main phase: {:?}",
        engine.pending()
    );
    cast_from_hand(&mut engine, THEM, spell);
    for _ in 0..20 {
        match engine.pending().clone() {
            Pending::ChooseCards {
                player, options, ..
            } if player == THEM => {
                let objects = choose(&engine, &options);
                engine
                    .apply(THEM, PlayerAction::ChooseObjects { objects })
                    .expect("their choice stands");
            }
            Pending::Priority { player, .. } if !stack_is_empty(&engine) => {
                let actor = engine.decision_actor().unwrap_or(player);
                engine.apply(actor, PlayerAction::PassPriority).unwrap();
            }
            Pending::Priority { player, .. } => {
                assert_eq!(player, ME, "back in my main phase");
                // A card they drew opened its miracle check, offered once
                // everyone has passed on the empty stack (CR 702.94a); the
                // fork refuses it until then (`pending_miracle`).
                pass_until(&mut engine, |e| {
                    e.state.pending_miracle.is_empty()
                        && matches!(e.pending(), Pending::Priority { .. })
                });
                return engine;
            }
            other => panic!("an unexpected question: {other:?}"),
        }
    }
    panic!("the spell did not resolve");
}

/// Two games apart only in what I could not see fork into one game: every
/// field of the two forks equal, journals whole, through two more turns of
/// play; and neither fork names a card its deal removed (`retired`, one
/// list per game).
#[track_caller]
fn assert_twins_fork_alike(
    one: &Engine<RegistryLookup>,
    two: &Engine<RegistryLookup>,
    retired: [&[ObjectId]; 2],
) {
    assert_eq!(public_facts(one), public_facts(two), "I see the same game");
    let (mut a, mut b) = (fork(one, 7), fork(two, 7));
    assert_names_none(&a, retired[0]);
    assert_names_none(&b, retired[1]);
    let start = a.state().turn.number;
    for _ in 0..80 {
        assert_indistinguishable(&a, &b);
        assert_eq!(a.journal().entries(), b.journal().entries());
        if a.state().turn.number >= start + 2 {
            return;
        }
        step(&mut a);
        step(&mut b);
    }
    panic!("the forks did not play two more turns");
}

/// They Brainstorm in my main phase: draw three, put two back on top of
/// their library. The twins put back different cards, so their hands and
/// their library tops differ, and I saw neither. The forks are one game,
/// and neither names a card that went back.
#[test]
fn cards_they_put_back_do_not_reach_the_fork() {
    let brainstorm = named("Brainstorm");
    let (mountain, swamp) = (named("Mountain"), named("Swamp"));
    let mut table = Table::new(53, their_deck());
    table.their_field = vec![entry(named("Island"))];
    table.their_hand = Some(vec![
        entry(brainstorm),
        entry(mountain),
        entry(swamp),
        entry(elves()),
        entry(quiet_artifact()),
        entry(mountain),
        entry(swamp),
    ]);
    let putting_back = |kinds: [CardIndex; 2]| {
        their_instant(&table, brainstorm, |engine, options| {
            let mut chosen = Vec::new();
            for kind in kinds {
                let card = options
                    .iter()
                    .copied()
                    .find(|&id| card_of(engine, id) == kind && !chosen.contains(&id))
                    .expect("a card of that kind in their hand");
                chosen.push(card);
            }
            chosen
        })
    };
    let one = putting_back([mountain, swamp]);
    let two = putting_back([elves(), quiet_artifact()]);
    let top = |engine: &Engine<RegistryLookup>| {
        let library = engine.state().zones.list(ZoneLocation::Library(THEM));
        sorted(
            library[library.len() - 2..]
                .iter()
                .map(|&id| card_of(engine, id))
                .collect(),
        )
    };
    assert_ne!(top(&one), top(&two), "different cards went back");
    assert_twins_fork_alike(&one, &two, [&retired_ids(&one), &retired_ids(&two)]);
}

/// They tutor in my main phase (Vampiric Tutor: search their library for a
/// card, shuffle, put it on top). The twins differ only in the order of
/// the hidden libraries before the search; they find the same card. The
/// forks are one game, and neither names a card of the real libraries.
#[test]
fn a_tutor_and_its_shuffle_do_not_reach_the_fork() {
    let tutor = named("Vampiric Tutor");
    let swamp = named("Swamp");
    let tutoring = |seed| {
        let mut table = Table::new(seed, their_deck());
        table.their_field = vec![entry(swamp)];
        table.their_hand = Some(vec![
            entry(tutor),
            entry(swamp),
            entry(named("Mountain")),
            entry(swamp),
            entry(named("Mountain")),
            entry(swamp),
            entry(named("Mountain")),
        ]);
        their_instant(&table, tutor, |engine, options| {
            vec![
                options
                    .iter()
                    .copied()
                    .find(|&id| card_of(engine, id) == elves())
                    .expect("Elves in their library"),
            ]
        })
    };
    let (before_one, before_two) = (
        Table::new(61, their_deck()).settled_main(),
        Table::new(62, their_deck()).settled_main(),
    );
    assert_ne!(
        identities(&hidden_zones(&before_one))[1],
        identities(&hidden_zones(&before_two))[1],
        "their libraries are in different orders"
    );
    let (one, two) = (tutoring(61), tutoring(62));
    let top = |engine: &Engine<RegistryLookup>| {
        let library = engine.state().zones.list(ZoneLocation::Library(THEM));
        card_of(engine, *library.last().unwrap())
    };
    assert_eq!(
        (top(&one), top(&two)),
        (elves(), elves()),
        "both found the Elves"
    );
    assert_twins_fork_alike(&one, &two, [&retired_ids(&one), &retired_ids(&two)]);
}

/// I control their turn (Mindslaver, scheduled as
/// `Effect::ControlPlayerNextTurn` schedules it), so I may look at their
/// hand (CR 722.4): the fork keeps it as I see it, the same objects, and
/// deals their library. Taken in their upkeep, before their draw: a card
/// drawn from a shuffled library keeps the deck-list id of the slot that
/// was on top, so twins taken after it would differ in that id.
#[test]
fn the_hand_of_a_player_i_control_stays_as_i_see_it() {
    let swamp = named("Swamp");
    let controlled = |seed| {
        let mut table = Table::new(seed, their_deck());
        table.their_hand = Some(vec![
            entry(elves()),
            entry(quiet_artifact()),
            entry(named("Mountain")),
            entry(swamp),
            entry(elves()),
            entry(swamp),
            entry(named("Mountain")),
        ]);
        let mut engine = table.settled_main();
        engine
            .state
            .turn_controls
            .push(crate::state::PlayerTurnControl {
                controller: ME,
                player: THEM,
                turn: engine.state.turn.number + 1,
                began: None,
            });
        pass_until(&mut engine, |e| {
            e.state().turn.active == THEM
                && e.state().turn.step == crate::turn::Step::Upkeep
                && matches!(e.pending(), Pending::Priority { .. })
        });
        assert_eq!(
            engine.state().turn.step,
            crate::turn::Step::Upkeep,
            "their upkeep, under my control"
        );
        assert!(engine.may_inspect_private(ME, THEM), "I control their turn");
        engine
    };
    let (one, two) = (controlled(71), controlled(72));
    let hand = |e: &Engine<RegistryLookup>| e.state().zones.list(ZoneLocation::Hand(THEM)).clone();
    let library =
        |e: &Engine<RegistryLookup>| e.state().zones.list(ZoneLocation::Library(THEM)).clone();
    assert_eq!(hand(&one), hand(&two), "I see the same hand in both");
    let forked = fork(&one, 7);
    assert_eq!(hand(&forked), hand(&one), "their hand stays as I see it");
    assert!(
        library(&forked)
            .iter()
            .all(|id| !library(&one).contains(id)),
        "their library is dealt"
    );
    let hidden = |e: &Engine<RegistryLookup>| -> Vec<ObjectId> {
        [ZoneLocation::Library(THEM), ZoneLocation::Library(ME)]
            .into_iter()
            .flat_map(|loc| e.state().zones.list(loc).clone())
            .collect()
    };
    assert_twins_fork_alike(&one, &two, [&hidden(&one), &hidden(&two)]);
}

/// A spell on the stack whose payment names a card in their hand (a card
/// discarded to pay for it, back in that hand): the record points at a
/// hidden card, and the fork is refused rather than dealt out from under
/// it. Written by hand: no card here pays by discarding and then returns
/// the discarded card in response.
#[test]
fn a_payment_naming_a_hidden_card_is_refused() {
    let mut engine = settled_main(73, their_deck());
    elves_passed_over(&mut engine);
    let spell = engine.state().zones.list(ZoneLocation::Stack)[0];
    let hidden = engine.state().zones.list(ZoneLocation::Hand(THEM))[0];
    let state = engine
        .dev_state_mut(ME)
        .expect("the fixture trusts its seats");
    let discarded = state.source_identity(hidden).unwrap();
    state
        .object_mut(spell)
        .unwrap()
        .paid
        .get_or_insert_with(Default::default)
        .discarded
        .push(discarded);
    assert_eq!(
        engine.fork_for(ME, DeckKnowledge::Known, 1).err(),
        Some(ForkRefusal::ReferencedHiddenCard("an object"))
    );
}

/// A fork is a simulation, not a seat at a table: nobody in it may rewrite
/// the board, and their priority holds and standing answers are theirs, not
/// mine to simulate them by. Mine stay mine.
#[test]
fn a_fork_drops_the_chairs_settings() {
    let mut source = settled_main(53, their_deck());
    source.automation[0].hold = crate::choice::PriorityHold::Always;
    source.automation[1].set_yield(baylee_core::ids::AbilityRef::new(elves(), 0), true);
    assert!(source.dev_state_mut(ME).is_some());
    let mut fork = fork(&source, 3);
    assert_eq!(fork.automation[0], source.automation[0], "mine are kept");
    assert_eq!(
        fork.automation[1],
        crate::choice::SeatAutomation::default(),
        "theirs are the defaults"
    );
    assert!(fork.dev_state_mut(ME).is_none(), "no developer door");
    assert!(fork.dev_state_mut(THEM).is_none());
}

/// Notes the game made about a hidden card — an ability used this turn, a
/// Wand of Ith's look — go with the card, and a dealt card takes its own:
/// a Plague Rats defines its power and toughness (CR 604.3) and an Anger
/// works from a graveyard, so each dealt one is noted under its new id.
/// Their sideboard is dealt like their hand; mine is mine to see and is
/// kept.
#[test]
fn notes_go_with_the_card_and_dealt_cards_take_their_own() {
    let rats = named("Plague Rats");
    let anger = named("Anger");
    let mut table = Table::new(59, deck(&[named("Mountain"), rats, anger, named("Swamp")]));
    table.their_side = vec![entry(named("Lightning Bolt")), entry(rats), entry(anger)];
    table.my_side = vec![entry(named("Unsummon"))];
    let mut source = table.settled_main();
    let card = source.state().zones.list(ZoneLocation::Hand(THEM))[0];
    let version = source.state().object(card).unwrap().version;
    let state = source.dev_state_mut(ME).unwrap();
    state.ability_fires.insert(
        (
            baylee_core::ids::DamageSourceRef {
                object: card,
                version,
            },
            0,
        ),
        1,
    );
    state.per_turn.wand_of_ith_targets.push((ME, card));
    let retired = retired_ids(&source);

    let fork = fork(&source, 9);
    assert_names_none(&fork, &retired);
    // The cards each list notes, by identity: as many in the fork as in the
    // game, now under the dealt cards' ids.
    let noted = |engine: &Engine<RegistryLookup>, ids: Vec<ObjectId>| -> Vec<CardIndex> {
        sorted(
            ids.iter()
                .filter_map(|&id| engine.state().object(id).and_then(|o| o.card))
                .map(|card| card.index)
                .collect(),
        )
    };
    let cda = |engine: &Engine<RegistryLookup>| {
        noted(
            engine,
            engine
                .state()
                .printed_pt_cda
                .iter()
                .map(|(id, _)| *id)
                .collect(),
        )
    };
    let offboard = |engine: &Engine<RegistryLookup>| {
        noted(engine, engine.state().offboard_static_candidates.clone())
    };
    assert!(
        cda(&source).len() > 1 && offboard(&source).len() > 1,
        "the fixture has some"
    );
    assert_eq!(cda(&fork), cda(&source));
    assert!(cda(&fork).iter().all(|&card| card == rats));
    assert_eq!(offboard(&fork), offboard(&source));
    assert!(offboard(&fork).iter().all(|&card| card == anger));
    let side = |engine: &Engine<RegistryLookup>, seat| {
        engine
            .state()
            .zones
            .list(ZoneLocation::OutsideGame(seat))
            .clone()
    };
    assert_eq!(side(&fork, ME), side(&source, ME), "my sideboard is kept");
    assert!(
        side(&fork, THEM)
            .iter()
            .all(|id| !side(&source, THEM).contains(id))
    );
    let identities_of = |engine: &Engine<RegistryLookup>, ids: &[ObjectId]| -> Vec<CardIndex> {
        sorted(
            ids.iter()
                .map(|&id| {
                    engine
                        .state()
                        .object(id)
                        .and_then(|o| o.card)
                        .unwrap()
                        .index
                })
                .collect(),
        )
    };
    assert_eq!(
        identities_of(&fork, &side(&fork, THEM)),
        identities_of(&source, &side(&source, THEM)),
        "their sideboard holds the same cards"
    );
}

/// Every id the deal removes from `engine`'s fork for me: their hand and
/// library, their sideboard, my library.
fn retired_ids(engine: &Engine<RegistryLookup>) -> Vec<ObjectId> {
    [
        ZoneLocation::Hand(THEM),
        ZoneLocation::Library(THEM),
        ZoneLocation::OutsideGame(THEM),
        ZoneLocation::Library(ME),
    ]
    .into_iter()
    .flat_map(|loc| engine.state().zones.list(loc).clone())
    .collect()
}

/// That no field of `engine` — the complete print, journal included — names
/// any of `ids`.
#[track_caller]
fn assert_names_none(engine: &Engine<RegistryLookup>, ids: &[ObjectId]) {
    let print = engine.fingerprint();
    let journal = format!("{:?}", engine.journal().entries());
    for id in ids {
        // `ObjectId(slot#generation)`: the closing parenthesis keeps slot 1
        // from matching slot 12.
        let name = format!("{id:?}");
        let fields: Vec<&str> = print
            .0
            .iter()
            .filter(|(_, value)| value.contains(&name))
            .map(|(field, _)| *field)
            .collect();
        assert!(fields.is_empty(), "{fields:?} still name {name}");
        assert!(!journal.contains(&name), "the journal still names {name}");
    }
}
