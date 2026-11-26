use super::*;

fn check(label: &str, text: &str) -> CardBody {
    let parsed = parse(text);
    transcode(&parsed, &cats(), Some(&tokens())).unwrap_or_else(|| {
        panic!(
            "{label} refused: {:?}",
            refusal_reason(&parsed, &cats(), Some(&tokens()))
        )
    })
}

#[test]
fn mandatory_graveyard_return_without_mandatory_flag() {
    let body = check(
        "tomb_fortress_proxy",
        "Name:Tomb Fortress Proxy\nTypes:Land Cave\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigReturn\n\
         SVar:TrigReturn:DB$ ChangeZone | Origin$ Graveyard | Destination$ Battlefield | Hidden$ True | ChangeType$ Creature.YouCtrl\n\
         Oracle:When Tomb Fortress Proxy enters, return a creature card from your graveyard to the battlefield.",
    );
    assert!(
        body.abilities[0].contains("Effect::ChooseFromYourGraveyard"),
        "{}",
        body.abilities[0]
    );
    assert!(
        body.abilities[0].contains("optional: false"),
        "{}",
        body.abilities[0]
    );
}

#[test]
fn conditional_search_library_threshold_and_flashback() {
    let body = check(
        "increasing_ambition_proxy",
        "Name:Increasing Ambition Proxy\nManaCost:4 B\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Library | Destination$ Hand | ChangeType$ Card | ChangeNum$ SearchCount | Mandatory$ True\n\
         SVar:SearchCount:Count$wasCastFromGraveyard.2.1\n\
         K:Flashback:7 B\n\
         Oracle:Search your library for a card and put that card into your hand. If this spell was cast from a graveyard, instead search your library for two cards and put those cards into your hand.",
    );
    assert!(
        body.abilities[0].contains("Condition::SourceCastFromZone(ZoneRef::Graveyard)"),
        "{}",
        body.abilities[0]
    );
    assert!(
        body.abilities[0].contains("Effect::SearchLibrary"),
        "{}",
        body.abilities[0]
    );
    assert!(
        body.abilities[0].contains("optional: false"),
        "{}",
        body.abilities[0]
    );

    let threshold_body = check(
        "threshold_search_proxy",
        "Name:Threshold Search Proxy\nManaCost:1 W\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Library | Destination$ Hand | ChangeType$ Land.Basic | ChangeNum$ SearchCount\n\
         SVar:SearchCount:Count$Threshold.2.1\n\
         Oracle:Search your library for a basic land card and put it into your hand. Threshold — If seven or more cards are in your graveyard, instead search your library for up to two basic land cards and put them into your hand.",
    );
    assert!(
        threshold_body.abilities[0].contains("Condition::GraveyardCountAtLeast(7)"),
        "{}",
        threshold_body.abilities[0]
    );
    assert!(
        threshold_body.abilities[0].contains("optional: true"),
        "{}",
        threshold_body.abilities[0]
    );
}

#[test]
fn roll_dice_beneath_top_library_without_origin() {
    let body = check(
        "underdark_rift_proxy",
        "Name:Underdark Rift Proxy\nTypes:Land\n\
         A:AB$ TargetOnly | Cost$ 5 T Exile<1/CARDNAME> | ValidTgts$ Artifact,Creature,Planeswalker | IsCurse$ True | SubAbility$ DBRollDice\n\
         SVar:DBRollDice:DB$ RollDice | Sides$ 10 | ResultSVar$ X | SubAbility$ DBReturn\n\
         SVar:DBReturn:DB$ ChangeZone | Defined$ Targeted | Destination$ Library | LibraryPosition$ X",
    );
    assert!(
        body.abilities[0].contains("Effect::PutTargetIntoLibraryBeneathTop"),
        "{}",
        body.abilities[0]
    );
}

