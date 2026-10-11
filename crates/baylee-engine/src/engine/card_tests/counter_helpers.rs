//! Shared real stack fixtures for source- and target-qualified counters.
#[allow(clippy::wildcard_imports)] // card test vocabulary
use super::*;
use baylee_core::generated::index;
pub(super) fn artifact_activation(counter: CardIndex) -> (Engine<RegistryLookup>, ObjectId) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let friendly = if counter == index::AYESHA_TANAKA {
        vec![counter]
    } else {
        vec![]
    };
    let mut e = Duel::new(SEED, forest())
        .hand(0, &[counter])
        .battlefield(0, &friendly)
        .battlefield(1, &[index::MIND_STONE])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    e.apply(p0, PlayerAction::PassPriority).unwrap();
    e.dev_state_mut(p0).unwrap().players[1]
        .mana_pool
        .add(ManaColor::Colorless, 1);
    e.refresh_offer();
    activate(&mut e, p1, index::MIND_STONE, 1);
    let ability = *e.state().zones.list(ZoneLocation::Stack).last().unwrap();
    pass_until(
        &mut e,
        |e| matches!(e.pending(),Pending::Priority{player,..} if *player==p0),
    );
    (e, ability)
}
pub(super) fn spell_fixture(
    counter: CardIndex,
    case: u8,
) -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let spell = match case {
        2 => index::CHAIN_LIGHTNING,
        3 => index::UNHOLY_STRENGTH,
        _ => index::LIGHTNING_BOLT,
    };
    let friendly = if counter == index::RING_OF_IMMORTALS {
        vec![index::GRIZZLY_BEARS, counter]
    } else {
        vec![index::GRIZZLY_BEARS]
    };
    let mut e = Duel::new(SEED, forest())
        .hand(0, &[counter])
        .hand(1, &[spell])
        .battlefield(0, &friendly)
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut e);
    pass_until(&mut e, |e| {
        e.state().turn.phase == Phase::FirstMain
            && e.state().turn.active == p1
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p1)
    });
    let host = on_battlefield(&e, if case == 1 { p1 } else { p0 }, index::GRIZZLY_BEARS).unwrap();
    let s = e.dev_state_mut(p0).unwrap();
    s.players[1].mana_pool.add(ManaColor::Red, 2);
    s.players[1].mana_pool.add(ManaColor::Black, 2);
    e.refresh_offer();
    cast_with_floating(&mut e, p1, spell);
    aim_at(&mut e, p1, host);
    let spell = on_stack(&e, spell).unwrap();
    pass_until(
        &mut e,
        |e| matches!(e.pending(),Pending::Priority{player,..} if *player==p0),
    );
    (e, spell, host)
}
