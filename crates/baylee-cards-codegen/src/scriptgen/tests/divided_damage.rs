//! Synthetic divisions exercise the vocabulary without requiring a corpus checkout.

use super::{read, refused};

#[test]
fn fixed_damage_and_independent_opponent_gain_use_separate_target_groups() {
    let body = read(
        "Name:Test division\nTypes:Sorcery\n\
         A:SP$ DealDamage | ValidTgts$ Any | NumDmg$ 11 | TargetMin$ 0 | TargetMax$ 11 | DividedAsYouChoose$ 11 | SubAbility$ Bonus\n\
         SVar:Bonus:DB$ GainLife | ValidTgts$ Opponent | LifeAmount$ 6",
    );
    let spell = &body.abilities[0];
    assert!(spell.contains("DivisionAmount::Fixed(11)"));
    assert!(spell.contains("TargetReq::up_to(TargetSpec::AnyTarget, 11)"));
    assert!(spell.contains("second_targets = Some(TargetReq::one(TargetSpec::AnyOpponent))"));
    assert!(spell.contains("who: PlayerRel::SecondTargetPlayerOrController"));
    assert!(spell.contains("amount: Amount::Fixed(6)"));
}

#[test]
fn announced_x_plus_an_offset_and_all_target_upper_bound_are_read_together() {
    let body = read(
        "Name:Test offset\nTypes:Instant\n\
         A:SP$ DealDamage | ValidTgts$ Any | NumDmg$ Total | TargetMin$ 0 | TargetMax$ Population | DividedAsYouChoose$ Total\n\
         SVar:Total:SVar$Announced/Plus.2\n\
         SVar:Announced:Count$xPaid\n\
         SVar:Population:PlayerCountPlayers$Amount/Plus.Permanents\n\
         SVar:Permanents:Count$Valid Any",
    );
    assert!(body.abilities[0].contains("DivisionAmount::XPlus(2)"));
    assert!(body.abilities[0].contains("TargetReq::up_to(TargetSpec::AnyTarget, u8::MAX)"));
}

#[test]
fn chosen_x_total_can_itself_bound_object_targets() {
    let body = read(
        "Name:Test creatures\nTypes:Sorcery\n\
         A:SP$ DealDamage | ValidTgts$ Creature | NumDmg$ Chosen | TargetMin$ 0 | TargetMax$ Chosen | DividedAsYouChoose$ Chosen\n\
         SVar:Chosen:Count$xPaid",
    );
    assert!(body.abilities[0].contains("DivisionAmount::XPlus(0)"));
    assert!(body.abilities[0].contains("TargetSpec::Object(&Filter::CREATURE)"));
}

#[test]
fn division_near_misses_refuse_every_unread_rule() {
    let base = "Name:Test division\nTypes:Sorcery\nA:SP$ DealDamage | ValidTgts$ Any | NumDmg$ 11 | TargetMin$ 0 | TargetMax$ 11 | DividedAsYouChoose$ 11";
    for candidate in [
        format!("{base} | NoPrevention$ True"),
        format!("{base} | Cost$ R PayLife<2>"),
        base.replace("TargetMin$ 0", "TargetMin$ 1"),
        base.replace("ValidTgts$ Any", "ValidTgts$ Player"),
        base.replace("ValidTgts$ Any", "ValidTgts$ Opponent"),
        base.replace("DividedAsYouChoose$ 11", "DividedAsYouChoose$ 10"),
        base.replace("TargetMax$ 11", "TargetMax$ 255"),
        base.replace("TargetMax$ 11", "TargetMax$ Unknown"),
        format!(
            "{base} | SubAbility$ Bonus\nSVar:Bonus:DB$ GainLife | ValidTgts$ Player | LifeAmount$ 6"
        ),
        format!(
            "{base} | SubAbility$ Bonus\nSVar:Bonus:DB$ GainLife | ValidTgts$ Opponent | Defined$ You | LifeAmount$ 6"
        ),
        format!(
            "{base} | SubAbility$ Bonus\nSVar:Bonus:DB$ GainLife | ValidTgts$ Opponent | LifeAmount$ 6 | SubAbility$ Draw\nSVar:Draw:DB$ Draw | NumCards$ 1"
        ),
    ] {
        assert!(refused(&candidate), "{candidate}");
    }
}
