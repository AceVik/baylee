//! Saga tests (CR 714): lore counters as the Saga enters and as its
//! controller's precombat main phase begins, chapter triggers, sacrifice
//! after the final chapter.
//!
//! "After your draw step" is the reminder text a Saga is printed with and is
//! where this module's old wording came from. The rule is CR 714.3b and
//! CR 505.4 — a turn-based action of the precombat main phase, before
//! anybody has priority in it — and the engine used to do it a whole phase
//! later still, as combat began.

use super::*;
use baylee_core::ids::{CardIndex, PrintRef};
use baylee_core::preset::{
    AIProfile, DeckEntry, Finish, FormatId, GamePreset, PrintInfo, SeatController, SeatSpec,
};

struct RegistryLookup;
impl CardLookup for RegistryLookup {
    fn card(&self, index: CardIndex) -> Option<&'static baylee_cards_dsl::CardDef> {
        baylee_cards::by_index(index)
    }
}

fn card_index(oracle_id: &str) -> CardIndex {
    baylee_cards::by_oracle_id(oracle_id)
        .expect("card exists")
        .index
}

fn island() -> CardIndex {
    card_index("b2c6aa39-2d2a-459c-a555-fb48ba993373")
}
fn urzas_saga() -> CardIndex {
    card_index("4c6a0c30-b547-4eff-8ff4-0ca25803c076")
}

fn entry(card: CardIndex) -> DeckEntry {
    DeckEntry {
        card,
        print: PrintRef::new(0),
    }
}

fn preset(seed: u64, hand0: Vec<CardIndex>) -> GamePreset {
    let deck: Vec<DeckEntry> = (0..60).map(|_| entry(island())).collect();
    let mk = |hand: Vec<CardIndex>| SeatSpec {
        controller: SeatController::Ai(AIProfile::default()),
        capabilities: baylee_core::preset::SeatCapabilities::default(),
        deck: deck.clone(),
        sideboard: vec![],
        commanders: vec![],
        starting_life: None,
        starting_hand: Some(hand.into_iter().map(entry).collect()),
        starting_battlefield: vec![],
        emblems: vec![],
        team: None,
    };
    GamePreset {
        format: FormatId::Freeform,
        seed,
        house_rules: HouseRules::default(),
        modifiers: vec![],
        prints: vec![PrintInfo {
            scryfall_id: uuid::Uuid::nil(),
            lang: "EN".into(),
            finish: Finish::Normal,
        }],
        seats: vec![mk(hand0), mk(vec![])],
    }
}

fn keep_mulligans(engine: &mut Engine<RegistryLookup>) {
    for _ in 0..2 {
        match engine.pending().clone() {
            Pending::Mulligan { player, .. } => {
                engine.apply(player, PlayerAction::MulliganKeep).unwrap();
            }
            other => panic!("expected mulligan, got {other:?}"),
        }
    }
}

/// Passes/answers everything until p0 has priority with the saga in
/// `legal.lands`, then plays it.
fn drive_and_play_saga(engine: &mut Engine<RegistryLookup>, p0: PlayerId) {
    let mut guard = 0;
    loop {
        guard += 1;
        assert!(guard < 400, "no land-play window");
        match engine.pending().clone() {
            Pending::Priority { player, legal } if player == p0 => {
                if let Some(&card) = legal.lands.iter().find(|id| {
                    engine.state().object(**id).unwrap().card.unwrap().index == urzas_saga()
                }) {
                    engine
                        .apply(player, PlayerAction::PlayLand { card })
                        .unwrap();
                    return;
                }
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
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
            Pending::DiscardChoice { player, count } => {
                let hand: Vec<_> = engine
                    .state()
                    .zones
                    .list(crate::zone::ZoneLocation::Hand(player))
                    .clone();
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: hand[..count as usize].to_vec(),
                        },
                    )
                    .unwrap();
            }
            Pending::YesNo { player, .. } => {
                engine.apply(player, PlayerAction::YesNo(false)).unwrap();
            }
            other => panic!("unexpected pending: {other:?}"),
        }
    }
}

