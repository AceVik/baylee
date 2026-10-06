//! `cards/creatures/mv_2/lotus_cobra.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Lotus Cobra prints one line: Landfall — "Whenever a land you control
/// enters, add one mana of any color." This scenario plays it from the
/// beginning: the 2/1 Snake enters for {1}{G}, the pool is empty afterward,
/// and when its controller plays a land, the color question covers all five
/// colors — so the black mana in the pool can only come from the trigger, and
/// the land that triggered it stayed untapped. The second clause "you
/// control" is checked with a land that enters under the opponent's control:
/// there no one asks for a color.
#[test]
fn lotus_cobra_adds_a_mana_of_any_color_for_your_land_and_none_for_theirs() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(31, forest())
        .battlefield(0, &[forest(), forest()])
        .hand(0, &[lotus_cobra(), forest()])
        .hand(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // {1}{G} from the two Forests, and after that the pool is empty again —
    // this is what makes the mana reading further down an exact statement.
    cast_from_hand(&mut engine, p0, lotus_cobra());
    pass_until(&mut engine, stack_is_empty);
    let cobra = on_battlefield(&engine, p0, lotus_cobra()).expect("the Cobra resolved");
    assert_eq!(pt(&engine, cobra), (2, 1), "the body the card prints");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{1}}{{G}} is all that the two Forests gave"
    );

    // The land drop that the card is about.
    let land = play_land(&mut engine, p0, forest());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseColor { .. })
    });
    let Pending::ChooseColor { player, options } = engine.pending().clone() else {
        unreachable!("pass_until hält auf nichts als der Farbfrage an");
    };
    assert_eq!(player, p0, "the controller of the Cobra names the color");
    assert_eq!(
        options.len(),
        5,
        "\"any color\" is the game's five colors (CR 105.4): {options:?}"
    );
    for color in [
        ManaColor::White,
        ManaColor::Blue,
        ManaColor::Black,
        ManaColor::Red,
        ManaColor::Green,
    ] {
        assert!(
            options.contains(&color),
            "`any color` schließt {color:?} ein"
        );
    }

    engine
        .apply(p0, PlayerAction::ChooseColor(ManaColor::Black))
        .expect("black was one of the offered colors");
    pass_until(&mut engine, stack_is_empty);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Black),
        1,
        "the named color, and no default value"
    );
    assert_eq!(
        pool.total(),
        1,
        "ein Mana, aus einem Land, und sonst nichts"
    );
    assert!(
        !is_tapped(&engine, land),
        "der Wald, der den Auslöser ausgelöst hat, hat selbst nichts bezahlt: \
         das Mana kam von der Cobra"
    );

    // Die andere Hälfte von „you control": dasselbe Ereignis auf der anderen
    // Seite des Tisches.
    reach_their_main_phase(&mut engine, p1);
    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!("p1 holds a main phase priority: {:?}", engine.pending())
    };
    assert_eq!(player, p1, "and it is the opponent who holds it");
    let theirs = *legal.lands.first().expect("p1 has a land in hand");
    engine
        .apply(p1, PlayerAction::PlayLand { card: theirs })
        .expect("a land drop in one's own main phase");

    // Ein Landfall-Auslöser würde die Farbfrage auf den Stapel legen, und sie
    // wäre beantwortet, bevor diese Phase enden kann; nichts anderes im Spiel
    // stellt hier eine.
    let mut asked = false;
    for _ in 0..8 {
        match engine.pending().clone() {
            Pending::ChooseColor { .. } => {
                asked = true;
                break;
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            _ => break,
        }
    }
    assert!(
        !asked,
        "a land under the opponent's control is not a land *you control*, so \
         the Cobra offers no mana"
    );
}
