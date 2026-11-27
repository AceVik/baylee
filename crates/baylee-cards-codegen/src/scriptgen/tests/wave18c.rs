//! Wave 18c scriptgen tests covering all target mechanics and failure boundaries.

use super::refusal_reason;
use super::{cats, read, read_with_tokens};

fn refuses(script: &str) -> bool {
    refusal_reason(&super::parse(script), &cats(), None).is_some()
}

#[test]
fn wave18c_choose_basic_land_type_and_declare_blockers_text() {
    let mirage = read(
        "Name:Shimmering Mirage Proxy\nManaCost:1 U\nTypes:Instant\n\
         A:SP$ ChooseType | Defined$ You | Type$ Basic Land | SubAbility$ DBAnimate\n\
         SVar:DBAnimate:DB$ Animate | ValidTgts$ Land | Types$ ChosenType | RemoveLandTypes$ True | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | Defined$ You | NumCards$ 1",
    );
    assert!(mirage.abilities[0].contains("Effect::ChooseBasicLandType"));
    assert!(mirage.abilities[0].contains("Effect::BecomeChosenBasicLandType"));
    assert!(mirage.abilities[0].contains("Effect::draw(1)"));

    let fog = read(
        "Name:Fog Patch Proxy\nManaCost:1 G\nTypes:Instant\n\
         Text:Cast this spell only during the declare blockers step.\n\
         A:SP$ BecomesBlocked | Defined$ Valid Creature.attacking | ActivationPhases$ Declare Blockers",
    );
    assert!(fog.abilities[0].contains("Effect::BecomesBlocked("));

    assert!(
        crate::scriptgen::transcode(
            &super::parse(
                "Name:BadText\nManaCost:1 G\nTypes:Instant\n\
                 Text:Cast this spell only during the declare blockers step.\n\
                 A:SP$ BecomesBlocked | Defined$ Valid Creature.attacking",
            ),
            &cats(),
            None,
        )
        .is_none()
    );
}

#[test]
fn wave18c_any_player_unless_player_variants() {
    let browbeat = read(
        "Name:Browbeat Proxy\nManaCost:2 R\nTypes:Sorcery\n\
         A:SP$ Draw | NumCards$ 3 | ValidTgts$ Player | UnlessCost$ DamageYou<5> | UnlessPayer$ Player",
    );
    assert!(browbeat.abilities[0].contains("Effect::AnyPlayerMayPayCost"));
    assert!(browbeat.abilities[0].contains("AnyPlayerCost::DamageToYou(5)"));

    let pangolin = read(
        "Name:Pangolin Proxy\nManaCost:3 B B\nTypes:Creature Beast\nPT:6/5\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigSac\n\
         SVar:TrigSac:DB$ Sacrifice | UnlessCost$ Sac<2/Creature> | UnlessPayer$ Player | UnlessSwitched$ True",
    );
    assert!(pangolin.abilities[0].contains("AnyPlayerCost::Sacrifice"));
    assert!(pangolin.abilities[0].contains("count: 2"));
    assert!(pangolin.abilities[0].contains("on_payment: true"));

    let scrying = read(
        "Name:Rhystic Scrying Proxy\nManaCost:2 U U\nTypes:Sorcery\n\
         A:SP$ Draw | Defined$ You | NumCards$ 3 | SubAbility$ DBDiscard\n\
         SVar:DBDiscard:DB$ Discard | Defined$ You | NumCards$ 3 | Mode$ TgtChoose | UnlessPayer$ Player | UnlessCost$ 2 | UnlessSwitched$ True",
    );
    assert!(scrying.abilities[0].contains("Effect::AnyPlayerMayPayManaAmount"));

    let soul = read(
        "Name:Soul Strings Proxy\nManaCost:X B\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Graveyard | Destination$ Hand | TargetMin$ 2 | TargetMax$ 2 | ValidTgts$ Creature.YouCtrl | UnlessCost$ X | UnlessPayer$ Player\n\
         SVar:X:Count$xPaid",
    );
    assert!(soul.abilities[0].contains("Effect::AnyPlayerMayPayManaAmount"));
    assert!(soul.abilities[0].contains("mana: Amount::X"));
}