/// Passes/answers everything for `steps` pending choices.
fn drive(engine: &mut Engine<RegistryLookup>, steps: usize) {
    for _ in 0..steps {
        match engine.pending().clone() {
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
            Pending::DiscardChoice { player, count } => {
                let hand: Vec<_> = engine
                    .state()
                    .zones
                    .list(crate::zone::ZoneLocation::Hand(player))
                    .clone();
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: hand[..count as usize].to_vec(),
                        },
                    )
                    .unwrap();
            }
            Pending::ChooseCards { player, .. } => {
                engine
                    .apply(player, PlayerAction::ChooseObjects { objects: vec![] })
                    .unwrap();
            }
            Pending::YesNo { player, .. } => {
                engine.apply(player, PlayerAction::YesNo(false)).unwrap();
            }
            other => panic!("unexpected pending: {other:?}"),
        }
    }
}

fn saga_object(engine: &Engine<RegistryLookup>) -> Option<ObjectId> {
    engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Battlefield)
        .iter()
        .copied()
        .find(|id| {
            engine
                .state()
                .object(*id)
                .and_then(|o| o.card)
                .is_some_and(|c| c.index == urzas_saga())
        })
}

#[test]
fn saga_ticks_through_chapters_and_sacrifices_after_final() {
    let mut engine = Engine::new(&preset(3, vec![urzas_saga()]), RegistryLookup).unwrap();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);
    drive_and_play_saga(&mut engine, p0);
    // ETB: lore counter 1 + chapter I on the stack.
    let saga = saga_object(&engine).expect("saga on the battlefield");
    assert_eq!(
        engine
            .state()
            .object(saga)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::CounterKind::Lore),
        1
    );
    // Let chapter I resolve, then drive on through p0's later turns — each
    // of their precombat main phases begins by adding a lore counter, and
    // chapter III ends the saga.
    drive(&mut engine, 200);
    // The saga is played *during* a main phase, so it takes no turn-based
    // counter that turn: lore 2 at the start of p0's next precombat main
    // and 3 at the one after, then the saga is sacrificed (counters >= the
    // final chapter once III has resolved).
    let still_there = saga_object(&engine);
    let in_graveyard = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Graveyard(p0))
        .iter()
        .any(|id| {
            engine
                .state()
                .object(*id)
                .and_then(|o| o.card)
                .is_some_and(|c| c.index == urzas_saga())
        });
    assert!(
        still_there.is_none() && in_graveyard,
        "saga sacrificed after chapter III (battlefield: {:?}, graveyard: {in_graveyard})",
        still_there.is_some()
    );
}

