//! Reader unit tests for as-enters and resolution-time creature-type choices.

use super::*;

fn cats_with_wall() -> SubtypeCatalogs {
    let mut c = cats();
    c.creature.push("Wall".into());
    c.creature.push("Sliver".into());
    c.normalize();
    c
}

fn read_w(text: &str) -> CardBody {
    let parsed = parse(text);
    let c = cats_with_wall();
    transcode(&parsed, &c, None).unwrap_or_else(|| {
        panic!(
            "should be read in full, refused: {:?}",
            refusal_reason(&parsed, &c, None)
        )
    })
}

fn refused_w(text: &str) -> bool {
    transcode(&parse(text), &cats_with_wall(), None).is_none()
}

#[test]
fn as_enters_choose_creature_type_enables_chosen_type_filters_and_add_chosen_subtype() {
    let anthem = read_w(
        "Name:Triumph Fixture\nTypes:Enchantment\n\
         K:ETBReplacement:Other:Pick\n\
         SVar:Pick:DB$ ChooseType | Defined$ You | Type$ Creature\n\
         S:Mode$ Continuous | Affected$ Creature.ChosenType | AddPower$ 1 | AddToughness$ 1",
    );
    assert_eq!(anthem.enter_modifiers, ["EnterModifier::ChooseSubtype"]);
    assert!(
        anthem.abilities[0].contains("Filter::MatchesChosenTypeOfSource"),
        "{:?}",
        anthem.abilities
    );

    let self_type = read_w(
        "Name:Throne Fixture\nTypes:Artifact Creature Golem\n\
         K:ETBReplacement:Other:Pick\n\
         SVar:Pick:DB$ ChooseType | Defined$ You | Type$ Creature\n\
         S:Mode$ Continuous | Affected$ Card.Self | AddType$ ChosenType",
    );
    assert_eq!(
        self_type.abilities,
        ["static_ability!(Filter::This, Modifier::AddChosenSubtype)"]
    );

    // Without a creature-type choice, `ChosenType` in a filter is refused.
    assert!(refused_w(
        "Name:Bad Filter\nTypes:Enchantment\n\
         S:Mode$ Continuous | Affected$ Creature.ChosenType | AddPower$ 1 | AddToughness$ 1"
    ));
    assert!(refused_w(
        "Name:Land Chooser With Creature Filter\nTypes:Enchantment\n\
         K:ETBReplacement:Other:Pick\n\
         SVar:Pick:DB$ ChooseType | Defined$ You | Type$ Basic Land\n\
         S:Mode$ Continuous | Affected$ Creature.ChosenType | AddPower$ 1 | AddToughness$ 1"
    ));
    // Opponent choosing is refused.
    assert!(refused_w(
        "Name:Opp Choice\nTypes:Creature\n\
         K:ETBReplacement:Other:Pick\n\
         SVar:Pick:DB$ ChooseType | Defined$ Opponent | Type$ Creature"
    ));
}

