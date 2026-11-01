//! `cards/creatures/mv_2/wall_of_diffusion.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn wall_of_diffusion() -> CardIndex {
    card_index("ae42f803-f4ac-45f3-9ce7-227a59096f19")
}

/// `Wall of Diffusion` ({1}{R}, 0/5 Creature — Wall, `Coverage::Partial`):
/// `Defender`
/// `This creature can block creatures with shadow as though it had shadow.`
#[test]
fn wall_of_diffusion_casts_and_enters_as_a_zero_five_with_defender() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain()])
        .hand(0, &[wall_of_diffusion()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, wall_of_diffusion());
    pass_until(&mut engine, stack_is_empty);

    let wall = on_battlefield(&engine, p0, wall_of_diffusion()).unwrap();
    let ch = engine.state().object(wall).unwrap().characteristics();
    assert_eq!((ch.power, ch.toughness), (Some(0), Some(5)));
    assert!(ch.keywords.contains(KeywordSet::DEFENDER));
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending() else {
        panic!("attackers");
    };
    assert!(!attackers.contains(&wall), "defender is enforced in play");
}
