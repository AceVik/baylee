//! Forks dealt from a public prior (`DeckKnowledge::Prior`): the seat knows
//! its own deck list and no other, so the other players' hidden cards are
//! drawn from what it believes of them and the real ones are removed
//! unread. Two games apart only in what the seat cannot see — their deck
//! list, their hand, their library — fork into the same game.

use super::*;
use crate::engine::{PriorFault, PublicPrior};
use std::fmt::Write as _;

fn plains() -> CardIndex {
    named("Plains")
}

fn lions() -> CardIndex {
    named("Savannah Lions")
}

fn grist() -> CardIndex {
    named("Grist, the Hunger Tide")
}

/// A deck with no card of `their_deck` in it, and as many names.
fn other_deck() -> Vec<DeckEntry> {
    deck(&[
        named("Island"),
        named("Hill Giant"),
        named("Giant Growth"),
        named("Lightning Bolt"),
    ])
}

/// What I take them to hold, having seen none of it: cards of neither deck
/// (Plains, Savannah Lions) and one of each (Mountain, Giant Growth).
fn belief() -> PublicPrior {
    PublicPrior::new().with_player(
        THEM,
        vec![
            (plains(), 5),
            (lions(), 3),
            (named("Mountain"), 2),
            (named("Giant Growth"), 1),
        ],
    )
}

fn fork_on(
    engine: &Engine<RegistryLookup>,
    prior: &PublicPrior,
    policy_seed: u64,
) -> Engine<RegistryLookup> {
    engine
        .fork_for(ME, DeckKnowledge::Prior(prior), policy_seed)
        .unwrap_or_else(|refusal| panic!("a settled priority forks on a prior: {refusal}"))
}

fn cards_in(engine: &Engine<RegistryLookup>, loc: ZoneLocation) -> Vec<CardIndex> {
    engine
        .state()
        .zones
        .list(loc)
        .iter()
        .map(|&id| card_of(engine, id))
        .collect()
}

/// Every zone of the game, card by card: each object's id, card, print,
/// name as text, incarnation and timestamp, and whether it is tapped. The
/// names are read through the table, so two games that number their names
/// differently and hold the same cards print the same.
fn every_zone(engine: &Engine<RegistryLookup>) -> String {
    let s = engine.state();
    let mut zones = vec![ZoneLocation::Battlefield, ZoneLocation::Stack];
    for player in [ME, THEM] {
        zones.extend([
            ZoneLocation::Hand(player),
            ZoneLocation::Library(player),
            ZoneLocation::Graveyard(player),
            ZoneLocation::Exile(player),
            ZoneLocation::Command(player),
            ZoneLocation::OutsideGame(player),
        ]);
    }
    let mut print = String::new();
    for loc in zones {
        write!(print, "{loc:?}:").expect("a string takes any text");
        for &id in s.zones.list(loc) {
            let object = s.object(id).expect("a zone lists objects");
            let now = object.characteristics();
            write!(
                print,
                " ({id:?} {:?} {:?} {:?}/{:?} v{} t{} {:?} {:?})",
                object.card,
                s.names.get(now.name),
                now.power,
                now.toughness,
                object.version,
                object.timestamp,
                now.types,
                object.status,
            )
            .expect("a string takes any text");
        }
        print.push('\n');
    }
    print
}

/// Everything two forks are compared by where their name tables are not
/// numbered alike: every zone card by card, what everybody sees, the
/// question asked with its whole offer, and the random stream to come.
fn whole_game(engine: &Engine<RegistryLookup>) -> String {
    format!(
        "{}{}\n{:?}\n{:?}",
        every_zone(engine),
        public_facts(engine),
        engine.pending(),
        next_draws(engine),
    )
}

