//! `cards/creatures/mv_2/skyshroud_war_beast.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
fn toughness_of(engine: &Engine<RegistryLookup>, object: ObjectId) -> Option<i16> {
    engine
        .state()
        .object(object)
        .and_then(|o| o.characteristics().toughness)
}

const P1: PlayerId = PlayerId::new(1);

fn skyshroud_war_beast() -> CardIndex {
    card_index("c6ad434b-6e34-439b-8046-f0047c06e371")
}

fn forest() -> CardIndex {
    card_index("b34bb2dc-c1af-4d77-b0b3-a0fb342a5fc6")
}

fn karplusan_forest() -> CardIndex {
    card_index("b53f216d-1592-4eee-b204-502a805fbc8c")
}

#[test]
fn skyshroud_war_beast_sizes_to_chosen_opponents_nonbasic_lands() {
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), karplusan_forest(), karplusan_forest()])
        .battlefield(1, &[forest(), karplusan_forest()])
        .hand(0, &[skyshroud_war_beast()])
        .hand(1, &[karplusan_forest()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);

    cast_from_hand(&mut e, P0, skyshroud_war_beast());
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChoosePlayer { .. })
    });
    e.apply(P0, PlayerAction::ChoosePlayer(P1)).unwrap();
    pass_until(&mut e, stack_is_empty);

    let beast = on_battlefield(&e, P0, skyshroud_war_beast()).unwrap();
    assert!(keywords_of(&e, beast).contains(KeywordSet::TRAMPLE));
    assert_eq!(
        (power_of(&e, beast), toughness_of(&e, beast)),
        (Some(1), Some(1)),
        "ignores P0 nonbasics and P1 basics"
    );

    // P1 plays another nonbasic land -> War Beast grows dynamically to 2/2.
    reach_their_main_phase(&mut e, P1);
    play_land(&mut e, P1, karplusan_forest());
    assert_eq!(
        (power_of(&e, beast), toughness_of(&e, beast)),
        (Some(2), Some(2))
    );
}
