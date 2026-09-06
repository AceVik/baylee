//! Commander (CR 903): the command zone, casting out of it, and the cards
//! that ask a question about a commander.
//!
//! The through-line is that commander-ness belongs to the *card* and not to
//! the zone it is sitting in. Every reader in the engine got that backwards
//! in the same way — it looked in the command zone — and every one of them
//! was therefore wrong at exactly the moment it was asked, because the
//! questions ("do you control your commander", "what colour is it") are
//! only ever interesting once the commander has left that zone.

use super::testkit::*;
use super::*;
use baylee_core::ids::{CardIndex, ObjectId};
use baylee_core::mana::ManaColor;

fn forest() -> CardIndex {
    card_index("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
}
fn plains() -> CardIndex {
    card_index("bc71ebf6-2056-41f7-be35-b2e5c34afa99")
}
fn island() -> CardIndex {
    card_index("b2c6aa39-2d2a-459c-a555-fb48ba993373")
}
/// Katara, the Fearless — {G}{W}{U}, a legendary creature: three lands
/// pay for her, and her colour identity is three colours rather than one,
/// so a reader that answered "colourless" is caught rather than flattered.
fn katara() -> CardIndex {
    card_index("0972d46e-423b-454e-87c7-a2d40fb6fb6d")
}
fn arcane_signet() -> CardIndex {
    card_index("0bc7f093-bef0-4f1a-852c-4b75ebf54838")
}
fn flawless_maneuver() -> CardIndex {
    card_index("4e183439-17d2-47ff-9d99-5e22821d91e3")
}
/// Supreme Verdict — {1}{W}{U}{U}, "Destroy all creatures". The kill in
/// these tests is a card someone casts, not a call to `sba::destroy`: a
/// commander that only ever dies by the engine reaching in has never been
/// through the path a game uses.
fn supreme_verdict() -> CardIndex {
    card_index("0230de18-8d15-4cfa-9d42-7ccddd9f9570")
}
/// Swords to Plowshares — {W}, and the *exile* half of CR 903.9a. The rule
/// names two zones and a test that only ever visits one of them proves half
/// a `matches!`.
fn swords_to_plowshares() -> CardIndex {
    card_index("b1544f21-7e98-461b-aed5-e748b0168c52")
}

fn tap_all_mana(engine: &mut Engine<RegistryLookup>, seat: PlayerId) {
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    for source in legal.mana_abilities.clone() {
        engine
            .apply(seat, PlayerAction::ActivateManaAbility { source })
            .unwrap();
    }
}

/// True once `seat` has `card` on the battlefield *and* holds priority
/// again.
///
/// Both halves matter: `pass_until` stops the moment its predicate holds,
/// and a test that then reads `LegalActions` off a pending belonging to the
/// other seat is reading the wrong player's options.
fn resolved_and_back_to(engine: &Engine<RegistryLookup>, seat: PlayerId, card: CardIndex) -> bool {
    on_battlefield(engine, seat, card).is_some()
        && matches!(engine.pending(), Pending::Priority { player, .. } if *player == seat)
}

/// Taps one specific land of `seat`'s, named by the card it is.
///
/// [`tap_all_mana`] is no good where the *colours* are the point: a board of
/// Forest, Plains, Island, Forest, Island tapped in list order can produce
/// three mana that do not pay `{G}{W}{U}`, and a castability assertion would
/// then be answering a question about colour while claiming to answer one
/// about the tax.
#[track_caller]
fn tap_land(engine: &mut Engine<RegistryLookup>, seat: PlayerId, land: CardIndex) {
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let state = engine.state();
    let source = legal
        .mana_abilities
        .iter()
        .copied()
        .find(|id| {
            state
                .object(*id)
                .and_then(|o| o.card)
                .is_some_and(|c| c.index == land)
        })
        .expect("an untapped land of that kind");
    engine
        .apply(seat, PlayerAction::ActivateManaAbility { source })
        .unwrap();
}

/// The one card of `card` in `seat`'s hand.
#[track_caller]
fn in_hand(engine: &Engine<RegistryLookup>, seat: PlayerId, card: CardIndex) -> ObjectId {
    let state = engine.state();
    state
        .zones
        .list(crate::zone::ZoneLocation::Hand(seat))
        .iter()
        .copied()
        .find(|id| {
            state
                .object(*id)
                .and_then(|o| o.card)
                .is_some_and(|c| c.index == card)
        })
        .expect("the card is in hand")
}

/// True once CR 903.9a's question is the thing the game is waiting on.
fn asking_about_a_commander(engine: &Engine<RegistryLookup>) -> bool {
    matches!(
        engine.pending(),
        Pending::YesNo {
            prompt: crate::choice::YesNoPrompt::CommanderZone { .. },
            ..
        }
    )
}

/// Answers CR 903.9a's question, and returns the seat that was asked.
#[track_caller]
fn answer_commander_zone(engine: &mut Engine<RegistryLookup>, yes: bool) -> PlayerId {
    let Pending::YesNo { player, .. } = engine.pending().clone() else {
        panic!(
            "expected the command-zone question, got {:?}",
            engine.pending()
        )
    };
    engine.apply(player, PlayerAction::YesNo(yes)).unwrap();
    player
}

/// Which zone `card` is in.
#[track_caller]
fn zone_of(engine: &Engine<RegistryLookup>, card: ObjectId) -> crate::zone::Zone {
    engine.state().object(card).expect("the card exists").zone
}

/// The one commander in the command zone, by id.
#[track_caller]
fn commander_of(engine: &Engine<RegistryLookup>, seat: PlayerId) -> ObjectId {
    let zone = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Command(seat));
    assert_eq!(zone.len(), 1, "exactly one card in the command zone");
    zone[0]
}