/// Urza's Saga grants *itself* two abilities — chapter I's `{T}: Add {C}` and
/// chapter II's `{2}, {T}: Create a Construct` — and for as long as a
/// permanent had one synthetic slot the second was never offered. The card
/// said `Coverage::Implemented` and half of it could not be played.
///
/// The sharp part is the second half: activating slot 1 has to *run* slot 1.
/// An engine that decoded the index differently from how the offer encoded it
/// would tap the saga for `{C}` while the player was buying a Construct, and
/// nothing about the action would look wrong.
#[test]
fn a_saga_granted_two_abilities_offers_and_runs_both() {
    use crate::choice::granted_ability;

    let mut preset = preset(3, vec![urzas_saga()]);
    // Chapter II costs `{2}`, so there has to be something to pay it with.
    preset.seats[0].starting_battlefield = vec![entry(island()), entry(island())];
    let mut engine = Engine::new(&preset, RegistryLookup).unwrap();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);
    drive_and_play_saga(&mut engine, p0);
    let saga = saga_object(&engine).expect("saga on the battlefield");

    // Chapter II resolves on the second lore counter, which is p0's next
    // draw step; chapter III sacrifices the saga on the third, so this stops
    // in between rather than driving a fixed number of steps.
    let mut guard = 0;
    loop {
        guard += 1;
        assert!(guard < 400, "chapter II never resolved");
        let lore = engine
            .state()
            .object(saga)
            .expect("the saga is still on the battlefield")
            .counters
            .get(baylee_cards_dsl::CounterKind::Lore);
        if lore == 2
            && let Pending::Priority { player, .. } = engine.pending()
            && *player == p0
            && engine.state().zones.stack_is_empty()
        {
            break;
        }
        drive(&mut engine, 1);
    }

    let islands: Vec<_> = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            engine
                .state()
                .object(*id)
                .and_then(|o| o.card)
                .is_some_and(|c| c.index == island())
        })
        .collect();
    assert_eq!(islands.len(), 2, "two lands to pay chapter II with");
    for source in islands {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source })
            .expect("an Island taps for mana");
    }

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority")
    };
    assert!(
        legal.abilities.contains(&(saga, granted_ability(0))),
        "chapter I's mana ability"
    );
    assert!(
        legal.abilities.contains(&(saga, granted_ability(1))),
        "chapter II's Construct ability — the one that used to have no slot"
    );

    let before = engine.state().players[0].mana_pool.total();
    assert_eq!(before, 2, "both Islands are in the pool");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: saga,
                ability_index: granted_ability(1),
            },
        )
        .expect("chapter II's ability is activatable");

    assert!(
        engine
            .state()
            .object(saga)
            .expect("the saga survives its own ability")
            .status
            .contains(Status::TAPPED),
        "the ability costs `{{T}}`"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the `{{2}}` was spent — an engine that ran chapter I instead would \
         have paid nothing and added a colorless"
    );
}

fn doubling_season() -> CardIndex {
    card_index("01546b7d-a233-4176-8843-d732074dc5b6")
}

/// Continuous effects the Saga is the source of.
///
/// Every chapter Urza's Saga has grants it an ability, so this is "how
/// many chapters have resolved" without asking the trigger queue — and
/// unlike the offer below it does not depend on the mana to pay for one.
fn grants_from(engine: &Engine<RegistryLookup>, saga: ObjectId) -> usize {
    engine
        .state()
        .effects
        .iter()
        .filter(|fx| fx.source == Some(saga))
        .count()
}

fn lore(engine: &Engine<RegistryLookup>, saga: ObjectId) -> u16 {
    engine
        .state()
        .object(saga)
        .expect("the saga is on the battlefield")
        .counters
        .get(baylee_cards_dsl::CounterKind::Lore)
}

/// Drives until p0 holds priority in their own precombat main phase on a
/// turn later than `after`, and stops **without** passing it.
///
/// Stopping there is the whole point: the lore counter of CR 505.4 is placed
/// before anybody has priority (CR 505.6), so the first offer p0 is made in
/// that phase is where the count can be read without having agreed to
/// anything first.
fn reach_p0_precombat_main(engine: &mut Engine<RegistryLookup>, p0: PlayerId, after: u32) {
    let mut guard = 0;
    loop {
        guard += 1;
        assert!(guard < 400, "no precombat main window");
        let turn = engine.state().turn;
        if turn.active == p0
            && turn.phase == Phase::FirstMain
            && turn.number > after
            && matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0)
        {
            return;
        }
        drive(engine, 1);
    }
}