#[test]
fn discarded_trigger_graveyard_to_exile_counters_and_linked_bag() {
    let hit_body = check(
        "discard_hit",
        "Name:Discard Hit Proxy\nManaCost:1 B\nTypes:Creature Skeleton\nPT:1/3\n\
         T:Mode$ Discarded | ValidCard$ Card.OppOwn | TriggerZones$ Battlefield | Execute$ TrigExile\n\
         SVar:TrigExile:DB$ ChangeZone | Defined$ TriggeredCard | Origin$ Graveyard | Destination$ Exile | WithCountersType$ HIT",
    );
    assert!(
        hit_body.abilities[0].contains("Effect::ExileEventFromGraveyardWithCounters"),
        "{}",
        hit_body.abilities[0]
    );
    assert!(
        hit_body.abilities[0].contains("counters::HIT"),
        "{}",
        hit_body.abilities[0]
    );

    let bag_body = check(
        "bag_of_holding_proxy",
        "Name:Bag of Holding Proxy\nManaCost:1\nTypes:Artifact\n\
         T:Mode$ Discarded | ValidCard$ Card.YouCtrl | TriggerZones$ Battlefield | Execute$ TrigExile\n\
         SVar:TrigExile:DB$ ChangeZone | Defined$ TriggeredCard | Origin$ Graveyard | Destination$ Exile\n\
         A:AB$ ChangeZoneAll | Cost$ 4 T Sac<1/CARDNAME> | ChangeType$ Card.ExiledWithSource | Origin$ Exile | Destination$ Hand\n\
         Oracle:Whenever you discard a card, exile that card from your graveyard.\\n{4}, {T}, Sacrifice Bag of Holding Proxy: Return all cards exiled with Bag of Holding Proxy to their owner's hand.",
    );
    assert!(
        bag_body.abilities[0].contains("Effect::exile_linked"),
        "{}",
        bag_body.abilities[0]
    );
    assert!(
        bag_body.abilities[1].contains("Effect::ReturnLinkedExile"),
        "{}",
        bag_body.abilities[1]
    );
}

#[test]
fn remembered_token_imprint_counter_and_eot_pump_sacrifice() {
    let sending = check(
        "summoners_sending_proxy",
        "Name:Summoners Sending Proxy\nManaCost:1 W\nTypes:Enchantment\n\
         T:Mode$ Phase | Phase$ End of Turn | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ TrigExile | OptionalDecider$ You\n\
         SVar:TrigExile:DB$ ChangeZone | Origin$ Graveyard | Destination$ Exile | ValidTgts$ Creature | SubAbility$ DBToken | RememberChanged$ True\n\
         SVar:DBToken:DB$ Token | TokenScript$ u_1_1_wizard_flying | TokenOwner$ You | ConditionDefined$ Remembered | ConditionPresent$ Card.Creature | ImprintTokens$ True | SubAbility$ DBPutCounter\n\
         SVar:DBPutCounter:DB$ PutCounter | Defined$ Imprinted | CounterType$ P1P1 | CounterNum$ 1 | ConditionDefined$ Remembered | ConditionPresent$ Card.cmcGE4 | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True | ClearImprinted$ True",
    );
    assert!(
        sending.abilities[0].contains("Effect::CreateToken"),
        "{}",
        sending.abilities[0]
    );
    assert!(
        sending.abilities[0].contains("Effect::RepeatEachObject"),
        "{}",
        sending.abilities[0]
    );
    assert!(
        sending.abilities[0].contains("Effect::AddCounterFilter"),
        "{}",
        sending.abilities[0]
    );
    assert!(
        sending.abilities[0].contains("ThisWay::ExiledFromGraveyard"),
        "{}",
        sending.abilities[0]
    );

    let emperor = check(
        "emperor_of_bones_proxy",
        "Name:Emperor of Bones Proxy\nManaCost:1 B\nTypes:Creature Skeleton\nPT:2/2\n\
         T:Mode$ Phase | Phase$ BeginCombat | ValidPlayer$ You | Execute$ TrigExile | TriggerZones$ Battlefield\n\
         SVar:TrigExile:DB$ ChangeZone | Origin$ Graveyard | Destination$ Exile | TargetMin$ 0 | TargetMax$ 1 | ValidTgts$ Card\n\
         T:Mode$ CounterAddedOnce | ValidCard$ Card.Self | TriggerZones$ Battlefield | CounterType$ P1P1 | Execute$ TrigChangeZone\n\
         SVar:TrigChangeZone:DB$ ChangeZone | ChangeType$ Creature.ExiledWithSource | GainControl$ True | Origin$ Exile | Hidden$ True | Mandatory$ True | RememberChanged$ True | Destination$ Battlefield | WithCountersType$ FINALITY | SubAbility$ DBPump\n\
         SVar:DBPump:DB$ Pump | Defined$ Remembered | KW$ Haste | AtEOT$ Sacrifice | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         Oracle:Whenever one or more +1/+1 counters are put on Emperor of Bones Proxy, put a creature card exiled with Emperor of Bones Proxy onto the battlefield under your control with a finality counter on it. It gains haste. Sacrifice it at the beginning of the next end step.",
    );
    assert!(
        emperor.abilities[1].contains("Effect::CaptureAffected"),
        "{}",
        emperor.abilities[1]
    );
    assert!(
        emperor.abilities[1].contains("CounterKind::Finality"),
        "{}",
        emperor.abilities[1]
    );
    assert!(
        emperor.abilities[1].contains("KeywordSet::HASTE"),
        "{}",
        emperor.abilities[1]
    );
    assert!(
        emperor.abilities[1].contains("Effect::SacrificeEvent"),
        "{}",
        emperor.abilities[1]
    );
}

