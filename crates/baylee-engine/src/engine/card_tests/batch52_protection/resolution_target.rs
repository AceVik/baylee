// Append as separate child of batch52_protection; all helpers inherited.
#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn gaining_protection_in_response_fizzles_whole_psionic_blast_including_caster_damage() {
    let mut e = Duel::new(SEED, index::FOREST)
        .hand(0, &[index::PSIONIC_BLAST])
        .hand(1, &[index::TEFERI_S_PROTECTION])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    mana(&mut e);
    cast(&mut e, P0, index::PSIONIC_BLAST);
    targets(&mut e, vec![], vec![P1]);
    cast(&mut e, P1, index::TEFERI_S_PROTECTION);
    let life = e.state.players[0].life;
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state.players[0].life, life,
        "illegal sole target prevents the entire spell resolving: even its untargeted two damage to its caster"
    );
}