/// CR 714.3 puts a Saga's two lore counters on opposite sides of CR 614.16,
/// and this is the board that reads both halves at once.
///
/// The one it takes **as it enters** (CR 714.3a) is a replacement effect
/// (CR 614.1c), so a Doubling Season doubles it: the Saga arrives on two
/// counters and owes chapters I *and* II, which is CR 714.2b's window — "was
/// less than N and became at least N" — rather than "the next chapter".
///
/// The one it takes **as the precombat main phase begins** (CR 714.3b,
/// CR 505.4) is a turn-based action, which is neither of the two things
/// CR 614.16 names, so the same enchantment does not touch it: the count
/// goes to three and not four. That number is the whole test in one
/// assertion — four says the turn-based path went through the doubling door,
/// two says the counter is still landing a phase late.
#[test]
fn a_saga_under_a_doubling_season_enters_on_two_and_still_ticks_by_one() {
    use crate::choice::granted_ability;

    let mut preset = preset(5, vec![urzas_saga()]);
    // Chapter II's granted ability costs `{2}`, and an ability nobody can
    // pay for is not offered — so without the Islands the second half of
    // "both chapters ran" could not be seen at all.
    preset.seats[0].starting_battlefield =
        vec![entry(doubling_season()), entry(island()), entry(island())];
    let mut engine = Engine::new(&preset, RegistryLookup).unwrap();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);
    drive_and_play_saga(&mut engine, p0);

    let saga = saga_object(&engine).expect("saga on the battlefield");
    assert_eq!(lore(&engine, saga), 2, "one counter, put twice");

    // Both chapters, not just the first. The board says so rather than the
    // trigger queue: chapter I grants the Saga `{T}: Add {C}` and chapter II
    // grants it the Construct ability, so two granted slots offered is two
    // chapter abilities having resolved.
    //
    // Driven to the *grants* and not to an idle stack, because a played land
    // hands priority back before `collect_triggers` has run: the stack is
    // briefly empty with both chapters still owed, and a test that stopped
    // there would read a board nothing had happened to yet.
    let mut guard = 0;
    while grants_from(&engine, saga) < 2 {
        guard += 1;
        assert!(guard < 60, "chapters I and II did not both resolve");
        drive(&mut engine, 1);
    }
    let mut guard = 0;
    while !(engine.state().zones.stack_is_empty()
        && matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0))
    {
        guard += 1;
        assert!(guard < 40, "the two chapters never finished resolving");
        drive(&mut engine, 1);
    }
    let islands: Vec<_> = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            engine
                .state()
                .object(*id)
                .and_then(|o| o.card)
                .is_some_and(|c| c.index == island())
        })
        .collect();
    for source in islands {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source })
            .expect("an Island taps for mana");
    }
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority")
    };
    assert!(
        legal.abilities.contains(&(saga, granted_ability(0)))
            && legal.abilities.contains(&(saga, granted_ability(1))),
        "chapters I and II both resolved on the way in: {:?}",
        legal.abilities
    );

    // The turn-based half, read at the first offer of p0's next precombat
    // main phase — before anything could have been done about it.
    reach_p0_precombat_main(&mut engine, p0, 1);
    let saga = saga_object(&engine).expect("the saga is still on the battlefield");
    assert_eq!(
        lore(&engine, saga),
        3,
        "a turn-based action is not an effect, so the Season does not see it"
    );

    // And chapter III resolves, and CR 714.4 sacrifices the Saga, inside the
    // phase whose beginning placed the counter. That is what the move buys:
    // all of it used to happen as combat began, a whole phase after the
    // rules put it.
    let mut guard = 0;
    while saga_object(&engine).is_some() {
        guard += 1;
        assert!(guard < 40, "the saga was never sacrificed");
        assert_eq!(
            engine.state().turn.phase,
            Phase::FirstMain,
            "chapter III belongs to the precombat main phase, not to combat"
        );
        drive(&mut engine, 1);
    }
}

/// Chapter abilities of `saga` that have triggered and are still on the
/// stack.
///
/// Everything Urza's Saga puts on the stack is a chapter — the two abilities
/// it grants are activated and are activated by nobody here — so this counts
/// what the source owns rather than decoding each index.
fn chapters_on_the_stack(engine: &Engine<RegistryLookup>, saga: ObjectId) -> usize {
    engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Stack)
        .iter()
        .filter(|id| {
            engine
                .state()
                .object(**id)
                .and_then(|o| o.ability)
                .is_some_and(|loc| loc.source == saga)
        })
        .count()
}