/// Plays for whoever is asked, by the question alone: a land if one may be
/// played, the first castable card if one may be cast, a pass otherwise,
/// no attackers and no blockers. Returns what it answered.
fn play(engine: &mut Engine<RegistryLookup>) -> String {
    let actor = engine.decision_actor().expect("somebody is asked");
    let action = match engine.pending() {
        Pending::Priority { legal, .. } => {
            if let Some(&card) = legal.lands.first() {
                PlayerAction::PlayLand { card }
            } else {
                PlayerAction::PassPriority
            }
        }
        Pending::ChooseAttackers { .. } => PlayerAction::DeclareAttackers { attackers: vec![] },
        Pending::ChooseBlockers { .. } => PlayerAction::DeclareBlockers { blockers: vec![] },
        _ => {
            answer_one(engine)
                .unwrap_or_else(|rest| panic!("unanswerable: {rest:?}"))
                .1
        }
    };
    let said = format!("{actor:?} {action:?}");
    engine.apply(actor, action).expect("the answer is legal");
    said
}

/// The fields of the complete comparison two forks of twins under a prior
/// always differ in: the two caches a fork carries from its game, which
/// still hold the names and faces of the real decks.
const CACHES: [&str; 2] = ["state.names", "state.bases"];

/// Plays two forks side by side for `turns` turns, with the same answers,
/// and holds them equal at every decision: every zone card by card, the
/// offer, the stream and the journal. With `complete`, every field of the
/// complete comparison too, but the caches and the fields it names; without
/// it the forks number their names differently, which the arena and every
/// copy of a face print.
#[track_caller]
fn assert_forks_play_alike(
    a: &mut Engine<RegistryLookup>,
    b: &mut Engine<RegistryLookup>,
    turns: u32,
    complete: Option<&[&str]>,
) {
    let start = a.state().turn.number;
    let mut lands = 0;
    for _ in 0..200 {
        assert_eq!(whole_game(a), whole_game(b));
        assert_eq!(a.journal().entries(), b.journal().entries());
        if let Some(apart) = complete {
            let differing: Vec<&str> = a
                .fingerprint()
                .differing(&b.fingerprint())
                .into_iter()
                .filter(|field| !CACHES.contains(field) && !apart.contains(field))
                .collect();
            assert!(differing.is_empty(), "the forks differ in {differing:?}");
        }
        if a.state().turn.number >= start + turns {
            assert!(lands > 0, "a dealt land was played");
            return;
        }
        let (said_a, said_b) = (play(a), play(b));
        assert_eq!(said_a, said_b, "the same answer to the same question");
        lands += usize::from(said_a.contains("PlayLand"));
    }
    panic!("the forks did not play {turns} more turns");
}

/// Two games apart in everything I cannot see: their deck list — no card in
/// common — their hand and their library. I have seen the same game, and
/// forking each on the same prior and policy seed gives one game: the same
/// card in every slot of every zone under the same id, the same offer, the
/// same stream, the same journal, and every field of the complete
/// comparison but the two caches. They stay one game as both are played on
/// with the same answers: my Elves cast and resolved, then three turns of
/// draws, land plays from the dealt hands, and passes.
///
/// The complete comparison can hold here because the two lists intern as
/// many names and the prior lists cards of neither, so both forks give the
/// dealt cards' names the same numbers; the test below is the case where
/// they do not.
#[test]
fn their_deck_list_does_not_reach_a_fork_on_a_prior() {
    let one = settled_main(1, their_deck());
    let two = settled_main(2, other_deck());
    assert_eq!(
        public_facts(&one),
        public_facts(&two),
        "I see the same game"
    );
    let theirs = |engine: &Engine<RegistryLookup>| {
        let mut cards = cards_in(engine, ZoneLocation::Hand(THEM));
        cards.extend(cards_in(engine, ZoneLocation::Library(THEM)));
        sorted(cards)
    };
    assert!(
        theirs(&one).iter().all(|card| !theirs(&two).contains(card)),
        "their decks share no card"
    );
    assert_eq!(one.state().names.len(), two.state().names.len());

    // Cards of neither list; Plague Rats defines its own power and
    // toughness wherever it is (CR 604.3).
    let prior = PublicPrior::new().with_player(
        THEM,
        vec![(plains(), 5), (lions(), 3), (named("Plague Rats"), 2)],
    );
    let (mut a, mut b) = (fork_on(&one, &prior, 7), fork_on(&two, &prior, 7));
    assert_eq!(hidden_zones(&a), hidden_zones(&b), "the same cards dealt");
    assert_names_none(&a, &retired_ids(&one));
    assert_names_none(&b, &retired_ids(&two));
    assert_eq!(whole_game(&a), whole_game(&b));

    for fork in [&mut a, &mut b] {
        elves_passed_over(fork);
        fork.apply(THEM, PlayerAction::PassPriority).unwrap();
        assert!(stack_is_empty(fork), "the Elves resolved");
    }
    assert_forks_play_alike(&mut a, &mut b, 3, Some(&[]));
}