/// Setup: a commander starts in the command zone, and nowhere else. The
/// second half is the one worth asserting — a commander shuffled into the
/// library would be a 101st card that also happens to be drawable.
#[test]
fn a_commander_starts_in_the_command_zone_and_never_in_the_library() {
    let p0 = PlayerId::new(0);
    let engine = Duel::new(3, forest()).commander(0, &[katara()]).start();

    let id = commander_of(&engine, p0);
    let marked = &engine.state().commanders[0];
    assert_eq!(marked.len(), 1, "one marked commander");
    assert_eq!(
        marked[0].object, id,
        "the marker names the card in the zone"
    );
    assert_eq!(marked[0].casts, 0, "it has not been cast yet");

    let state = engine.state();
    for zone in [
        crate::zone::ZoneLocation::Library(p0),
        crate::zone::ZoneLocation::Hand(p0),
    ] {
        assert!(
            state
                .zones
                .list(zone)
                .iter()
                .filter_map(|id| state.object(*id))
                .all(|o| o.card.is_none_or(|c| c.index != katara())),
            "the commander is not in {zone:?}"
        );
    }
}

/// Casting from the command zone (CR 903.8), and the two counters it
/// moves: the seat's (Commander's Insight) and the commander's own (the
/// tax). Nothing offered a command-zone card before this existed, so the
/// commander was a card you could look at and never play.
#[test]
fn a_commander_can_be_cast_out_of_the_command_zone() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(4, forest())
        .commander(0, &[katara()])
        .battlefield(0, &[forest(), plains(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let card = commander_of(&engine, p0);
    // The mana first: `can_cast` filters by what the pool covers, so an
    // untapped board offers nothing and would prove nothing either way.
    tap_all_mana(&mut engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&card),
        "the commander is offered from the command zone"
    );

    engine.apply(p0, PlayerAction::CastSpell { card }).unwrap();
    pass_until(&mut engine, |e| resolved_and_back_to(e, p0, katara()));

    assert_eq!(
        engine.state().commander_casts[0],
        1,
        "the seat's count, which Commander's Insight reads"
    );
    assert_eq!(
        engine.state().commanders[0][0].casts,
        1,
        "and the commander's own count, which the tax reads"
    );
}

/// Arcane Signet: "Add one mana of any color in your commander's color
/// identity." The identity is a property of the commander card wherever it
/// is (CR 903.4) — and reading the command *zone* meant the Signet went
/// colourless the instant the commander was cast, which is the only state
/// of the game a Commander deck spends any time in.
#[test]
fn arcane_signet_still_reads_the_commander_once_it_has_been_cast() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(5, forest())
        .commander(0, &[katara()])
        .battlefield(0, &[forest(), plains(), island(), arcane_signet()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let card = commander_of(&engine, p0);
    let signet = on_battlefield(&engine, p0, arcane_signet()).expect("signet deployed");

    // `tap_all_mana` takes only `legal.mana_abilities`, which is the CR
    // 305.6 intrinsic-land shortcut — the Signet's printed ability is an
    // ordinary activated one and is untouched by it.
    tap_all_mana(&mut engine, p0);
    engine.apply(p0, PlayerAction::CastSpell { card }).unwrap();
    pass_until(&mut engine, |e| resolved_and_back_to(e, p0, katara()));

    // Katara is on the battlefield now, and the command zone is empty.
    assert!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Command(p0))
            .is_empty(),
        "the commander left the zone the old lookup was reading"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(id, _)| *id == signet)
        .expect("the Signet offers its mana ability");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .unwrap();
    let Pending::ChooseColor { options, .. } = engine.pending().clone() else {
        panic!(
            "expected a colour choice, got {:?} — a Signet offering no choice \
             is one that found no commander",
            engine.pending()
        )
    };
    assert_eq!(
        options,
        vec![ManaColor::White, ManaColor::Blue, ManaColor::Green],
        "Katara's colour identity, in colour order"
    );
}