/// CR 714.4 has a second clause, and until a Saga could owe more than one
/// chapter at a time nothing could reach it: the Saga is sacrificed only
/// when it "isn't the source of a chapter ability that has triggered but not
/// yet left the stack".
///
/// Two Doubling Seasons put four lore counters on Urza's Saga as it enters
/// (the card is not legendary, so this is an ordinary board), and all three
/// chapters are owed at once. The stack is last-in-first-out, so **III
/// resolves first** — and a sacrifice check that asked only "are the lore
/// counters at the final chapter" would answer yes there and put the Saga
/// into the graveyard with chapters I and II still waiting to resolve on a
/// permanent that had left.
///
/// The assertion is the invariant rather than a step count: at no point is
/// the Saga off the battlefield while a chapter of it is still on the stack.
#[test]
fn a_saga_owing_three_chapters_at_once_outlives_the_first_one_to_resolve() {
    let mut preset = preset(7, vec![urzas_saga()]);
    preset.seats[0].starting_battlefield = vec![entry(doubling_season()), entry(doubling_season())];
    let mut engine = Engine::new(&preset, RegistryLookup).unwrap();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);
    drive_and_play_saga(&mut engine, p0);

    let saga = saga_object(&engine).expect("saga on the battlefield");
    assert_eq!(lore(&engine, saga), 4, "one counter, doubled twice");

    let mut deepest = 0;
    let mut guard = 0;
    loop {
        guard += 1;
        assert!(guard < 80, "the saga was never sacrificed");
        let waiting = chapters_on_the_stack(&engine, saga);
        deepest = deepest.max(waiting);
        if saga_object(&engine).is_none() {
            assert_eq!(
                waiting, 0,
                "CR 714.4: the saga was sacrificed with {waiting} of its own \
                 chapters still on the stack"
            );
            break;
        }
        drive(&mut engine, 1);
    }
    assert_eq!(
        deepest, 3,
        "all three chapters were owed at once — a lower number means the \
         board never reached the state this test is about"
    );
}

fn tishanas_tidebinder() -> CardIndex {
    card_index("2993dc7d-723d-4a9b-94bd-4bb02a9f7243")
}

/// The chapter abilities of `saga` sitting on the stack.
fn chapter_ids(engine: &Engine<RegistryLookup>, saga: ObjectId) -> Vec<ObjectId> {
    engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Stack)
        .iter()
        .copied()
        .filter(|id| {
            engine
                .state()
                .object(*id)
                .and_then(|o| o.ability)
                .is_some_and(|loc| loc.source == saga)
        })
        .collect()
}

/// Drives to the one window in which chapter III can be answered: on the
/// stack, alone, with p0 holding priority. Answers with `(saga, chapter)`.
fn reach_a_lone_chapter_three(
    engine: &mut Engine<RegistryLookup>,
    p0: PlayerId,
) -> (ObjectId, ObjectId) {
    let mut guard = 0;
    loop {
        guard += 1;
        assert!(guard < 300, "chapter III never reached the stack");
        let saga = saga_object(engine).expect("the saga is on the battlefield");
        let chapters = chapter_ids(engine, saga);
        if lore(engine, saga) == 3
            && chapters.len() == 1
            && matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0)
        {
            return (saga, chapters[0]);
        }
        drive(engine, 1);
    }
}

