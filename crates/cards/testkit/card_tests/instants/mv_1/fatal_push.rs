//! `cards/instants/mv_1/fatal_push.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn fatal_push() -> CardIndex {
    card_index("16437a83-be52-44cd-a768-a767c9347eb2")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

/// Every creature is a legal target; the mana-value restriction is checked
/// only when the spell resolves. Without Revolt, two is destroyed and four is
/// not. Resolving the first instant does not itself enable Revolt.
#[test]
fn fatal_push_without_revolt_destroys_only_mana_value_two_or_less() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(6201, swamp())
        .hand(0, &[fatal_push(), fatal_push()])
        .battlefield(0, &[swamp(), swamp()])
        .battlefield(1, &[grizzly_bears(), juzam_djinn(), serra_angel()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    tap_all_mana(&mut engine, p0);

    let bears = on_battlefield(&engine, p1, grizzly_bears()).expect("their Bears");
    let djinn = on_battlefield(&engine, p1, juzam_djinn()).expect("their Djinn");
    let angel = on_battlefield(&engine, p1, serra_angel()).expect("their Angel");
    cast_with_floating(&mut engine, p0, fatal_push());
    let options = aim_at(&mut engine, p0, bears);
    assert!(options.contains(&djinn) && options.contains(&angel));
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p1, grizzly_bears()).is_some());

    cast_with_floating(&mut engine, p0, fatal_push());
    aim_at(&mut engine, p0, djinn);
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p1, juzam_djinn()).is_some());
    assert!(on_battlefield(&engine, p1, serra_angel()).is_some());
    assert!(!engine.state().per_turn.revolt[0]);
}

/// A real destruction play enables Revolt for the rest of the turn. It then
/// raises the limit to four, not five.
#[test]
fn fatal_push_revolt_destroys_mana_value_four_but_not_five() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(6202, swamp())
        .hand(0, &[fatal_push(), fatal_push(), fatal_push()])
        .battlefield(0, &[grizzly_bears(), swamp(), swamp(), swamp()])
        .battlefield(1, &[juzam_djinn(), serra_angel()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    tap_all_mana(&mut engine, p0);

    let own_bears = on_battlefield(&engine, p0, grizzly_bears()).expect("our Bears");
    cast_with_floating(&mut engine, p0, fatal_push());
    aim_at(&mut engine, p0, own_bears);
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p0, grizzly_bears()).is_some());
    assert!(engine.state().per_turn.revolt[0]);

    let djinn = on_battlefield(&engine, p1, juzam_djinn()).expect("their Djinn");
    cast_with_floating(&mut engine, p0, fatal_push());
    aim_at(&mut engine, p0, djinn);
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p1, juzam_djinn()).is_some());

    let angel = on_battlefield(&engine, p1, serra_angel()).expect("their Angel");
    cast_with_floating(&mut engine, p0, fatal_push());
    aim_at(&mut engine, p0, angel);
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p1, serra_angel()).is_some());
}
