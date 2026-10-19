//! `cards/creatures/mv_5/thrull_champion.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// A duel at P0's first main phase with the named hand and boards, and every
/// colour of mana floating for both seats.
fn prepared(
    hand: &[CardIndex],
    mine: &[CardIndex],
    theirs: &[CardIndex],
) -> Engine<RegistryLookup> {
    let mut e = Duel::new(SEED, forest())
        .hand(0, hand)
        .battlefield(0, mine)
        .battlefield(1, theirs)
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    mana(&mut e);
    e
}

fn mana(e: &mut Engine<RegistryLookup>) {
    for p in &mut e.state.players {
        for c in ManaColor::ALL {
            p.mana_pool.add(c, 30);
        }
    }
    e.refresh_offer();
}

fn priority(e: &mut Engine<RegistryLookup>, p: PlayerId) {
    pass_until(
        e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p),
    );
}

fn activate_by_card(e: &mut Engine<RegistryLookup>, p: PlayerId, c: CardIndex, ability_index: u32) {
    priority(e, p);
    mana(e);
    let source = on_battlefield(e, p, c).expect("the source is on the battlefield");
    e.apply(
        p,
        PlayerAction::ActivateAbility {
            source,
            ability_index,
        },
    )
    .expect("the ability is activated");
}

fn targets(e: &mut Engine<RegistryLookup>, objects: Vec<ObjectId>) {
    let p = e.pending().asked().expect("a question is pending");
    e.apply(
        p,
        PlayerAction::ChooseTargets {
            objects,
            players: vec![],
        },
    )
    .expect("the targets are on the menu");
}

fn thrull_champion() -> CardIndex {
    card_index("728f0a3f-722d-4936-ade8-31883499f56d")
}

fn basal_thrull() -> CardIndex {
    card_index("9da50130-3f83-4968-983c-7dcba257cf1b")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Thrull Champion (728f0a3f-722d-4936-ade8-31883499f56d) — "Thrull creatures
/// get +1/+1."
#[test]
fn thrull_champion_gives_every_thrull_plus_one_plus_one_and_only_while_it_lives() {
    let mut e = prepared(
        &[],
        &[thrull_champion(), grizzly_bears()],
        &[basal_thrull()],
    );
    let champion = on_battlefield(&e, P0, thrull_champion()).expect("the Champion is out");
    let bears = on_battlefield(&e, P0, grizzly_bears()).expect("Grizzly Bears is out");
    let thrull = on_battlefield(&e, P1, basal_thrull()).expect("Basal Thrull is out");

    assert_eq!(pt(&e, champion), (3, 3), "a Thrull itself: 2/2 plus 1/+1");
    assert_eq!(
        pt(&e, thrull),
        (2, 3),
        "the opponent's 1/2 Thrull: plus 1/+1"
    );
    assert_eq!(pt(&e, bears), (2, 2), "a Bear is no Thrull");

    kill(&mut e, champion);
    assert_eq!(
        pt(&e, thrull),
        (1, 2),
        "without the Champion: the printed 1/2"
    );
}

/// Thrull Champion — "{T}: Gain control of target Thrull for as long as you
/// control this creature."
#[test]
fn thrull_champion_steals_a_thrull_until_it_leaves() {
    let mut e = prepared(&[], &[thrull_champion()], &[basal_thrull()]);
    let champion = on_battlefield(&e, P0, thrull_champion()).expect("the Champion is out");
    let thrull = on_battlefield(&e, P1, basal_thrull()).expect("Basal Thrull is out");
    assert_eq!(e.state().object(thrull).unwrap().controller, P1);

    activate_by_card(&mut e, P0, thrull_champion(), 1);
    targets(&mut e, vec![thrull]);
    pass_until(&mut e, stack_is_empty);
    assert!(is_tapped(&e, champion), "the ability's cost is {{T}}");
    assert_eq!(
        e.state().object(thrull).unwrap().controller,
        P0,
        "P0 controls the Thrull"
    );
    assert_eq!(pt(&e, thrull), (2, 3), "and it is still lorded");

    kill(&mut e, champion);
    assert_eq!(
        e.state().object(thrull).unwrap().controller,
        P1,
        "control lasts only while P0 controls the Champion"
    );
    assert_eq!(pt(&e, thrull), (1, 2));
}

/// Thrull Champion — the ability targets a *Thrull* only.
#[test]
fn thrull_champion_cannot_aim_at_a_non_thrull() {
    let mut e = prepared(
        &[],
        &[thrull_champion()],
        &[basal_thrull(), grizzly_bears()],
    );
    let thrull = on_battlefield(&e, P1, basal_thrull()).expect("Basal Thrull is out");
    let bears = on_battlefield(&e, P1, grizzly_bears()).expect("Grizzly Bears is out");
    activate_by_card(&mut e, P0, thrull_champion(), 1);
    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        panic!("the ability asks for a target, got {:?}", e.pending())
    };
    assert!(options.contains(&thrull));
    assert!(!options.contains(&bears), "Grizzly Bears is no Thrull");
}
