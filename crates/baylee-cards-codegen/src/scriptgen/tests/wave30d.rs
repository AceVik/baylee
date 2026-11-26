#![allow(clippy::too_many_lines)]

use super::*;

fn read_w30d(text: &str) -> CardBody {
    let parsed = parse(text);
    let c = cats();
    transcode(&parsed, &c, None).unwrap_or_else(|| {
        panic!(
            "should be read in full, refused: {:?}",
            refusal_reason(&parsed, &c, None)
        )
    })
}

fn refused_w30d(text: &str) -> bool {
    let parsed = parse(text);
    let c = cats();
    transcode(&parsed, &c, None).is_none()
}

#[test]
fn wave30d_unlocks_remember_valid_prevention_and_graveyard_player_choice() {
    // 1. Mutational Advantage (`DB$ Effect | RememberObjects$ Valid Permanent.YouCtrl+HasCounters`).
    let mutational = read_w30d(
        "Name:Mutational Advantage\nManaCost:1 G U\nTypes:Instant\n\
         A:SP$ PumpAll | ValidCards$ Permanent.YouCtrl+HasCounters | KW$ Hexproof & Indestructible | SubAbility$ DBPrevent\n\
         SVar:DBPrevent:DB$ Effect | ReplacementEffects$ RPrevent | RememberObjects$ Valid Permanent.YouCtrl+HasCounters | SubAbility$ DBProliferate\n\
         SVar:RPrevent:Event$ DamageDone | Prevent$ True | ValidTarget$ Card.IsRemembered\n\
         SVar:DBProliferate:DB$ Proliferate",
    );
    assert!(
        mutational.abilities[0].contains("Effect::RepeatEach")
            && mutational.abilities[0].contains("RepeatDomain::EachMatchingPermanent(&")
            && mutational.abilities[0].contains("Effect::ModifyEvent")
            && mutational.abilities[0].contains("Modifier::PreventDamageFrom(&Filter::Any)")
            && mutational.abilities[0].contains("Duration::UntilEndOfTurn")
            && !mutational.abilities[0].contains("AddGlobalTempModifier")
            && mutational.abilities[0].contains("Effect::Proliferate"),
        "{}",
        mutational.abilities[0]
    );

    // 2. Obscura Confluence (`ValidTgts$ Player | Hidden$ True | Chooser$ Targeted | ChangeType$ Creature.TargetedPlayerOwn`).
    let obscura = read_w30d(
        "Name:Obscura Confluence\nManaCost:1 W U B\nTypes:Instant\n\
         A:SP$ Charm | Choices$ Animate,Connive,Return | CharmNum$ 3 | CanRepeatModes$ True\n\
         SVar:Animate:DB$ Animate | ValidTgts$ Creature | RemoveAllAbilities$ True | Power$ 1 | Toughness$ 1\n\
         SVar:Connive:DB$ Connive | ValidTgts$ Creature\n\
         SVar:Return:DB$ ChangeZone | ValidTgts$ Player | Hidden$ True | Mandatory$ True | ChangeType$ Creature.TargetedPlayerOwn | Chooser$ Targeted | ChangeNum$ 1 | Origin$ Graveyard | Destination$ Hand",
    );
    assert!(
        obscura.abilities[0].contains("can_repeat_modes: true")
            && obscura.abilities[0].contains("TargetSpec::AnyPlayer")
            && obscura.abilities[0].contains(
                "Effect::ChooseCard { chooser: PlayerRel::Chosen, zone: ChooseZone::Graveyard(PlayerRel::Chosen), filter: &Filter::CREATURE, min: 1, max: Amount::Fixed(1), movement: ChooseMove::Hand, then: &[] }"
            ),
        "{}",
        obscura.abilities[0]
    );

    // 3. Scrounge (`ValidTgts$ Opponent | Hidden$ True | Chooser$ Targeted | ChangeType$ Artifact.TargetedPlayerCtrl | GainControl$ True`).
    let scrounge = read_w30d(
        "Name:Scrounge\nManaCost:2 B\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Graveyard | Destination$ Battlefield | ValidTgts$ Opponent | Hidden$ True | ChangeType$ Artifact.TargetedPlayerCtrl | ChangeNum$ 1 | GainControl$ True | IsCurse$ True | Chooser$ Targeted | Mandatory$ True",
    );
    assert!(
        scrounge.abilities[0].contains("TargetSpec::AnyOpponent")
            && scrounge.abilities[0].contains(
                "Effect::ChooseCard { chooser: PlayerRel::Chosen, zone: ChooseZone::Graveyard(PlayerRel::Chosen), filter: &Filter::ARTIFACT, min: 1, max: Amount::Fixed(1), movement: ChooseMove::Battlefield { tapped: false }, then: &[] }"
            ),
        "{}",
        scrounge.abilities[0]
    );
}

