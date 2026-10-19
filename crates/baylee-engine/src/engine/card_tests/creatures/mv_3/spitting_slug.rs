//! `cards/creatures/mv_3/spitting_slug.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::choice::YesNoPrompt;
use crate::turn::{Phase, Step};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn spitting_slug() -> CardIndex {
    card_index("77af5e31-ac4d-4112-9138-e12abb5ce144")
}

/// A Spitting Slug (2/4, with two Forests) and a Gray Ogre (2/2) fight on turn
/// one, P0 attacking. The Slug is on the attacking side when `slug_attacks`,
/// else it is the blocker. Its controller is asked "pay {1}{G}?" and answers
/// `pay`; the trigger is resolved and combat damage not yet dealt.
fn scene(slug_attacks: bool, pay: bool) -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
    let (slug_seat, ogre_seat) = if slug_attacks { (0, 1) } else { (1, 0) };
    let mut engine = Duel::new(SEED, forest())
        .battlefield(slug_seat, &[spitting_slug(), forest(), forest()])
        .battlefield(ogre_seat, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);
    let slug_player = PlayerId::new(u8::try_from(slug_seat).expect("a seat"));
    let ogre_player = PlayerId::new(u8::try_from(ogre_seat).expect("a seat"));
    let slug = on_battlefield(&engine, slug_player, spitting_slug()).expect("the Slug is out");
    let ogre = on_battlefield(&engine, ogre_player, gray_ogre()).expect("the Ogre is out");
    let (attacker, blocker) = if slug_attacks {
        (slug, ogre)
    } else {
        (ogre, slug)
    };

    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::DeclareAttackers
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(attacker, Defender::Player(P1))],
            },
        )
        .expect("the attack is declared");
    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::DeclareBlockers
    });
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(blocker, attacker)],
            },
        )
        .expect("the block is declared");

    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayMana { .. },
                ..
            }
        )
    });
    let asked = engine.pending().asked().expect("someone is asked");
    assert_eq!(asked, slug_player, "the Slug's controller is asked");
    engine
        .apply(asked, PlayerAction::YesNo(pay))
        .expect("answered");
    if pay {
        tap_all_mana(&mut engine, asked);
        engine
            .apply(asked, PlayerAction::PassPriority)
            .expect("the payment is made");
    }
    pass_until(&mut engine, stack_is_empty);
    (engine, slug, ogre)
}

fn first_strike(engine: &Engine<RegistryLookup>, id: ObjectId) -> bool {
    keywords(engine, id).contains(KeywordSet::FIRST_STRIKE)
}

fn damage_of(engine: &Engine<RegistryLookup>, id: ObjectId) -> u32 {
    u32::from(engine.state().object(id).expect("the object").damage)
}

/// Spitting Slug (77af5e31-ac4d-4112-9138-e12abb5ce144) — "Whenever this
/// creature blocks or becomes blocked, you may pay {1}{G}. If you do, this
/// creature gains first strike until end of turn. Otherwise, each creature
/// blocking or blocked by this creature gains first strike until end of turn."
/// Attacking and blocked, paid: the Slug has first strike, kills the Ogre
/// before it deals damage.
#[test]
fn spitting_slug_blocked_and_paid_gains_first_strike() {
    let (mut engine, slug, ogre) = scene(true, true);
    assert!(first_strike(&engine, slug), "the Slug gained first strike");
    assert!(!first_strike(&engine, ogre), "the Ogre did not");
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert!(
        on_battlefield(&engine, P1, gray_ogre()).is_none(),
        "the Ogre died"
    );
    assert_eq!(damage_of(&engine, slug), 0, "dead before it could hit back");
}

/// Blocked and not paid: the blocker gets first strike instead, not the Slug.
#[test]
fn spitting_slug_blocked_and_unpaid_gives_the_blocker_first_strike() {
    let (mut engine, slug, ogre) = scene(true, false);
    assert!(!first_strike(&engine, slug));
    assert!(first_strike(&engine, ogre), "the blocking Ogre gained it");
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(
        damage_of(&engine, slug),
        2,
        "the Ogre struck first, then died to the 2/4's regular damage"
    );
    assert!(on_battlefield(&engine, P1, gray_ogre()).is_none());
    assert!(on_battlefield(&engine, P0, spitting_slug()).is_some());
}

/// The "blocks" half: the Slug blocks, does not pay, and the creature it
/// blocked gains first strike.
#[test]
fn spitting_slug_blocking_and_unpaid_gives_the_attacker_first_strike() {
    let (mut engine, slug, ogre) = scene(false, false);
    assert!(!first_strike(&engine, slug));
    assert!(first_strike(&engine, ogre), "the blocked Ogre gained it");
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(damage_of(&engine, slug), 2);
    assert!(on_battlefield(&engine, P0, gray_ogre()).is_none());
}

/// Blocking and paid: the Slug has first strike.
#[test]
fn spitting_slug_blocking_and_paid_gains_first_strike() {
    let (mut engine, slug, ogre) = scene(false, true);
    assert!(first_strike(&engine, slug));
    assert!(!first_strike(&engine, ogre));
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(damage_of(&engine, slug), 0);
    assert!(on_battlefield(&engine, P0, gray_ogre()).is_none());
}