/// Flawless Maneuver: "If you control a commander, you may cast this spell
/// without paying its mana cost." The condition read the command zone, so
/// it was false in every game ever played — the card has been a {2}{W}
/// instant with a line of text nothing consulted.
#[test]
fn flawless_maneuver_is_free_only_once_the_commander_is_on_the_battlefield() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(6, forest())
        .commander(0, &[katara()])
        .battlefield(0, &[forest(), plains(), island()])
        .hand(0, &[flawless_maneuver()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let state = engine.state();
    let maneuver = state
        .zones
        .list(crate::zone::ZoneLocation::Hand(p0))
        .iter()
        .copied()
        .find(|id| {
            state
                .object(*id)
                .and_then(|o| o.card)
                .is_some_and(|c| c.index == flawless_maneuver())
        })
        .expect("the maneuver is in hand");

    // Nothing tapped yet, so {2}{W} is out of reach and the free clause is
    // the only other reading — and its condition is false while the
    // commander is still in the command zone.
    let card = commander_of(&engine, p0);
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&maneuver),
        "no mana and no commander on the battlefield: nothing pays for it"
    );

    tap_all_mana(&mut engine, p0);
    engine.apply(p0, PlayerAction::CastSpell { card }).unwrap();
    pass_until(&mut engine, |e| resolved_and_back_to(e, p0, katara()));

    // The pool is empty — the three lands paid for Katara — so the only
    // reading under which this is castable is the free one.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&maneuver),
        "with the commander out it is free"
    );
}

/// A predicate for "seat `p` holds priority in their own first main phase".
fn own_main_phase(engine: &Engine<RegistryLookup>, p: PlayerId) -> bool {
    engine.state().turn.active == p
        && matches!(engine.state().turn.phase, Phase::FirstMain)
        && matches!(engine.pending(), Pending::Priority { player, .. } if *player == p)
}

