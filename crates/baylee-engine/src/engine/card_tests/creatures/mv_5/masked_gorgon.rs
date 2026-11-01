//! `cards/creatures/mv_5/masked_gorgon.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn masked_gorgon_grants_green_and_white_creatures_protection_from_gorgons_and_gains_protection_at_threshold()
 {
    let gorgon = card_index("f41b1a5b-bc99-4c51-9665-e251f04ad83f");
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));

    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                savannah_lions(),
            ],
        )
        .battlefield(1, &[grizzly_bears(), savannah_lions(), gray_ogre()])
        .hand(0, &[gorgon, dark_ritual()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    seed_graveyard(&mut engine, p0, 6);
    for s in all_on_battlefield(&engine, p0, swamp()).into_iter().take(5) {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source: s })
            .unwrap();
    }
    let card = in_hand(&engine, p0, gorgon).unwrap();
    engine.apply(p0, PlayerAction::CastSpell { card }).unwrap();
    pass_until(&mut engine, stack_is_empty);

    let mg = on_battlefield(&engine, p0, gorgon).expect("Masked Gorgon on battlefield");
    let p0_lions = on_battlefield(&engine, p0, savannah_lions()).expect("P0 Savannah Lions");
    let p1_bears = on_battlefield(&engine, p1, grizzly_bears()).expect("P1 Grizzly Bears");
    let p1_lions = on_battlefield(&engine, p1, savannah_lions()).expect("P1 Savannah Lions");
    let p1_ogre = on_battlefield(&engine, p1, gray_ogre()).expect("P1 Gray Ogre");

    // Cast Dark Ritual as the 7th card entering P0's graveyard (threshold).
    let last_swamp = all_on_battlefield(&engine, p0, swamp())
        .into_iter()
        .find(|&id| !is_tapped(&engine, id))
        .unwrap();
    engine
        .apply(p0, PlayerAction::ActivateManaAbility { source: last_swamp })
        .unwrap();
    let dr = in_hand(&engine, p0, dark_ritual()).unwrap();
    engine
        .apply(p0, PlayerAction::CastSpell { card: dr })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    // Advance to P1's turn: P1 attacks with Grizzly Bears (green); P0's Masked Gorgon
    // cannot block it because green creatures have protection from Gorgons, while
    // P0's Savannah Lions can block it.
    reach_their_main_phase(&mut engine, p1);
    let p1_blocks = attack_and_collect_blocks(&mut engine, p1_bears, p0);
    assert!(
        !p1_blocks.iter().any(|opt| opt.blocker == mg),
        "Masked Gorgon cannot block a green creature (protection from Gorgons)"
    );
    assert!(
        p1_blocks.iter().any(|opt| opt.blocker == p0_lions),
        "Savannah Lions can block Grizzly Bears"
    );
    engine
        .apply(p0, PlayerAction::DeclareBlockers { blockers: vec![] })
        .expect("no blocks");
    pass_until(&mut engine, |e| {
        matches!(e.state().turn.phase, Phase::SecondMain) && stack_is_empty(e)
    });

    // Advance back to P0's turn: Masked Gorgon attacks at threshold (protection from
    // green and from white), so P1's Grizzly Bears and Savannah Lions cannot block it,
    // while P1's Gray Ogre (red) can.
    reach_their_main_phase(&mut engine, p0);
    let p0_blocks = attack_and_collect_blocks(&mut engine, mg, p1);
    assert!(
        !p0_blocks.iter().any(|opt| opt.blocker == p1_bears),
        "green creature cannot block Masked Gorgon at threshold"
    );
    assert!(
        !p0_blocks.iter().any(|opt| opt.blocker == p1_lions),
        "white creature cannot block Masked Gorgon at threshold"
    );
    assert!(
        p0_blocks.iter().any(|opt| opt.blocker == p1_ogre),
        "red creature can still block Masked Gorgon at threshold"
    );
}
