//! `cards/creatures/mv_3/el_hajjaj.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn el_hajjaj() -> CardIndex {
    card_index("f92c9a5d-853f-4157-89ba-8d8c8033c533")
}

fn fog() -> CardIndex {
    card_index("27e9db49-7af7-4bef-ad4c-bf5dfb92030d")
}

/// Declares El-Hajjâj as p0's attacker and returns the blockers offer.
fn attack(engine: &mut Engine<RegistryLookup>, hajjaj: ObjectId) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    declare_band_attack(engine, p0, p1, &[hajjaj]);
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
}

fn life_changes(engine: &Engine<RegistryLookup>, seat: PlayerId) -> usize {
    engine
        .journal()
        .entries()
        .iter()
        .filter(|e| matches!(e.event, crate::event::GameEvent::LifeChanged { player, .. } if player == seat))
        .count()
}

/// El-Hajjâj: "Whenever this creature deals damage, you gain that much
/// life." A 1/1, unblocked, hits the player for 1 and its controller gains 1.
#[test]
fn el_hajjaj_gains_life_for_unblocked_combat_damage_to_a_player() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[el_hajjaj()])
        .life(0, 10)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let hajjaj = on_battlefield(&engine, p0, el_hajjaj()).expect("seated");
    attack(&mut engine, hajjaj);
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
    });
    assert_eq!(engine.state().players[1].life, 19, "1 damage dealt");
    assert_eq!(engine.state().players[0].life, 11, "and 1 life gained");
}

/// Blocked by a creature, it deals damage to that creature (not to a player)
/// and still gains that much: a 1/1 hitting a 2/2 Gray Ogre gains 1, and the
/// Ogre takes it.
#[test]
fn el_hajjaj_gains_the_damage_it_dealt_to_a_blocker() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[el_hajjaj()])
        .battlefield(1, &[gray_ogre()])
        .life(0, 10)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let hajjaj = on_battlefield(&engine, p0, el_hajjaj()).expect("seated");
    let ogre = on_battlefield(&engine, p1, gray_ogre()).expect("seated");
    attack(&mut engine, hajjaj);
    declare_band_blocks(&mut engine, p1, &[(ogre, hajjaj)]);
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
    });
    assert_eq!(
        engine.state().object(ogre).map(|o| o.damage),
        Some(1),
        "the Ogre took 1"
    );
    assert_eq!(engine.state().players[1].life, 20, "no player was hit");
    assert_eq!(engine.state().players[0].life, 11, "1 life for 1 damage");
}

/// A pumped 4-power El-Hajjâj blocked by two Gray Ogres deals one combat
/// damage step's worth of damage, however it is split, and its trigger is one
/// trigger for the whole step: its controller gains the total once (4), in a
/// single life change.
#[test]
fn el_hajjaj_double_blocked_triggers_once_for_the_total() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[el_hajjaj(), forest()])
        .hand(0, &[giant_growth()])
        .battlefield(1, &[gray_ogre(), gray_ogre()])
        .life(0, 10)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let hajjaj = on_battlefield(&engine, p0, el_hajjaj()).expect("seated");
    let ogres = all_on_battlefield(&engine, p1, gray_ogre());
    assert_eq!(ogres.len(), 2);

    cast_from_hand(&mut engine, p0, giant_growth());
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![hajjaj],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, hajjaj), (4, 4));

    attack(&mut engine, hajjaj);
    declare_band_blocks(&mut engine, p1, &[(ogres[0], hajjaj), (ogres[1], hajjaj)]);
    let before = life_changes(&engine, p0);
    // An unbanded attacker's controller may divide its damage (CR 510.1c);
    // answer any such question by giving the first blocker lethal (2) and the
    // rest to the second.
    loop {
        match engine.pending().clone() {
            Pending::ChooseNumber { player, max, .. } => {
                let n = if max >= 2 { 2 } else { max };
                engine.apply(player, PlayerAction::ChooseNumber(n)).unwrap();
            }
            Pending::Priority { player, .. } => {
                if matches!(engine.state().turn.phase, Phase::SecondMain) {
                    break;
                }
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected: {other:?}"),
        }
    }
    assert!(
        in_graveyard(&engine, p1, gray_ogre()).is_some(),
        "at least one blocker took lethal damage"
    );
    assert_eq!(
        engine.state().players[0].life,
        14,
        "gained the whole 4 combat damage dealt"
    );
    // However the 4 damage was split between the two blockers, it is one
    // damage step: one trigger, one gain of the total.
    assert_eq!(
        life_changes(&engine, p0) - before,
        1,
        "one trigger for the step, so one life change"
    );
}

/// Prevented damage is not dealt: Fog stops El-Hajjâj's combat damage, and
/// there is nothing to gain.
#[test]
fn el_hajjaj_gains_nothing_when_its_damage_is_prevented() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[el_hajjaj()])
        .battlefield(1, &[forest(), forest()])
        .hand(1, &[fog()])
        .life(0, 10)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let hajjaj = on_battlefield(&engine, p0, el_hajjaj()).expect("seated");
    attack(&mut engine, hajjaj);
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();
    // Fog is an instant: p1 casts it once priority comes round after blocks.
    pass_until(
        &mut engine,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p1),
    );
    cast_from_hand(&mut engine, p1, fog());
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain)
    });
    assert_eq!(engine.state().players[1].life, 20, "damage prevented");
    assert_eq!(engine.state().players[0].life, 10, "so no life gained");
    let _ = p0;
}