/// CR 903.9a and CR 903.8 in one game, because neither means much without
/// the other: a commander that dies is *offered* the command zone, and
/// coming back is what makes the next cast cost `{2}` more.
///
/// The tax is asserted through castability rather than through the counter.
/// `casts == 2` proves a number moved; it does not prove the cost
/// computation ever read it, and a tax nothing charges is not a tax. So the
/// second cast is offered exactly the mana that paid for the first — three
/// lands, in the three colours the cost names — and must be refused.
#[test]
fn a_commander_returns_to_the_command_zone_and_costs_two_more_next_time() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(7, forest())
        .commander(0, &[katara()])
        .battlefield(0, &[forest(), plains(), island(), forest(), island()])
        .battlefield(1, &[plains(), island(), island(), island()])
        .hand(1, &[supreme_verdict()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // First cast: the printed {G}{W}{U} and nothing more.
    let card = commander_of(&engine, p0);
    for land in [forest(), plains(), island()] {
        tap_land(&mut engine, p0, land);
    }
    engine.apply(p0, PlayerAction::CastSpell { card }).unwrap();
    pass_until(&mut engine, |e| resolved_and_back_to(e, p0, katara()));
    assert_eq!(engine.state().commanders[0][0].casts, 1);

    // Seat 1 wraths the board.
    pass_until(&mut engine, |e| own_main_phase(e, p1));
    let verdict = in_hand(&engine, p1, supreme_verdict());
    tap_all_mana(&mut engine, p1);
    engine
        .apply(p1, PlayerAction::CastSpell { card: verdict })
        .unwrap();

    // The question — who is asked, about what, and under which handle.
    pass_until(&mut engine, asking_about_a_commander);
    let Pending::YesNo {
        player,
        prompt,
        source,
    } = engine.pending().clone()
    else {
        unreachable!("the predicate just matched")
    };
    assert_eq!(player, p0, "the owner is asked, not the wrath's caster");
    assert_eq!(
        prompt,
        crate::choice::YesNoPrompt::CommanderZone { card },
        "the id survives the zone change (CR 400.7), so it names the same card"
    );
    assert_eq!(
        source,
        Some(baylee_core::ids::AbilityRef::new(
            katara(),
            baylee_core::ids::AbilityRef::COMMANDER_ZONE
        )),
        "a standing answer is filed under this commander, not under all of them"
    );
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    assert_eq!(zone_of(&engine, card), crate::zone::Zone::Command);

    // Seat 0's next turn. Five lands untap; the three that paid last time
    // are no longer enough, and two more is exactly the difference.
    pass_until(&mut engine, |e| own_main_phase(e, p0));
    for land in [forest(), plains(), island()] {
        tap_land(&mut engine, p0, land);
    }
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&card),
        "{{G}}{{W}}{{U}} paid the first cast and must not pay the second (CR 903.8)"
    );
    for land in [forest(), island()] {
        tap_land(&mut engine, p0, land);
    }
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&card),
        "with {{2}} more on top of the printed cost it is castable again"
    );
    engine.apply(p0, PlayerAction::CastSpell { card }).unwrap();
    pass_until(&mut engine, |e| resolved_and_back_to(e, p0, katara()));
    assert_eq!(engine.state().commanders[0][0].casts, 2);
}

/// "Its owner *may*" (CR 903.9a): no is an answer, and it is an answer that
/// sticks. The question is asked once per arrival, so a commander left in
/// the graveyard stays there instead of asking again at every state-based
/// check — which, running before every priority grant, would be a game that
/// never advances past its own prompt.
///
/// `pass_until` is half the assertion: it panics on any pending it does not
/// know how to answer, so a second command-zone question fails this test
/// rather than being quietly answered.
#[test]
fn a_declined_commander_stays_where_it_died_and_is_not_asked_again() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(8, forest())
        .commander(0, &[katara()])
        .battlefield(0, &[forest(), plains(), island()])
        .battlefield(1, &[plains(), island(), island(), island()])
        .hand(1, &[supreme_verdict()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let card = commander_of(&engine, p0);
    tap_all_mana(&mut engine, p0);
    engine.apply(p0, PlayerAction::CastSpell { card }).unwrap();
    pass_until(&mut engine, |e| resolved_and_back_to(e, p0, katara()));

    pass_until(&mut engine, |e| own_main_phase(e, p1));
    let verdict = in_hand(&engine, p1, supreme_verdict());
    tap_all_mana(&mut engine, p1);
    engine
        .apply(p1, PlayerAction::CastSpell { card: verdict })
        .unwrap();

    pass_until(&mut engine, asking_about_a_commander);
    assert_eq!(answer_commander_zone(&mut engine, false), p0);
    assert_eq!(zone_of(&engine, card), crate::zone::Zone::Graveyard);

    // A whole turn's worth of state-based checks later, still no second ask.
    pass_until(&mut engine, |e| own_main_phase(e, p0));
    assert_eq!(
        zone_of(&engine, card),
        crate::zone::Zone::Graveyard,
        "declining leaves it where it is"
    );
    assert_eq!(
        engine.state().commanders[0][0].casts,
        1,
        "and the tax has not moved: it counts casts, not deaths"
    );
}

/// Exile is the other half of CR 903.9a's sentence, and a `matches!` with
/// two arms is a test with two cases. Swords to Plowshares does it for {W}.
#[test]
fn a_commander_exiled_rather_than_killed_is_offered_the_same_way() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(9, forest())
        .commander(0, &[katara()])
        .battlefield(0, &[forest(), plains(), island()])
        .battlefield(1, &[plains()])
        .hand(1, &[swords_to_plowshares()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let card = commander_of(&engine, p0);
    tap_all_mana(&mut engine, p0);
    engine.apply(p0, PlayerAction::CastSpell { card }).unwrap();
    pass_until(&mut engine, |e| resolved_and_back_to(e, p0, katara()));
    let on_table = on_battlefield(&engine, p0, katara()).expect("she resolved");

    pass_until(&mut engine, |e| own_main_phase(e, p1));
    let swords = in_hand(&engine, p1, swords_to_plowshares());
    tap_all_mana(&mut engine, p1);
    engine
        .apply(p1, PlayerAction::CastSpell { card: swords })
        .unwrap();
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!("expected a target choice, got {:?}", engine.pending())
    };
    assert!(
        options.contains(&on_table),
        "the commander is a legal target"
    );
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![on_table],
            },
        )
        .unwrap();

    pass_until(&mut engine, asking_about_a_commander);
    assert_eq!(answer_commander_zone(&mut engine, true), p0);
    assert_eq!(zone_of(&engine, card), crate::zone::Zone::Command);
}