/// The same where the prior lists cards of their real lists (a Mountain of
/// one, a Giant Growth of the other) and the two lists do not even intern
/// as many names, so the forks number the names of the dealt cards
/// differently. No rule orders or iterates by a name's number, so the games
/// are still one: compared by every zone with its names as text, the offer,
/// the stream and the journal.
#[test]
fn a_shorter_list_of_theirs_does_not_reach_a_fork_on_a_prior() {
    let one = settled_main(1, their_deck());
    let two = settled_main(2, deck(&[named("Island"), named("Hill Giant")]));
    assert_eq!(
        public_facts(&one),
        public_facts(&two),
        "I see the same game"
    );
    assert_ne!(
        one.state().names.len(),
        two.state().names.len(),
        "their lists intern different numbers of names"
    );
    let prior = belief();
    let (mut a, mut b) = (fork_on(&one, &prior, 7), fork_on(&two, &prior, 7));
    assert_eq!(hidden_zones(&a), hidden_zones(&b), "the same cards dealt");
    assert_forks_play_alike(&mut a, &mut b, 3, None);
}

/// Grist, the Hunger Tide in their hand registers its static there and has
/// registered it in their library before: the effect table of that game has
/// counted registrations a game without Grist has not, and holds an effect
/// and a text override naming Grist. Whether they hold such a card is
/// theirs to know, so neither twin is refused, as the one with Grist is
/// where I know the lists. The fork removes the card with its effect and
/// its override, and the twins — one with Grist, one with a list sharing no
/// card — are one game.
///
/// What the real Grist leaves is in the effect table: its count of
/// registrations, which is the id the next effect takes, and its
/// generation, which keys the projection (`characteristics_generation`
/// repeats it). No rule reads either number but to tell two effects apart
/// or to know a cache is stale. With a prior that deals Grists of its own
/// each is registered as the fork's own card, a 1/1 creature in their hand
/// in both forks, under effect ids that start at that count, which their
/// overrides repeat.
#[test]
fn a_static_that_works_while_hidden_does_not_reach_a_fork_on_a_prior() {
    let mut with = Table::new(5, their_deck());
    let mut hand = vec![entry(named("Mountain")); 7];
    hand[0] = entry(grist());
    with.their_hand = Some(hand);
    let mut without = Table::new(6, other_deck());
    let mut hand = vec![entry(named("Island")); 7];
    hand[0] = entry(named("Hill Giant"));
    hand[1] = entry(named("Raging Goblin"));
    without.their_hand = Some(hand);
    let (one, two) = (with.settled_main(), without.settled_main());
    assert_eq!(
        public_facts(&one),
        public_facts(&two),
        "I see the same game"
    );
    assert_eq!(one.state().names.len(), two.state().names.len());
    assert_eq!(
        one.fork_for(ME, DeckKnowledge::Known, 7).err(),
        Some(ForkRefusal::MarkedHiddenCard(
            "a static that works in a hidden zone"
        )),
        "refused where I know the lists"
    );
    let real = one.state().zones.list(ZoneLocation::Hand(THEM))[0];
    assert_eq!(card_of(&one, real), grist());
    assert!(
        one.state()
            .effects
            .iter()
            .any(|effect| effect.source == Some(real)),
        "the real Grist's static is registered"
    );
    assert!(two.state().effects.iter().next().is_none());

    let prior = PublicPrior::new().with_player(THEM, vec![(plains(), 3), (lions(), 1)]);
    let (mut a, mut b) = (fork_on(&one, &prior, 7), fork_on(&two, &prior, 7));
    assert_names_none(&a, &retired_ids(&one));
    for fork in [&a, &b] {
        assert!(fork.state().effects.iter().next().is_none());
        assert!(fork.state().effect_text_overrides.is_empty());
    }
    assert_forks_play_alike(
        &mut a,
        &mut b,
        3,
        Some(&["state.effects", "state.characteristics_generation"]),
    );

    let prior = PublicPrior::new().with_player(THEM, vec![(grist(), 1), (plains(), 3)]);
    let (mut a, mut b) = (fork_on(&one, &prior, 7), fork_on(&two, &prior, 7));
    assert_names_none(&a, &retired_ids(&one));
    for fork in [&a, &b] {
        let dealt: Vec<ObjectId> = fork
            .state()
            .zones
            .list(ZoneLocation::Hand(THEM))
            .iter()
            .copied()
            .filter(|&id| card_of(fork, id) == grist())
            .collect();
        assert!(!dealt.is_empty(), "the prior dealt them a Grist");
        for id in dealt {
            let grist = fork.state().object(id).unwrap().characteristics();
            assert_eq!((grist.power, grist.toughness), (Some(1), Some(1)));
        }
        let sources: Vec<ObjectId> = fork
            .state()
            .effects
            .iter()
            .filter_map(|effect| effect.source)
            .collect();
        assert!(!sources.is_empty(), "the dealt Grists' statics");
        assert!(
            sources.iter().all(|&id| fork.state().object(id).is_some()),
            "every effect's source is in the fork"
        );
    }
    assert_forks_play_alike(&mut a, &mut b, 3, None);
}

