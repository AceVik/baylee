use super::*;

fn attach_cats() -> SubtypeCatalogs {
    let mut c = cats();
    c.artifact.push("Equipment".into());
    c.creature.push("Cat".into());
    c.enchantment.push("Aura".into());
    c.normalize();
    c
}

fn read_attach(text: &str) -> CardBody {
    let parsed = parse(text);
    let catalog = attach_cats();
    transcode(&parsed, &catalog, None).unwrap_or_else(|| {
        panic!(
            "should be read in full, refused: {:?}",
            refusal_reason(&parsed, &catalog, None)
        )
    })
}

fn refused_attach(text: &str) -> bool {
    transcode(&parse(text), &attach_cats(), None).is_none()
}

#[test]
fn unattach_target_delayed_and_enchanted_rules_and_refusals() {
    let disarm = read_attach(
        "Name:Disarm\nManaCost:U\nTypes:Instant\n\
         A:SP$ Unattach | ValidTgts$ Creature | Defined$ Valid Equipment.AttachedTo Card.targetedBy",
    );
    assert!(
        disarm.abilities[0].contains("Effect::UnattachAllEquipmentFromTarget"),
        "{:?}",
        disarm.abilities[0]
    );

    let carry_away = read_attach(
        "Name:Carry Away\nManaCost:U U\nTypes:Enchantment Aura\nK:Enchant:Equipment\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigUnattach\n\
         SVar:TrigUnattach:DB$ Unattach | Defined$ Enchanted",
    );
    assert!(
        carry_away.abilities[1].contains(
            "Effect::Unattach { target: TargetSpec::Object(&Filter::AttachedToBySource) }"
        ),
        "{:?}",
        carry_away.abilities[1]
    );

    let compleation = read_attach(
        "Name:Tamiyo's Compleation\nManaCost:3 U\nTypes:Enchantment Aura\nK:Flash\n\
         K:Enchant:Artifact,Creature,Planeswalker:artifact, creature, or planeswalker\n\
         T:Mode$ ChangesZone | ValidCard$ Card.Self | Origin$ Any | Destination$ Battlefield | Execute$ TrigTap\n\
         SVar:TrigTap:DB$ Tap | Defined$ Enchanted | SubAbility$ DBUnattach\n\
         SVar:DBUnattach:DB$ Unattach | Defined$ Enchanted | ConditionDefined$ Enchanted | ConditionPresent$ Equipment",
    );
    assert!(
        compleation.abilities[1].contains("Effect::TapAll { filter: &Filter::AttachedToBySource }")
            && compleation.abilities[1].contains("Condition::AttachedMatches(&COND2)")
            && compleation.abilities[1].contains(
                "Effect::Unattach { target: TargetSpec::Object(&Filter::AttachedToBySource) }"
            ),
        "{:?}",
        compleation.abilities[1]
    );

    for bad in [
        "Name:BadMultiUnattach\nManaCost:2 W\nTypes:Instant\nA:SP$ Tap | ValidTgts$ Creature | TargetMin$ 2 | TargetMax$ 2 | SubAbility$ DBUnattach\nSVar:DBUnattach:DB$ Unattach | Defined$ Valid Equipment.AttachedTo Card.targetedBy",
        "Name:BadSpellEnchanted\nManaCost:U\nTypes:Instant\nA:SP$ Unattach | Defined$ Enchanted",
        "Name:BadNonDelayedRem\nManaCost:U\nTypes:Instant\nA:SP$ Unattach | Defined$ DelayTriggerRememberedLKI",
    ] {
        assert!(refused_attach(bad), "must refuse:\n{bad}");
    }
}