#[test]
#[allow(clippy::too_many_lines)]
fn wave18c_counter_control_destroy_and_turn_face_up_variants() {
    let sliver = read(
        "Name:Diffusion Sliver Proxy\nManaCost:1 U\nTypes:Creature Sliver\nPT:1/1\n\
         T:Mode$ BecomesTarget | ValidSource$ SpellAbility.OppCtrl | ValidTarget$ Creature.YouCtrl+inZoneBattlefield | TriggerZones$ Battlefield | Execute$ TrigCounter\n\
         SVar:TrigCounter:DB$ Counter | Defined$ TriggeredSourceSA | UnlessCost$ 2 | UnlessPayer$ TriggeredSourceSAController",
    );
    assert!(
        sliver.abilities[0].contains("Effect::CounterEventSpellOrAbility")
            && sliver.abilities[0].contains("player: PlayerRel::EventPlayer"),
        "{}",
        sliver.abilities[0]
    );

    let crumple = read(
        "Name:Spell Crumple Proxy\nManaCost:1 U U\nTypes:Instant\n\
         A:SP$ Counter | TargetType$ Spell | ValidTgts$ Card | Destination$ BottomOfLibrary | SubAbility$ DBChange\n\
         SVar:DBChange:DB$ ChangeZone | Origin$ Stack | Destination$ Library | LibraryPosition$ -1 | Defined$ Parent",
    );
    assert!(
        crumple.abilities[0].contains("CounterTargetSpellTo(CounterDestination::BottomOfLibrary)")
            && crumple.abilities[0].contains("Effect::PutSourceOnBottomOfLibrary"),
        "{}",
        crumple.abilities[0]
    );

    let silence = read(
        "Name:Swift Silence Proxy\nManaCost:2 W U U\nTypes:Instant\n\
         A:SP$ Counter | Defined$ ValidStack Spell.Other | RememberCountered$ True | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | NumCards$ X | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Remembered$Amount",
    );
    assert!(
        silence.abilities[0].contains("Effect::CounterAll")
            && silence.abilities[0].contains("CounterScope::Spells")
            && silence.abilities[0].contains("Amount::ThisWay(ThisWay::Countered)"),
        "{}",
        silence.abilities[0]
    );

    let plasm = read(
        "Name:Plasm Capture Proxy\nManaCost:G G U U\nTypes:Instant\n\
         A:SP$ Counter | TargetType$ Spell | RememberCounteredCMC$ True | ValidTgts$ Card | SubAbility$ DBDelTrig\n\
         SVar:DBDelTrig:DB$ DelayedTrigger | Mode$ Phase | Phase$ Main1 | ValidPlayer$ You | Execute$ AddMana | RememberNumber$ True | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:AddMana:DB$ Mana | Produced$ Combo Any | Amount$ X\n\
         SVar:X:Count$TriggerRememberAmount",
    );
    assert!(
        plasm.abilities[0].contains("Effect::DelayedManaAnyColorAtNextMain"),
        "{}",
        plasm.abilities[0]
    );

    let counterpoint = read(
        "Name:Counterpoint Proxy\nManaCost:3 U B\nTypes:Instant\n\
         A:SP$ Counter | TargetType$ Spell | ValidTgts$ Card | RememberCounteredCMC$ True | SubAbility$ DBPlay\n\
         SVar:DBPlay:DB$ Play | ValidZone$ Graveyard | Valid$ Card.YouCtrl | ValidSA$ Spell.cmcLEX | Controller$ You | Optional$ True | Amount$ 1 | WithoutManaCost$ True | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Count$RememberedNumber",
    );
    assert!(
        counterpoint
            .statics
            .contains("Filter::CmcAtMostAmount(&Amount::TargetCmc)")
            && counterpoint.abilities[0].contains("Effect::CounterTargetSpell"),
        "{}",
        counterpoint.abilities[0]
    );

    let flayer = read(
        "Name:Flayer Proxy\nManaCost:8 C C\nTypes:Creature Eldrazi\nPT:10/10\n\
         T:Mode$ SpellCast | ValidCard$ Card.Self | Execute$ TrigGC\n\
         SVar:TrigGC:DB$ GainControl | ValidTgts$ Creature | LoseControl$ EOT | Untap$ True | AddKWs$ Haste & Trample & Annihilator:2",
    );
    assert!(
        flayer.abilities[0].contains("Effect::GainControlEx")
            && flayer.abilities[0].contains("Modifier::GrantTriggered"),
        "{}",
        flayer.abilities[0]
    );

    let purse = read(
        "Name:Purse Proxy\nManaCost:2\nTypes:Artifact\n\
         A:AB$ Draw | Cost$ T | SubAbility$ DBPass\n\
         SVar:DBPass:DB$ GainControl | Defined$ Self | NewController$ NextPlayerToYourRight",
    );
    assert!(
        purse.abilities[0].contains("PlayerRel::PlayerToYourRight"),
        "{}",
        purse.abilities[0]
    );

    let deed = read(
        "Name:Pernicious Deed Proxy\nManaCost:1 B G\nTypes:Enchantment\n\
         A:AB$ DestroyAll | Cost$ X Sac<1/CARDNAME> | ValidCards$ Artifact.cmcLEX,Creature.cmcLEX,Enchantment.cmcLEX\n\
         SVar:X:Count$xPaid",
    );
    assert!(
        deed.abilities[0].contains("Effect::destroy_all(&")
            && deed.abilities[0].contains("Filter::CmcAtMostX"),
        "{}",
        deed.abilities[0]
    );

    let wake = read(
        "Name:Wake Proxy\nManaCost:3 R R R\nTypes:Sorcery\n\
         A:SP$ TargetOnly | ValidTgts$ Land | SubAbility$ DBAll\n\
         SVar:DBAll:DB$ DestroyAll | ValidCards$ TargetedCard.Self,Land.NotDefinedTargeted+sharesNameWith Targeted",
    );
    assert!(
        wake.abilities[0].contains("Effect::DestroyTargetAndSameName"),
        "{}",
        wake.abilities[0]
    );

    let mogg = read_with_tokens(
        "Name:Mogg Infestation Proxy\nManaCost:3 R R\nTypes:Sorcery\n\
         A:SP$ DestroyAll | ValidTgts$ Player | ValidCards$ Creature | ValidDescription$ all creatures targeted player controls | RememberDestroyed$ True | SubAbility$ DBTokenInfestation\n\
         SVar:DBTokenInfestation:DB$ Token | TokenAmount$ X | TokenScript$ r_1_1_goblin | TokenOwner$ Targeted\n\
         SVar:X:Remembered$Amount/Twice",
    );
    assert!(
        mogg.abilities[0].contains("Effect::DestroyAllOf { who: PlayerRel::Chosen")
            && mogg.abilities[0].contains("Effect::CreateTokensFor")
            && mogg.abilities[0].contains("Amount::Times"),
        "{}",
        mogg.abilities[0]
    );

    let induce = read(
        "Name:Induce Paranoia Proxy\nManaCost:2 U U\nTypes:Instant\n\
         A:SP$ Counter | TargetType$ Spell | ValidTgts$ Card | RememberCounteredCMC$ True | SubAbility$ DBMill\n\
         SVar:DBMill:DB$ Mill | NumCards$ X | Defined$ TargetedController | ConditionManaSpent$ B | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Count$RememberedNumber",
    );
    assert!(
        induce.abilities[0].contains("Effect::CounterTargetSpell")
            && induce.abilities[0].contains("Amount::ManaColorSpentToCast(ManaColor::Black), 1")
            && induce.abilities[0].contains("Amount::TargetCmc"),
        "{}",
        induce.abilities[0]
    );

    assert!(refuses(
        "Name:BadConditionManaSpentActivated\nManaCost:2\nTypes:Artifact\n\
         A:AB$ Draw | Cost$ T | NumCards$ 1 | ConditionManaSpent$ B",
    ));

    assert!(refuses(
        "Name:BadCounterRemNoSub\nManaCost:1 U\nTypes:Instant\n\
         A:SP$ Counter | TargetType$ Spell | ValidTgts$ Card | RememberCounteredCMC$ True",
    ));
}