/// The dealt cards are the prior's and nothing else's. With one card listed
/// every hidden slot of theirs holds it — their hand, their library, their
/// cards outside the game — at the sizes the zones had, as a card with no
/// printing of its own. With two cards at three to one, the fixed seed
/// deals them near three to one.
#[test]
fn their_hidden_cards_are_drawn_from_the_prior() {
    let mut table = Table::new(9, their_deck());
    table.their_side = vec![entry(named("Lightning Bolt")); 3];
    let source = table.settled_main();
    let zones = [
        ZoneLocation::Hand(THEM),
        ZoneLocation::Library(THEM),
        ZoneLocation::OutsideGame(THEM),
    ];

    let only = PublicPrior::new().with_player(THEM, vec![(lions(), 4)]);
    let fork = fork_on(&source, &only, 3);
    for loc in zones {
        let dealt = fork.state().zones.list(loc);
        assert_eq!(dealt.len(), source.state().zones.list(loc).len(), "{loc:?}");
        for &id in dealt {
            let object = fork.state().object(id).unwrap();
            let card = object.card.unwrap();
            assert_eq!(card.index, lions());
            assert_eq!(card.print, PrintRef::UNKNOWN);
            assert!(object.physical_rules.is_none());
            assert_eq!((object.version, object.timestamp), (0, 0));
            assert_eq!(
                fork.state().names.get(object.characteristics().name),
                "Savannah Lions"
            );
        }
    }

    let weighted = PublicPrior::new().with_player(THEM, vec![(plains(), 3), (lions(), 1)]);
    let dealt_on = |policy_seed| {
        let fork = fork_on(&source, &weighted, policy_seed);
        let dealt: Vec<CardIndex> = zones
            .into_iter()
            .flat_map(|loc| cards_in(&fork, loc))
            .collect();
        assert_eq!(dealt.len(), 43, "seven, thirty-three and three");
        let count = |card| dealt.iter().filter(|&&dealt| dealt == card).count();
        (count(plains()), count(lions()))
    };
    assert_eq!(dealt_on(3), (26, 17), "one deal of forty-three");
    let over_forty_seeds = (0..40)
        .map(dealt_on)
        .fold((0, 0), |sum, dealt| (sum.0 + dealt.0, sum.1 + dealt.1));
    assert_eq!(
        over_forty_seeds,
        (1262, 458),
        "near three to one over 1720 cards"
    );

    // The order the caller lists the cards in, and how it splits a weight,
    // do not matter.
    let relisted =
        PublicPrior::new().with_player(THEM, vec![(lions(), 1), (plains(), 1), (plains(), 2)]);
    assert_eq!(relisted, weighted);
    assert_eq!(
        relisted.cards(THEM),
        [(plains(), 3), (lions(), 1)].as_slice()
    );
    assert_eq!(relisted.players().collect::<Vec<_>>(), [THEM]);
    assert!(relisted.cards(ME).is_empty());
}