#[test]
fn attach_chosen_then_unexpected_request_and_refusals() {
    let req = read_attach(
        "Name:Unexpected Request\nManaCost:2 R\nTypes:Sorcery\n\
         A:SP$ GainControl | ValidTgts$ Creature | LoseControl$ EOT | Untap$ True | AddKWs$ Haste | SubAbility$ DBAttach\n\
         SVar:DBAttach:DB$ Attach | Optional$ True | Choices$ Equipment.YouCtrl | Defined$ ParentTarget | RememberAttached$ True | SubAbility$ DBDelayTrig\n\
         SVar:DBDelayTrig:DB$ DelayedTrigger | Mode$ Phase | Phase$ End of Turn | ConditionDefined$ Remembered | ConditionPresent$ Card | RememberObjects$ Remembered | Execute$ TrigUnattach | SubAbility$ DBCleanup\n\
         SVar:TrigUnattach:DB$ Unattach | Defined$ DelayTriggerRememberedLKI\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        req.abilities[0].contains("Effect::AttachChosenThen { filter: &ATTACH_CHOSEN1, to: TargetSpec::Object(&Filter::CREATURE), max: Amount::Fixed(1), optional: true, then_if_attached: &[Effect::RegisterDelayedTrigger { when: DelayedTriggerWhen::AtStep { step: DelayedStep::EndStep, whose: None, this_turn_only: false, recurring: false }, condition: None, targets: None, remember: TriggerRemember::EventObject, effects: &[Effect::Unattach { target: TargetSpec::EventObject }] }] }"),
        "{:?}",
        req.abilities[0]
    );

    let bad_scythe = "Name:Magitek Scythe\nManaCost:4\nTypes:Artifact Equipment\n         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigAttach | OptionalDecider$ You\n         SVar:TrigAttach:DB$ Attach | ValidTgts$ Creature.YouCtrl | RememberAttached$ True | SubAbility$ DBPump\n         SVar:DBPump:DB$ Pump | Defined$ Targeted | ConditionDefined$ Remembered | ConditionPresent$ Card | ConditionCompare$ GE1 | KW$ First Strike & HIDDEN CARDNAME must be blocked if able. | SubAbility$ DBCleanup\n         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\nK:Equip:2";
    assert!(refused_attach(bad_scythe));
}

