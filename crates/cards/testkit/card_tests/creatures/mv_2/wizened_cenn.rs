//! `cards/creatures/mv_2/wizened_cenn.rs`, played from Oracle text.
#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn wizened_cenn() -> CardIndex {
    card_index("b5905ef3-6565-462f-a015-2bb5b5b09eff")
}

// Checklist: other Kithkin you control get +1/+1; Cenn itself, a non-Kithkin
// under its controller, and a Kithkin controlled by the opponent stay unchanged.
#[test]
fn wizened_cenn_buffs_other_kithkin_on_its_side_only() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[wizened_cenn(), index::GOLDMEADOW_HARRIER, llanowar_elves()],
        )
        .battlefield(1, &[index::GOLDMEADOW_HARRIER])
        .start();
    keep_mulligans(&mut e);
    let cenn = on_battlefield(&e, p0, wizened_cenn()).expect("Cenn");
    let kin = on_battlefield(&e, p0, index::GOLDMEADOW_HARRIER).expect("our other Kithkin");
    let elf = on_battlefield(&e, p0, llanowar_elves()).expect("non-Kithkin");
    let their_kin = on_battlefield(&e, p1, index::GOLDMEADOW_HARRIER).expect("opposing Kithkin");
    assert_eq!(pt(&e, cenn), (2, 2), "other excludes Cenn itself");
    assert_eq!(pt(&e, kin), (2, 2));
    assert_eq!(pt(&e, elf), (1, 1));
    assert_eq!(pt(&e, their_kin), (1, 1));
}