/// My own library is dealt from my own cards, as where I know every list:
/// the same multiset in another order, and a prior that lists cards for me
/// changes nothing.
#[test]
fn my_library_is_my_own_cards_under_a_prior() {
    let source = settled_main(13, their_deck());
    let mine = ZoneLocation::Library(ME);
    let real = cards_in(&source, mine);
    let fork = fork_on(&source, &belief(), 3);
    let dealt = cards_in(&fork, mine);
    assert_eq!(sorted(dealt.clone()), sorted(real.clone()));
    assert_ne!(dealt, real, "in another order");
    assert_eq!(
        cards_in(&fork, ZoneLocation::Hand(ME)),
        cards_in(&source, ZoneLocation::Hand(ME)),
        "my hand is mine to see"
    );
    assert_eq!(
        source.state().zones.list(ZoneLocation::Hand(ME)),
        fork.state().zones.list(ZoneLocation::Hand(ME)),
    );

    let about_me = belief().with_player(ME, vec![(lions(), 1)]);
    assert_indistinguishable(&fork, &fork_on(&source, &about_me, 3));
}

/// Forking on a prior takes `&self` and gives nothing back, as any fork:
/// played on and shuffled, it leaves the source's hashes, stream and every
/// field what they were and what an untouched twin's are — the caches too,
/// though the fork has interned cards no deck holds.
#[test]
fn forking_on_a_prior_leaves_the_source_unchanged() {
    let control = settled_main(31, their_deck());
    let source = settled_main(31, their_deck());
    let before = record(&source);
    assert_eq!(before, record(&control), "the twins start equal");

    let mut fork = fork_on(&source, &belief(), 5);
    assert!(fork.state().names.find("Savannah Lions").is_some());
    for _ in 0..80 {
        if fork.state().turn.number >= 3 {
            break;
        }
        play(&mut fork);
    }
    assert!(fork.state().turn.number >= 3, "the fork played on");
    fork.state.shuffle_library(THEM);
    fork.state.shuffle_library(ME);

    assert_eq!(record(&source), before, "the source is untouched");
    assert_eq!(record(&source), record(&control));
    assert_eq!(next_draws(&source), next_draws(&control));
    assert_indistinguishable(&source, &control);
    assert_eq!(source.journal().entries(), control.journal().entries());
    assert_eq!(source.state().names.find("Savannah Lions"), None);
}