#[test]
fn wave30d_keeps_unsound_refusal_boundaries_strict() {
    let phantom = read_w30d(
        "Name:PhantomShape\nManaCost:1 W\nTypes:Creature Spirit\nPT:0/0\n\
         K:etbCounter:P1P1:2\n\
         R:Event$ DamageDone | ActiveZones$ Battlefield | ValidTarget$ Card.Self | ReplaceWith$ DBRemoveCounters | PreventionEffect$ True | AlwaysReplace$ True\n\
         SVar:DBRemoveCounters:DB$ RemoveCounter | Defined$ Self | CounterType$ P1P1 | CounterNum$ 1",
    );
    assert!(
        phantom.abilities.iter().any(|a| a.contains("Modifier::PreventDamageRemovingOneCounter { kind: CounterKind::P1P1, require_counter: false }")),
        "{:?}",
        phantom.abilities
    );

    for bad in [
        // Player shroud (DSL only has hexproof, not shroud).
        "Name:IvoryMaskShape\nManaCost:2 W W\nTypes:Enchantment\n\
         S:Mode$ Continuous | Affected$ You | AddKeyword$ Shroud",
        // Player-decided may counter placement with Placer$ TriggeredPlayer (AddCounter places as controller).
        "Name:LeyLineShape\nManaCost:3 G\nTypes:Enchantment\n\
         T:Mode$ Phase | Phase$ Upkeep | TriggerZones$ Battlefield | OptionalDecider$ TriggeredPlayer | Execute$ TrigPutCounter\n\
         SVar:TrigPutCounter:DB$ PutCounter | ValidTgts$ Creature | CounterType$ P1P1 | CounterNum$ 1 | TargetingPlayer$ TriggeredPlayer | Placer$ TriggeredPlayer",
        // Fixed-2 counter removal damage prevention remains refused (only fixed-1 is modeled).
        "Name:PhantomTwoShape\nManaCost:1 W\nTypes:Creature Spirit\nPT:0/0\n\
         K:etbCounter:P1P1:2\n\
         R:Event$ DamageDone | ActiveZones$ Battlefield | ValidTarget$ Card.Self | ReplaceWith$ DBRemoveCounters | PreventionEffect$ True | AlwaysReplace$ True\n\
         SVar:DBRemoveCounters:DB$ RemoveCounter | Defined$ Self | CounterType$ P1P1 | CounterNum$ 2",
        // Graveyard-to-battlefield conditional ETB counter based on battlefield entry types (CR 614.12 lookahead).
        "Name:RecommissionShape\nManaCost:1 W\nTypes:Sorcery\n\
         A:SP$ Effect | RememberObjects$ Targeted & Self | ReplacementEffects$ ETBCreat | ExileOnMoved$ Graveyard,Stack | SubAbility$ DBReturn\n\
         SVar:DBReturn:DB$ ChangeZone | Origin$ Graveyard | Destination$ Battlefield | ValidTgts$ Artifact.YouCtrl+cmcLE3,Creature.YouCtrl+cmcLE3\n\
         SVar:ETBCreat:Event$ Moved | ValidCard$ Creature.IsRemembered | Destination$ Battlefield | ReplaceWith$ DBPutP1P1 | ReplacementResult$ Updated\n\
         SVar:DBPutP1P1:DB$ PutCounter | Defined$ ReplacedCard | CounterType$ P1P1 | ETB$ True | CounterNum$ 1",
        // keyword CARDNAME "must be blocked if able" (not Lure MustBeBlockedByAllAble).
        "Name:LureCheckShape\nManaCost:2 G\nTypes:Creature\nPT:3/3\n\
         K:CARDNAME must be blocked if able.",
        // RememberObjects$ Valid with non-prevent ReplaceWith must stay refused.
        "Name:BadValidEffect\nManaCost:2 G\nTypes:Instant\n\
         A:SP$ Effect | ReplacementEffects$ R\n\
         SVar:R:Event$ DamageDone | ReplaceWith$ X | ValidTarget$ Card.IsRemembered\n\
         SVar:X:DB$ GainLife | LifeAmount$ 1",
        // RememberObjects$ Valid Card.Self on a spell must stay refused (no Filter::This on spell).
        "Name:BadValidSelfEffect\nManaCost:1 W\nTypes:Instant\n\
         A:SP$ Effect | ReplacementEffects$ R | RememberObjects$ Valid Card.Self\n\
         SVar:R:Event$ DamageDone | Prevent$ True | ValidTarget$ Card.IsRemembered",
        // RememberObjects$ Valid Triggered.YouCtrl must stay refused (contextual trigger reference).
        "Name:BadValidTriggeredEffect\nManaCost:1 W\nTypes:Instant\n\
         A:SP$ Effect | ReplacementEffects$ R | RememberObjects$ Valid Triggered.YouCtrl\n\
         SVar:R:Event$ DamageDone | Prevent$ True | ValidTarget$ Card.IsRemembered",
        // Player graveyard ChangeZone without DefinedPlayer and without Chooser$ Targeted stays refused.
        "Name:BadPlayerGYNoChooser\nManaCost:2 U\nTypes:Sorcery\n\
         A:SP$ ChangeZone | ValidTgts$ Player | Hidden$ True | Mandatory$ True | ChangeType$ Creature.TargetedPlayerOwn | Origin$ Graveyard | Destination$ Hand",
    ] {
        assert!(refused_w30d(bad), "must refuse:\n{bad}");
    }
}
