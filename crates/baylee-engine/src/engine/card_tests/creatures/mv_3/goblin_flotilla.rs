//! `cards/creatures/mv_3/goblin_flotilla.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::choice::YesNoPrompt;
use crate::turn::{Phase, Step};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn goblin_flotilla() -> CardIndex {
    card_index("014ca38f-d4ac-4417-bfa8-785a7d6fce43")
}

/// Walks to the "pay {R}?" question the beginning-of-combat trigger asks and
/// returns who is asked.
#[track_caller]
fn tax_question(engine: &mut Engine<RegistryLookup>) -> PlayerId {
    pass_until(engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayMana { .. },
                ..
            }
        )
    });
    engine.pending().asked().expect("someone is asked")
}

/// Declines the {R}.
#[track_caller]
fn decline(engine: &mut Engine<RegistryLookup>) {
    let who = tax_question(engine);
    engine
        .apply(who, PlayerAction::YesNo(false))
        .expect("declined");
}

/// Pays the {R} off a Mountain.
#[track_caller]
fn pay(engine: &mut Engine<RegistryLookup>) {
    let who = tax_question(engine);
    engine
        .apply(who, PlayerAction::YesNo(true))
        .expect("agreed to pay");
    tap_all_mana(engine, who);
    engine
        .apply(who, PlayerAction::PassPriority)
        .expect("the payment is made");
}

#[track_caller]
fn attack_with(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    attacker: ObjectId,
    at: PlayerId,
) {
    pass_until(engine, |e| e.state().turn.step == Step::DeclareAttackers);
    engine
        .apply(
            seat,
            PlayerAction::DeclareAttackers {
                attackers: vec![(attacker, Defender::Player(at))],
            },
        )
        .expect("the attack is declared");
    pass_until(engine, |e| e.state().turn.step == Step::DeclareBlockers);
}

/// Islandwalk (printed): with an Island on the other side the Ogre cannot
/// block the Flotilla; without one it can.
#[test]
fn goblin_flotilla_has_islandwalk() {
    let mut results = Vec::new();
    for island_there in [true, false] {
        let mut theirs = vec![gray_ogre()];
        if island_there {
            theirs.push(island());
        }
        let mut engine = Duel::new(SEED, mountain())
            .battlefield(0, &[goblin_flotilla()])
            .battlefield(1, &theirs)
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, P0);
        let flotilla = on_battlefield(&engine, P0, goblin_flotilla()).expect("the Flotilla");
        let ogre = on_battlefield(&engine, P1, gray_ogre()).expect("their Ogre");
        assert!(keywords(&engine, flotilla).contains(KeywordSet::ISLANDWALK));
        decline(&mut engine);
        attack_with(&mut engine, P0, flotilla, P1);
        let block = engine.apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(ogre, flotilla)],
            },
        );
        results.push(block.is_ok());
        if block.is_err() {
            pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
            assert_eq!(
                engine.state().players[1].life,
                18,
                "unblockable, so the two damage went to the player"
            );
        }
    }
    assert_eq!(
        results,
        vec![false, true],
        "blockable only without an Island on their side"
    );
}

/// "At the beginning of each combat, unless you pay {R}, whenever this
/// creature blocks or becomes blocked by a creature this combat, that creature
/// gains first strike": not paid, a blocker gets first strike and wins.
#[test]
fn goblin_flotilla_unpaid_gives_its_blocker_first_strike() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[goblin_flotilla()])
        .battlefield(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let flotilla = on_battlefield(&engine, P0, goblin_flotilla()).expect("the Flotilla");
    let ogre = on_battlefield(&engine, P1, gray_ogre()).expect("their Ogre");
    assert!(!keywords(&engine, ogre).contains(KeywordSet::FIRST_STRIKE));

    decline(&mut engine);
    attack_with(&mut engine, P0, flotilla, P1);
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(ogre, flotilla)],
            },
        )
        .expect("the Ogre blocks");
    pass_until(&mut engine, stack_is_empty);
    assert!(
        keywords(&engine, ogre).contains(KeywordSet::FIRST_STRIKE),
        "the blocking Ogre gained first strike"
    );
    assert!(
        !keywords(&engine, flotilla).contains(KeywordSet::FIRST_STRIKE),
        "only the other creature does"
    );

    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert!(
        on_battlefield(&engine, P0, goblin_flotilla()).is_none(),
        "first strike killed the Flotilla before it dealt damage"
    );
    assert_eq!(
        engine.state().object(ogre).expect("the Ogre lives").damage,
        0
    );
}

/// Paying {R} at the beginning of combat: no first strike, both 2/2s trade.
#[test]
fn goblin_flotilla_paid_gives_nobody_first_strike() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[goblin_flotilla(), mountain()])
        .battlefield(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let flotilla = on_battlefield(&engine, P0, goblin_flotilla()).expect("the Flotilla");
    let ogre = on_battlefield(&engine, P1, gray_ogre()).expect("their Ogre");

    pay(&mut engine);
    attack_with(&mut engine, P0, flotilla, P1);
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(ogre, flotilla)],
            },
        )
        .expect("the Ogre blocks");
    pass_until(&mut engine, stack_is_empty);
    assert!(!keywords(&engine, ogre).contains(KeywordSet::FIRST_STRIKE));

    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert!(
        on_battlefield(&engine, P0, goblin_flotilla()).is_none(),
        "regular damage: the Flotilla dies"
    );
    assert!(
        on_battlefield(&engine, P1, gray_ogre()).is_none(),
        "and so does the Ogre"
    );
}

/// The "blocks" half: the Flotilla on the defending side, its controller
/// declines the {R} at the attacker's beginning of combat, and the creature
/// it blocks gains first strike.
#[test]
fn goblin_flotilla_blocking_gives_the_attacker_first_strike() {
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[gray_ogre()])
        .battlefield(1, &[goblin_flotilla()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let ogre = on_battlefield(&engine, P0, gray_ogre()).expect("the Ogre");
    let flotilla = on_battlefield(&engine, P1, goblin_flotilla()).expect("the Flotilla");

    let asked = tax_question(&mut engine);
    assert_eq!(
        asked, P1,
        "its controller is the one asked, in my combat too"
    );
    decline(&mut engine);
    attack_with(&mut engine, P0, ogre, P1);
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(flotilla, ogre)],
            },
        )
        .expect("the Flotilla blocks");
    pass_until(&mut engine, stack_is_empty);
    assert!(
        keywords(&engine, ogre).contains(KeywordSet::FIRST_STRIKE),
        "the creature blocked by the Flotilla gained first strike"
    );
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert!(on_battlefield(&engine, P1, goblin_flotilla()).is_none());
    assert!(on_battlefield(&engine, P0, gray_ogre()).is_some());
}