#[test]
fn honest_refusals_on_zones_and_remembered_boundaries() {
    let parsed_unpaired_imprint = parse(
        "Name:Unpaired ClearImprinted Proxy\nManaCost:1 W\nTypes:Enchantment\n\
         T:Mode$ Phase | Phase$ End of Turn | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ TrigExile\n\
         SVar:TrigExile:DB$ ChangeZone | Origin$ Graveyard | Destination$ Exile | ValidTgts$ Creature | SubAbility$ DBToken | RememberChanged$ True\n\
         SVar:DBToken:DB$ Token | TokenScript$ u_1_1_wizard_flying | TokenOwner$ You | ConditionDefined$ Remembered | ConditionPresent$ Card.Creature | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True | ClearImprinted$ True",
    );
    assert!(transcode(&parsed_unpaired_imprint, &cats(), Some(&tokens())).is_none());

    let parsed_stash = parse(
        "Name:Tinybones Stash Proxy\nManaCost:1 B\nTypes:Creature Skeleton Rogue\nPT:1/3\n\
         T:Mode$ Discarded | ValidCard$ Card.OppOwn | TriggerZones$ Battlefield | Execute$ TrigExile\n\
         SVar:TrigExile:DB$ ChangeZone | Defined$ TriggeredCard | Origin$ Graveyard | Destination$ Exile | WithCountersType$ STASH",
    );
    assert!(transcode(&parsed_stash, &cats(), Some(&tokens())).is_none());
    assert_eq!(
        refusal_reason(&parsed_stash, &cats(), Some(&tokens())).as_deref(),
        Some("counter `STASH`")
    );

    let parsed_deaths_oasis = parse(
        "Name:Deaths Oasis Proxy\nManaCost:W B G\nTypes:Enchantment\n\
         T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | ValidCard$ Creature.YouCtrl+nonToken | Execute$ TrigMill\n\
         SVar:TrigMill:DB$ Mill | NumCards$ 2 | SubAbility$ DBReturn\n\
         SVar:DBReturn:DB$ ChangeZone | Origin$ Graveyard | Destination$ Hand | ChangeType$ Creature.YouOwn+cmcLTY | Hidden$ True | Optional$ True\n\
         SVar:Y:TriggeredCard$CardManaValue/Minus.1",
    );
    assert!(transcode(&parsed_deaths_oasis, &cats(), Some(&tokens())).is_none());

    let parsed_dollmaker = parse(
        "Name:Dedicated Dollmaker Proxy\nManaCost:1 W\nTypes:Artifact Creature Artificer\nPT:1/1\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigExile\n\
         SVar:TrigExile:DB$ ChangeZone | Origin$ Battlefield | Destination$ Exile | RememberChanged$ True | ValidTgts$ Artifact,Creature | SubAbility$ DBCopy\n\
         SVar:DBCopy:DB$ CopyPermanent | Defined$ Remembered | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(transcode(&parsed_dollmaker, &cats(), Some(&tokens())).is_none());
}