#[test]
fn resolution_choose_type_reads_animate_self_target_enchanted_additive_indefinite_and_all() {
    let targeted = read_w(
        "Name:Select Fixture\nTypes:Enchantment\n\
         A:AB$ ChooseType | Cost$ 1 | Defined$ You | Type$ Creature | InvalidTypes$ Wall | SubAbility$ Anim\n\
         SVar:Anim:DB$ Animate | ValidTgts$ Creature | Types$ ChosenType | RemoveCreatureTypes$ True",
    );
    assert_eq!(
        targeted.abilities,
        [
            "activated!(cost!(\"{1}\"), &[Effect::ChooseCreatureType { exclude: Some(subtypes::creature::WALL) }, Effect::BecomeChosenCreatureType { filter: &Filter::This, additive: false, duration: Duration::UntilEndOfTurn }], target = Some(TargetSpec::Object(&Filter::CREATURE)))"
        ]
    );

    let enchanted = read_w(
        "Name:Mask Fixture\nTypes:Enchantment Aura\nK:Enchant:Creature\n\
         A:AB$ ChooseType | Cost$ 1 | Type$ Creature | SubAbility$ Anim\n\
         SVar:Anim:DB$ Animate | Defined$ Enchanted | Types$ ChosenType | RemoveCreatureTypes$ True",
    );
    assert!(
        enchanted.abilities.iter().any(|a| a.contains(
            "Effect::BecomeChosenCreatureType { filter: &Filter::AttachedToBySource, additive: false, duration: Duration::UntilEndOfTurn }"
        )),
        "{:?}",
        enchanted.abilities
    );

    let granted_additive = read_w(
        "Name:Sliver Fixture\nTypes:Creature Sliver\n\
         S:Mode$ Continuous | Affected$ Sliver | AddAbility$ Give\n\
         SVar:Give:AB$ ChooseType | Cost$ 1 | Type$ Creature | SubAbility$ Anim\n\
         SVar:Anim:DB$ Animate | Types$ ChosenType",
    );
    assert!(
        granted_additive.abilities[0].contains(
            "Modifier::GrantActivated { cost: cost!(\"{1}\"), effects: &[Effect::ChooseCreatureType { exclude: None }, Effect::BecomeChosenCreatureType { filter: &Filter::This, additive: true, duration: Duration::UntilEndOfTurn }], mana_ability: false }"
        ),
        "{:?}",
        granted_additive.abilities
    );

    let indefinite_morph = read_w(
        "Name:Proteus Fixture\nTypes:Artifact Creature Construct\nK:Morph:0\n\
         T:Mode$ TurnFaceUp | ValidCard$ Card.Self | TriggerZones$ Battlefield | Execute$ Pick\n\
         SVar:Pick:DB$ ChooseType | Defined$ You | Type$ Creature | SubAbility$ Anim\n\
         SVar:Anim:DB$ Animate | Defined$ Self | Types$ ChosenType | RemoveCreatureTypes$ True | Duration$ Permanent",
    );
    assert!(
        indefinite_morph.abilities.iter().any(|a| a.contains(
            "Effect::BecomeChosenCreatureType { filter: &Filter::This, additive: false, duration: Duration::Indefinitely }"
        )),
        "{:?}",
        indefinite_morph.abilities
    );

    let all_creatures = read_w(
        "Name:Standardize Fixture\nTypes:Instant\n\
         A:SP$ ChooseType | Defined$ You | Type$ Creature | InvalidTypes$ Wall | SubAbility$ AnimAll\n\
         SVar:AnimAll:DB$ AnimateAll | ValidCards$ Creature | Types$ ChosenType | RemoveCreatureTypes$ True",
    );
    assert_eq!(
        all_creatures.abilities,
        [
            "spell!(&[Effect::ChooseCreatureType { exclude: Some(subtypes::creature::WALL) }, Effect::BecomeChosenCreatureType { filter: &Filter::CREATURE, additive: false, duration: Duration::UntilEndOfTurn }])"
        ]
    );
}