/// One wrath, two commanders, two questions — which is the whole reason
/// CR 903.9a's "since the last time state-based actions were checked" is
/// tracked per commander here and not as one watermark on the game. Both
/// arrive in the same pass; a single watermark moved by the first question
/// makes the second commander look like it has already been offered, and
/// its owner is never asked. That failure is silent, so it needs a test
/// that would notice: `pass_until` panics if the second question never
/// comes.
#[test]
fn one_wrath_that_kills_two_commanders_asks_both_owners() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(10, forest())
        .commander(0, &[katara()])
        .commander(1, &[katara()])
        .battlefield(0, &[forest(), plains(), island()])
        .battlefield(
            1,
            &[
                forest(),
                plains(),
                island(),
                plains(),
                island(),
                island(),
                island(),
            ],
        )
        .hand(1, &[supreme_verdict()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let first = commander_of(&engine, p0);
    tap_all_mana(&mut engine, p0);
    engine
        .apply(p0, PlayerAction::CastSpell { card: first })
        .unwrap();
    pass_until(&mut engine, |e| resolved_and_back_to(e, p0, katara()));

    // Seat 1 deploys their own and then wraths both: seven lands pay
    // {G}{W}{U} and {1}{W}{U}{U} in one main phase, and the pool does not
    // empty between two spells in the same step.
    pass_until(&mut engine, |e| own_main_phase(e, p1));
    let second = commander_of(&engine, p1);
    let verdict = in_hand(&engine, p1, supreme_verdict());
    tap_all_mana(&mut engine, p1);
    engine
        .apply(p1, PlayerAction::CastSpell { card: second })
        .unwrap();
    pass_until(&mut engine, |e| resolved_and_back_to(e, p1, katara()));
    engine
        .apply(p1, PlayerAction::CastSpell { card: verdict })
        .unwrap();

    // Seat order decides who is asked first, every replay.
    pass_until(&mut engine, asking_about_a_commander);
    assert_eq!(answer_commander_zone(&mut engine, true), p0);
    pass_until(&mut engine, asking_about_a_commander);
    assert_eq!(
        answer_commander_zone(&mut engine, true),
        p1,
        "the second commander's owner is asked too"
    );

    assert_eq!(zone_of(&engine, first), crate::zone::Zone::Command);
    assert_eq!(zone_of(&engine, second), crate::zone::Zone::Command);
}

/// [`pass_until`], plus the two questions a game long enough to land
/// twenty-one commander damage will ask on the way.
///
/// The shared helper panics on anything it does not recognise, which is
/// what makes it a good default and a bad fit here: eleven turns of drawing
/// without playing lands overflows a hand, and the cleanup discard
/// (CR 514.1) is not a rules event this test is about.
#[track_caller]
fn run_along(engine: &mut Engine<RegistryLookup>, pred: impl Fn(&Engine<RegistryLookup>) -> bool) {
    for _ in 0..200 {
        if pred(engine) {
            return;
        }
        match engine.pending().clone() {
            Pending::DiscardChoice { player, count } => {
                let hand: Vec<ObjectId> = engine
                    .state()
                    .zones
                    .list(crate::zone::ZoneLocation::Hand(player))
                    .iter()
                    .copied()
                    .take(count as usize)
                    .collect();
                engine
                    .apply(player, PlayerAction::ChooseObjects { objects: hand })
                    .unwrap();
            }
            // Everything else is what `pass_until` does, written out here
            // rather than delegated: that helper runs its own loop to its
            // own predicate, so a discard raised inside it never comes back
            // out to the arm above.
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseAttackers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
                    .unwrap();
            }
            Pending::ChooseBlockers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
                    .unwrap();
            }
            other => panic!("unexpected while running along: {other:?}"),
        }
    }
    panic!("condition never reached");
}

