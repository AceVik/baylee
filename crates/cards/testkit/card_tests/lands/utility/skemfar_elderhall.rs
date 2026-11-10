//! `cards/lands/utility/skemfar_elderhall.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Skemfar Elderhall` prints `This land enters tapped.`, `{{T}}: Add {{G}}.`, and `{{2}}{{B}}{{B}}{{G}}, {{T}}, Sacrifice this land: Up to one target creature you don't control gets -2/-2 until end of turn. Create two 1/1 green Elf Warrior creature tokens. Activate only as a sorcery.`
///
/// With no creature it does not control on the board, the "up to one target" -2/-2 names nothing and nobody is asked (CR 115.6); `activation_target_tests` plays it with a target.
/// Playing this land from hand puts it onto the battlefield tapped; after untapping on the next turn, floating `{{2}}{{B}}{{B}}{{G}}` allows activating ability 1 at sorcery speed, which sacrifices `Skemfar Elderhall` and creates two 1/1 green Elf Warrior creature tokens.
#[test]
fn skemfar_elderhall_enters_tapped_sacrifices_and_creates_two_elf_warrior_tokens() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[skemfar_elderhall()])
        .battlefield(0, &[swamp(), swamp(), forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let card = in_hand(&engine, p0, skemfar_elderhall()).expect("skemfar elderhall in hand");
    engine.apply(p0, PlayerAction::PlayLand { card }).unwrap();

    let elderhall =
        on_battlefield(&engine, p0, skemfar_elderhall()).expect("skemfar elderhall on battlefield");
    assert!(entered_tapped(&engine, elderhall));

    // Advance to p0's next main phase to untap.
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, elderhall));

    // Float {{2}}{{B}}{{B}}{{G}} (5 mana: 2 black, 3 green) from basics while keeping Skemfar Elderhall untapped.
    tap_mana_except(&mut engine, p0, elderhall);
    assert_eq!(engine.state().players[0].mana_pool.total(), 5);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Black),
        2
    );
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Green),
        3
    );
    assert!(!is_tapped(&engine, elderhall));

    activate(&mut engine, p0, skemfar_elderhall(), 1);

    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, skemfar_elderhall()).is_none());
    assert!(in_graveyard(&engine, p0, skemfar_elderhall()).is_some());

    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 2, "creates two Elf Warrior tokens");
    for &tok in &tokens {
        assert_eq!(pt(&engine, tok), (1, 1), "each token is a 1/1");
    }
}