#[test]
fn resolution_choose_type_drives_sweeps_untap_dig_until_decree_soultiller_peer_pressure_and_karona()
{
    let untap = read_w(
        "Name:Chronologist Fixture\nTypes:Creature Wizard\n\
         A:AB$ ChooseType | Cost$ U Sac<1/CARDNAME> | Type$ Creature | SubAbility$ Unt\n\
         SVar:Unt:DB$ UntapAll | ValidCards$ Creature.ChosenType",
    );
    assert!(
        untap.abilities[0].contains("Effect::UntapAll"),
        "{:?}",
        untap.abilities
    );

    let dig_until = read_w(
        "Name:Shapeshifter Fixture\nTypes:Creature Shapeshifter\n\
         A:AB$ ChooseType | Cost$ 2 U U Sac<1/CARDNAME> | Type$ Creature | SubAbility$ Dig\n\
         SVar:Dig:DB$ DigUntil | Valid$ Creature.ChosenType | ValidDescription$ creature card | FoundDestination$ Battlefield | RevealedDestination$ Library | Shuffle$ True",
    );
    assert!(
        dig_until.abilities[0].contains("Effect::RevealUntilMatching")
            && dig_until.abilities[0].contains("Effect::ShuffleLibrary { who: PlayerRel::You }"),
        "{:?}",
        dig_until.abilities
    );

    let decree = read_w(
        "Name:Decree Fixture\nTypes:Instant\n\
         A:SP$ ChooseType | Defined$ You | Type$ Creature | SubAbility$ Disc\n\
         SVar:Disc:DB$ Discard | ValidTgts$ Player | Mode$ RevealDiscardAll | DiscardValid$ Creature.ChosenType | SubAbility$ Dest\n\
         SVar:Dest:DB$ DestroyAll | ValidCards$ Creature.ChosenType+TargetedPlayerCtrl | NoRegen$ True",
    );
    assert!(
        decree.abilities[0].contains("Effect::RevealHand { who: PlayerRel::Chosen }")
            && decree.abilities[0].contains("Effect::DiscardMatching")
            && decree.abilities[0].contains("Effect::DestroyAllOf { who: PlayerRel::Chosen")
            && decree.abilities[0].contains("TargetSpec::AnyPlayer"),
        "{:?}",
        decree.abilities
    );

    let soultiller = read_w(
        "Name:Soultiller Fixture\nTypes:Creature Elf\n\
         T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | ValidCard$ Card.Self | Execute$ Pick\n\
         SVar:Pick:DB$ ChooseType | Defined$ You | Type$ Creature | SubAbility$ Shuf\n\
         SVar:Shuf:DB$ ChangeZoneAll | ChangeType$ Creature.ChosenType+YouOwn | Origin$ Graveyard | Destination$ Library | Shuffle$ True",
    );
    assert!(
        soultiller.abilities[0].contains("Effect::ShuffleGraveyardMatchingIntoLibrary"),
        "{:?}",
        soultiller.abilities
    );

    let peer = read_w(
        "Name:Peer Fixture\nTypes:Sorcery\n\
         A:SP$ ChooseType | Defined$ You | Type$ Creature | SubAbility$ Gain\n\
         SVar:Gain:DB$ GainControl | AllValid$ Creature.ChosenType | NewController$ You | ConditionCheckSVar$ Mine | ConditionSVarCompare$ GTY\n\
         SVar:Mine:Count$Valid Creature.ChosenType+YouCtrl\n\
         SVar:Y:PlayerCountOther$HighestValid Creature.ChosenType+YouCtrl",
    );
    assert!(
        peer.abilities[0].contains("Condition::ControlMoreThanEachOtherPlayer")
            && peer.abilities[0].contains("Modifier::GainControl, Duration::Indefinitely"),
        "{:?}",
        peer.abilities
    );

    let karona = read_w(
        "Name:Karona Fixture\nTypes:Legendary Creature Avatar\nK:Haste\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ Player | TriggerZones$ Battlefield | Execute$ Ctrl\n\
         SVar:Ctrl:DB$ GainControl | Defined$ Self | NewController$ TriggeredPlayer | Untap$ True",
    );
    assert!(
        karona
            .abilities
            .iter()
            .any(|a| a.contains("Effect::UntapSelf")
                && a.contains(
                    "Effect::ChangeController { new_controller: PlayerRel::ActivePlayer }"
                )),
        "{:?}",
        karona.abilities
    );

    let portal = read_w(
        "Name:Portal Fixture\nTypes:Artifact\n\
         Oracle:As Portal Fixture enters, choose a creature type.\\n{3}, {T}: You may put a creature card of the chosen type from your hand onto the battlefield.\n\
         K:ETBReplacement:Other:Pick\n\
         SVar:Pick:DB$ ChooseType | Defined$ You | Type$ Creature\n\
         A:AB$ ChangeZone | Cost$ 3 T | Origin$ Hand | Destination$ Battlefield | ChangeType$ Creature.ChosenType | ChangeNum$ 1",
    );
    assert!(
        portal.abilities[0].contains("Effect::PutFromHandOntoBattlefield")
            && portal.abilities[0].contains("optional: true"),
        "{:?}",
        portal.abilities
    );
}

#[test]
fn choose_type_refuses_unknown_invalid_type_or_missing_subability() {
    assert!(refused_w(
        "Name:No Sub\nTypes:Sorcery\n\
         A:SP$ ChooseType | Defined$ You | Type$ Creature"
    ));
    assert!(refused_w(
        "Name:Unknown Exclude\nTypes:Sorcery\n\
         A:SP$ ChooseType | Defined$ You | Type$ Creature | InvalidTypes$ Unicorn\n\
         SVar:Sub:DB$ DestroyAll | ValidCards$ Creature.ChosenType"
    ));
    assert!(refused_w(
        "Name:Multi Exclude\nTypes:Sorcery\n\
         A:SP$ ChooseType | Defined$ You | Type$ Creature | InvalidTypes$ Wall,Goblin | SubAbility$ Sub\n\
         SVar:Sub:DB$ DestroyAll | ValidCards$ Creature.ChosenType"
    ));
    assert!(refused_w(
        "Name:Unclaimed Param\nTypes:Sorcery\n\
         A:SP$ ChooseType | Defined$ You | Type$ Creature | Foo$ Bar | SubAbility$ Sub\n\
         SVar:Sub:DB$ DestroyAll | ValidCards$ Creature.ChosenType"
    ));
}