/// CR 903.10a: twenty-one combat damage from one commander and the player
/// loses, whatever their life total says.
///
/// Which is the whole assertion, and why the defender sits at forty life: a
/// 2/2 swinging eleven times deals twenty-two, and at twenty life the
/// player would have died of the damage long before the rule under test
/// ever fired. `LossReason::CommanderDamage` is therefore not decoration —
/// it is how this test tells the two rules apart.
#[test]
fn twenty_one_damage_from_one_commander_ends_it_at_any_life_total() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(11, forest())
        .commander(0, &[katara()])
        .battlefield(0, &[forest(), plains(), island()])
        .life(1, 40)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let card = commander_of(&engine, p0);
    tap_all_mana(&mut engine, p0);
    engine.apply(p0, PlayerAction::CastSpell { card }).unwrap();
    pass_until(&mut engine, |e| resolved_and_back_to(e, p0, katara()));

    // Eleven swings of a 2/2. She is summoning sick this turn, so the first
    // declaration is next turn's.
    for _ in 0..12 {
        if matches!(engine.pending(), Pending::GameOver(_)) {
            break;
        }
        run_along(&mut engine, |e| {
            matches!(e.pending(), Pending::GameOver(_))
                || matches!(e.pending(), Pending::ChooseAttackers { player, .. } if *player == p0)
        });
        if matches!(engine.pending(), Pending::GameOver(_)) {
            break;
        }
        let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
            unreachable!("the predicate just matched")
        };
        // Only declare what the engine offers: an attack this test invents
        // is an attack no player could have made.
        if attackers.contains(&card) {
            engine
                .apply(
                    p0,
                    PlayerAction::DeclareAttackers {
                        attackers: vec![(card, baylee_core::ids::Defender::Player(p1))],
                    },
                )
                .unwrap();
        } else {
            engine
                .apply(p0, PlayerAction::DeclareAttackers { attackers: vec![] })
                .unwrap();
        }
        run_along(&mut engine, |e| {
            matches!(e.pending(), Pending::GameOver(_)) || own_main_phase(e, p0)
        });
    }

    let tally = &engine.state().players[1].commander_damage;
    assert_eq!(tally.len(), 1, "one commander, one tally");
    assert_eq!(tally[0].0, card, "and it is keyed by the commander itself");
    assert!(
        tally[0].1 >= 21,
        "eleven swings of a 2/2 is twenty-two: {}",
        tally[0].1
    );
    assert!(
        engine.state().players[1].life > 0,
        "the point of forty life: they lose while still alive on the counter"
    );
    let Pending::GameOver(result) = engine.pending().clone() else {
        panic!("expected the game to be over, got {:?}", engine.pending())
    };
    assert_eq!(result.winner, Some(crate::win::Victor::Player(p0)));
    assert!(
        engine.journal().entries().iter().any(|e| matches!(
            e.event,
            crate::event::GameEvent::PlayerLost {
                player,
                reason: crate::event::LossReason::CommanderDamage
            } if player == p1
        )),
        "and the log says which rule did it"
    );
}