/// A prior that cannot be dealt from is refused, each for a fact about the
/// prior and the sizes of their hidden zones: nobody listed, an empty list,
/// a weight of zero, a card the lookup lacks, a player not at the table.
/// And not knowing the decks with no prior at all is refused as ever.
#[test]
fn a_prior_that_cannot_be_dealt_from_is_refused() {
    let source = settled_main(17, their_deck());
    let refused = |prior: PublicPrior| {
        source
            .fork_for(ME, DeckKnowledge::Prior(&prior), 1)
            .err()
            .map(|refusal| match refusal {
                ForkRefusal::UnusablePrior(fault) => fault,
                other => panic!("refused for something else: {other}"),
            })
    };
    assert_eq!(
        refused(PublicPrior::new()),
        Some(PriorFault::NoCards(THEM)),
        "nobody listed"
    );
    assert_eq!(
        refused(PublicPrior::new().with_player(ME, vec![(forest(), 1)])),
        Some(PriorFault::NoCards(THEM)),
        "only I am listed"
    );
    assert_eq!(
        refused(PublicPrior::new().with_player(THEM, vec![])),
        Some(PriorFault::NoCards(THEM)),
        "an empty list"
    );
    assert_eq!(
        refused(PublicPrior::new().with_player(THEM, vec![(plains(), 2), (lions(), 0)])),
        Some(PriorFault::NoWeight(THEM, lions()))
    );
    let nowhere = CardIndex::new(u32::MAX);
    assert_eq!(
        refused(PublicPrior::new().with_player(THEM, vec![(plains(), 2), (nowhere, 1)])),
        Some(PriorFault::UnknownCard(nowhere))
    );
    let absent = PlayerId::new(2);
    assert_eq!(
        refused(belief().with_player(absent, vec![(plains(), 1)])),
        Some(PriorFault::NoSuchPlayer(absent))
    );
    assert_eq!(refused(belief()), None, "a prior with cards for them forks");
    assert_eq!(
        source.fork_for(ME, DeckKnowledge::Unknown, 1).err(),
        Some(ForkRefusal::UnknownDecks)
    );
}

/// The same game, prior and policy seed give the same fork, field for
/// field; another policy seed deals their cards and my library differently
/// and rolls on another stream; another prior deals other cards.
#[test]
fn a_fork_on_a_prior_is_a_function_of_its_inputs() {
    let source = settled_main(19, their_deck());
    let prior = belief();
    let fork = fork_on(&source, &prior, 3);
    let again = fork_on(&source, &prior.clone(), 3);
    assert_indistinguishable(&fork, &again);
    assert_eq!(fork.snapshot_hash(), again.snapshot_hash());
    assert_eq!(next_draws(&fork), next_draws(&again));

    let other = fork_on(&source, &prior, 4);
    assert_ne!(
        identities(&hidden_zones(&fork)),
        identities(&hidden_zones(&other)),
        "another seed, another deal"
    );
    assert_ne!(next_draws(&fork), next_draws(&other));

    let lions_only = PublicPrior::new().with_player(THEM, vec![(lions(), 1)]);
    assert_ne!(
        identities(&hidden_zones(&fork))[..2],
        identities(&hidden_zones(&fork_on(&source, &lions_only, 3)))[..2],
        "another prior, other cards"
    );
}

/// What a hidden card of theirs carries is not read under a prior, so it
/// refuses nothing and goes with the card: a rider on a card in their hand
/// refuses the fork where I know the lists and not here, and the fork is
/// the one a game without the rider gives. My own library is still read: a
/// card of it that carries something is refused as ever.
#[test]
fn what_their_hidden_cards_carry_goes_with_them() {
    let plain = settled_main(47, their_deck());
    let mut marked = settled_main(47, their_deck());
    let card = marked.state().zones.list(ZoneLocation::Hand(THEM))[0];
    marked
        .dev_state_mut(ME)
        .unwrap()
        .object_mut(card)
        .unwrap()
        .riders
        .push(crate::object::Rider::RevealedInHandCantPlayUntilTurn);
    assert_eq!(
        marked.fork_for(ME, DeckKnowledge::Known, 1).err(),
        Some(ForkRefusal::MarkedHiddenCard("riders"))
    );
    let prior = belief();
    assert_indistinguishable(&fork_on(&marked, &prior, 1), &fork_on(&plain, &prior, 1));

    let mut mine = settled_main(47, their_deck());
    let card = mine.state().zones.list(ZoneLocation::Library(ME))[0];
    mine.dev_state_mut(ME)
        .unwrap()
        .object_mut(card)
        .unwrap()
        .riders
        .push(crate::object::Rider::RevealedInHandCantPlayUntilTurn);
    assert_eq!(
        mine.fork_for(ME, DeckKnowledge::Prior(&prior), 1).err(),
        Some(ForkRefusal::MarkedHiddenCard("riders"))
    );
}