/// Flashes in Tishana's Tidebinder off three Islands and points its trigger
/// at `chapter`, leaving the counter on the stack.
fn flash_in_a_tidebinder(engine: &mut Engine<RegistryLookup>, p0: PlayerId, chapter: ObjectId) {
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    for source in legal.mana_abilities {
        if engine
            .state()
            .object(source)
            .and_then(|o| o.card)
            .is_some_and(|c| c.index == island())
        {
            engine
                .apply(p0, PlayerAction::ActivateManaAbility { source })
                .expect("an Island taps for {U}");
        }
    }
    let tidebinder = engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Hand(p0))
        .iter()
        .copied()
        .find(|id| {
            engine
                .state()
                .object(*id)
                .and_then(|o| o.card)
                .is_some_and(|c| c.index == tishanas_tidebinder())
        })
        .expect("the tidebinder is in hand");
    engine
        .apply(p0, PlayerAction::CastSpell { card: tidebinder })
        .expect("flash, in response to the chapter");

    let mut guard = 0;
    loop {
        guard += 1;
        assert!(guard < 40, "the tidebinder never asked for a target");
        match engine.pending().clone() {
            Pending::ChooseTargets {
                player, options, ..
            } => {
                assert!(
                    options.contains(&chapter),
                    "chapter III was not offered as a target: {options:?}"
                );
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseTargets {
                            objects: vec![chapter],
                            players: vec![],
                        },
                    )
                    .expect("an ability on the stack is a legal target");
                return;
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected pending: {other:?}"),
        }
    }
}

/// CR 714.4 is a **state-based action**, and a chapter ability can leave the
/// stack without resolving.
///
/// Tishana's Tidebinder is flashed in in response to chapter III and counters
/// it. The Saga is then sitting at three lore counters with nothing of its
/// own on the stack, which is CR 714.4's sentence word for word — the number
/// of lore counters is greater than or equal to the final chapter number and
/// it isn't the source of a chapter ability that has triggered but not yet
/// left the stack — so it is sacrificed, and no chapter ever resolved to do
/// it.
///
/// The sacrifice used to live in `finish_resolution`, which only runs for an
/// ability that *resolved*, so a countered last chapter left the Saga on the
/// battlefield for the rest of the game: a permanent the rules say is not
/// there, tapping for mana and holding the abilities its earlier chapters
/// granted it. That is the difference between a rule and a step of
/// resolution.
#[test]
fn a_saga_whose_last_chapter_is_countered_is_sacrificed_anyway() {
    let mut preset = preset(11, vec![urzas_saga(), tishanas_tidebinder()]);
    preset.seats[0].starting_battlefield = vec![entry(island()), entry(island()), entry(island())];
    let mut engine = Engine::new(&preset, RegistryLookup).unwrap();
    keep_mulligans(&mut engine);
    let p0 = PlayerId::new(0);
    drive_and_play_saga(&mut engine, p0);

    let (saga, chapter) = reach_a_lone_chapter_three(&mut engine, p0);
    flash_in_a_tidebinder(&mut engine, p0, chapter);

    // Drain the stack. Nothing of the Saga's is left on it once the counter
    // resolves, which is the state CR 714.4 asks about.
    let mut guard = 0;
    while !engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Stack)
        .is_empty()
    {
        guard += 1;
        assert!(guard < 40, "the stack never drained");
        drive(&mut engine, 1);
    }
    assert!(
        chapter_ids(&engine, saga).is_empty(),
        "chapter III is still on the stack, so it was never countered"
    );
    // The other half of the Tidebinder's sentence reaches an artifact, a
    // creature or a planeswalker, and Urza's Saga is a land enchantment. It
    // is asserted here because this is the only board in the suite where
    // that clause is asked about something it must not reach — and because
    // it would otherwise be invisible: the Saga has no keywords to lose,
    // which is what a widened filter would take from it.
    assert!(
        !engine.state().effects.iter().any(
            |fx| matches!(fx.filter, crate::effects::EffectFilter::ObjectIs(id) if id == saga)
        ),
        "the tidebinder's rider was registered against a land"
    );
    assert!(
        saga_object(&engine).is_none(),
        "CR 714.4: the saga is at {} lore counters with no chapter of its own \
         on the stack, and is still on the battlefield",
        lore(&engine, saga)
    );
    assert!(
        engine
            .state()
            .zones
            .list(crate::zone::ZoneLocation::Graveyard(p0))
            .iter()
            .any(|id| engine
                .state()
                .object(*id)
                .and_then(|o| o.card)
                .is_some_and(|c| c.index == urzas_saga())),
        "the saga left the battlefield without reaching its owner's graveyard"
    );
}