#[test]
fn attach_object_move_all_ltb_and_relocate_attachments() {
    let sigarda = read_attach(
        "Name:Sigarda's Aid\nManaCost:W\nTypes:Enchantment\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Equipment+YouCtrl | TriggerZones$ Battlefield | Execute$ TrigAttach | OptionalDecider$ You\n\
         SVar:TrigAttach:DB$ Attach | ValidTgts$ Creature.YouCtrl | Object$ TriggeredCardLKICopy",
    );
    assert!(
        sigarda.abilities[0].contains("Effect::MayDo { effects: &[Effect::AttachObject { object: TargetSpec::EventObject, to: TargetSpec::Object(&Filter::YOUR_CREATURE), optional: false }] }"),
        "{:?}",
        sigarda.abilities[0]
    );

    let kemba = read_attach(
        "Name:Kemba, Kha Enduring\nManaCost:1 W\nTypes:Legendary Creature Cat Cleric\nPT:2/2\n\
         T:Mode$ ChangesZone | ValidCard$ Card.Self,Cat.Other+YouCtrl | Origin$ Any | Destination$ Battlefield | TriggerZones$ Battlefield | Execute$ TrigAttach\n\
         SVar:TrigAttach:DB$ Attach | Defined$ TriggeredCardLKICopy | Object$ Targeted | ValidTgts$ Equipment.YouCtrl | TargetMin$ 0 | TargetMax$ 1",
    );
    assert!(
        kemba.abilities[0].contains("Effect::AttachObject { object: TargetSpec::Object(&TARGET2), to: TargetSpec::EventObject, optional: false }")
            && kemba.abilities[0].contains("TargetReq::up_to_one(TargetSpec::Object(&TARGET2))"),
        "{:?}",
        kemba.abilities[0]
    );

    let glamer = read_attach(
        "Name:Glamer Spinners\nManaCost:4 W U\nTypes:Creature Faerie Wizard\nPT:2/4\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigAuraAttach\n\
         SVar:TrigAuraAttach:DB$ Attach | ValidTgts$ Permanent | Object$ Valid Aura.AttachedTo Targeted | Choices$ Permanent.sharesControllerWith Targeted | Move$ True",
    );
    assert!(
        glamer.abilities[0].contains("Effect::MoveAllAttachments { attachments: &MOVE_ATT1, from: TargetSpec::Object(&Filter::Any), recipient_filter: &MOVE_HOST2, must_change_recipient: true, optional: false }"),
        "{:?}",
        glamer.abilities[0]
    );

    let rhuk = read_attach(
        "Name:Rhuk, Hexgold Nabber\nManaCost:2 R\nTypes:Legendary Creature Goblin Rebel\nPT:2/2\n\
         T:Mode$ ChangesZone | ValidCard$ Creature.equipped+YouCtrl+Other | Origin$ Battlefield | Destination$ Graveyard | Execute$ TrigAttach | TriggerZones$ Battlefield | OptionalDecider$ You\n\
         SVar:TrigAttach:DB$ Attach | Object$ AttachedTo TriggeredCardLKICopy.Equipment | Defined$ Self",
    );
    assert!(
        rhuk.abilities[0].contains("Effect::MayDo { effects: &[Effect::AttachLtbAttachments { of: TargetSpec::EventObject, filter: &ATTACH_LTB2, to: TargetSpec::ThisObject, optional: false }] }"),
        "{:?}",
        rhuk.abilities[0]
    );

    let kudzu = read_attach(
        "Name:Kudzu\nManaCost:1 G G\nTypes:Enchantment Aura\nK:Enchant:Land\n\
         T:Mode$ Taps | ValidCard$ Card.AttachedBy | Execute$ TrigDestroy\n\
         SVar:TrigDestroy:DB$ Destroy | Defined$ TriggeredCardLKICopy | SubAbility$ DBAttach\n\
         SVar:DBAttach:DB$ Attach | Object$ Self | Chooser$ TriggeredCardController | Choices$ Land | Optional$ True",
    );
    assert!(
        kudzu.abilities[1].contains("Effect::RelocateAttachment { attachment: AttachmentSource::Spec(TargetSpec::ThisObject), chooser: PlayerRel::ControllerOfEvent, recipient_filter: &Filter::LAND, must_change_recipient: false, optional: true, then_if_moved: &[] }"),
        "{:?}",
        kudzu.abilities[1]
    );

    let nettlevine = read_attach(
        "Name:Nettlevine Blight\nManaCost:4 B B\nTypes:Enchantment Aura\nK:Enchant:Creature,Land:creature or land\n\
         S:Mode$ Continuous | AffectedDefined$ AttachedBy Self | AddTrigger$ NettlevineTrig\n\
         SVar:NettlevineTrig:Mode$ Phase | Phase$ End of Turn | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ NettlevineSac\n\
         SVar:NettlevineSac:DB$ Sacrifice | SubAbility$ NettlevineAttach\n\
         SVar:NettlevineAttach:DB$ Attach | Object$ OriginalHost | Choices$ Creature.YouCtrl,Land.YouCtrl",
    );
    assert!(
        nettlevine.abilities[1].contains("Modifier::GrantTriggeredDefinition(&GRANTED_TRIGGER_")
            && nettlevine.statics.contains("Effect::RelocateAttachment { attachment: AttachmentSource::GrantedSource, chooser: PlayerRel::You, recipient_filter: &ATTACH_TO"),
        "abilities={:?}, statics={:?}",
        nettlevine.abilities,
        nettlevine.statics
    );

    assert!(refused_attach(
        "Name:BadOriginalHostOutsideGrant\nManaCost:1 R\nTypes:Instant\nA:SP$ Attach | Object$ OriginalHost | Choices$ Creature",
    ));
}