/// What is public and names a card now hidden is still refused under a
/// prior, by the id alone: a commander of theirs in their library, and a
/// continuous effect about a card in their hand — whichever incarnation it
/// names, since which one the card is in counts its hidden moves.
#[test]
fn what_names_their_hidden_card_is_refused_under_a_prior() {
    let prior = belief();
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
        engine.fork_for(ME, DeckKnowledge::Prior(&prior), 1).err(),
        Some(ForkRefusal::HiddenCommander)
    );

    let mut engine = settled_main(47, their_deck());
    let card = engine.state().zones.list(ZoneLocation::Hand(THEM))[0];
    let version = engine.state().object(card).unwrap().version;
    let mut about = |version| {
        let mut engine_effects = engine.state.effects.clone();
        engine_effects.register(crate::effects::ContinuousEffect {
            id: baylee_core::ids::EffectId::new(0),
            source: None,
            controller: ME,
            origin: crate::effects::EffectOrigin::Resolution,
            layer: baylee_cards_dsl::Modifier::AddKeyword(baylee_cards_dsl::KeywordSet::HASTE)
                .layer(),
            timestamp: 1,
            duration: baylee_cards_dsl::Duration::UntilEndOfTurn,
            filter: crate::effects::EffectFilter::ObjectIs(card, version),
            modifier: baylee_cards_dsl::Modifier::AddKeyword(baylee_cards_dsl::KeywordSet::HASTE),
        });
        let kept = std::mem::replace(&mut engine.state.effects, engine_effects);
        let known = engine.fork_for(ME, DeckKnowledge::Known, 1).err();
        let on_prior = engine.fork_for(ME, DeckKnowledge::Prior(&prior), 1).err();
        engine.state.effects = kept;
        (known, on_prior)
    };
    let refusal = Some(ForkRefusal::ReferencedHiddenCard("a continuous effect"));
    assert_eq!(about(version), (refusal, refusal), "its incarnation now");
    assert_eq!(
        about(version + 1),
        (None, refusal),
        "another incarnation: mine to tell apart only where I may read it"
    );
}

/// Whether owners are asked to order cards put into a graveyard together
/// (CR 404.3) is decided at setup by every deck list: on when any holds a
/// card that reads the order, Nether Shadow for one. A fork on a prior
/// decides it again from its own cards, so a Shadow in their real list
/// turns nothing on, and one the prior deals does.
#[test]
fn graveyard_order_is_decided_by_the_cards_of_the_fork() {
    let shadow = named("Nether Shadow");
    let with = settled_main(3, deck(&[named("Swamp"), shadow]));
    let without = settled_main(3, their_deck());
    assert!(with.state().graveyard_order.enabled, "their list has it");
    assert!(!without.state().graveyard_order.enabled);
    assert!(
        fork(&with, 1).state().graveyard_order.enabled,
        "kept where I know the lists"
    );

    let prior = belief();
    for source in [&with, &without] {
        let fork = fork_on(source, &prior, 1);
        let order = &fork.state().graveyard_order;
        assert!(!order.enabled, "no card of the fork reads the order");
        assert_eq!(order.captured_through, 0);
    }
    let shadows = PublicPrior::new().with_player(THEM, vec![(shadow, 1)]);
    for source in [&with, &without] {
        let fork = fork_on(source, &shadows, 1);
        let order = &fork.state().graveyard_order;
        assert!(order.enabled, "the prior dealt a card that reads it");
        assert_eq!(order.captured_through, fork.state().journal.last_seq());
    }
    let (mut a, mut b) = (fork_on(&with, &prior, 1), fork_on(&without, &prior, 1));
    assert_forks_play_alike(&mut a, &mut b, 2, None);
}
