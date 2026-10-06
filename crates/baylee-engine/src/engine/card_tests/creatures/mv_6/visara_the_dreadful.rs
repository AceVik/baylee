//! `cards/creatures/mv_6/visara_the_dreadful.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Visara the Dreadful: "{T}: Destroy target creature. It can't be
/// regenerated."
///
/// The clause on an *ability* rather than a spell, which is a different
/// resolver path to the same door — and the one that repeats, since Visara
/// untaps every turn while Terminate is cast once.
///
/// The Troll shields itself for `{B}` off the one Swamp, and Visara needs
/// no mana at all, so the only thing standing between the two creatures is
/// the sentence under test.
#[test]
fn visara_stares_through_a_regeneration_shield() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(515, forest())
        .battlefield(0, &[visara_the_dreadful(), lotleth_troll(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let visara = on_battlefield(&engine, p0, visara_the_dreadful()).expect("the Gorgon is seated");
    let troll = on_battlefield(&engine, p0, lotleth_troll()).expect("the Troll is seated");
    assert!(
        keywords(&engine, visara).contains(KeywordSet::FLYING),
        "flying is the card's other printed line"
    );

    tap_all_mana(&mut engine, p0);
    raise_a_shield(&mut engine, p0, troll, 1);
    assert!(
        !is_tapped(&engine, visara),
        "Visara makes no mana, so tapping the board left her ability payable"
    );

    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: visara,
                ability_index: 0,
            },
        )
        .expect("the tap is the whole cost");
    aim_at(&mut engine, p0, troll);
    pass_until(&mut engine, |e| at_rest(e, p0));

    assert!(
        in_graveyard(&engine, p0, lotleth_troll()).is_some(),
        "the shield did not save it"
    );
    assert!(
        is_tapped(&engine, visara),
        "and the Gorgon paid with her tap"
    );
}
