use super::*;

fn wave27b_cats() -> SubtypeCatalogs {
    let mut c = cats();
    for cr in [
        "Soldier", "Warrior", "Dwarf", "Minotaur", "Dragon", "Human", "Wizard", "Spider", "Knight",
        "Noble", "Rogue", "Warlock", "Kor",
    ] {
        c.creature.push(cr.into());
    }
    for art in ["Equipment", "Vehicle", "Treasure"] {
        c.artifact.push(art.into());
    }
    for ench in ["Aura", "Role", "Curse"] {
        c.enchantment.push(ench.into());
    }
    c.land.push("Town".into());
    c.normalize();
    c
}

fn wave27b_tokens() -> TokenLookup {
    TokenLookup::held(&[
        (
            "role_wicked",
            "Name:Wicked\nManaCost:no cost\nTypes:Enchantment Aura Role\nK:Enchant:Creature\n\
             S:Mode$ Continuous | Affected$ Creature.EnchantedBy | AddPower$ 1 | AddToughness$ 1\n\
             T:Mode$ ChangesZone | ValidCard$ Card.Self | Origin$ Battlefield | Destination$ Graveyard | Execute$ TrigDamage\n\
             SVar:TrigDamage:DB$ LoseLife | LifeAmount$ 1 | Defined$ Opponent",
        ),
        (
            "role_royal",
            "Name:Royal\nManaCost:no cost\nTypes:Enchantment Aura Role\nK:Enchant:Creature\n\
             S:Mode$ Continuous | Affected$ Creature.EnchantedBy | AddPower$ 1 | AddToughness$ 1 | AddKeyword$ Ward:1",
        ),
        (
            "role_monster",
            "Name:Monster\nManaCost:no cost\nTypes:Enchantment Aura Role\nK:Enchant:Creature\n\
             S:Mode$ Continuous | Affected$ Creature.EnchantedBy | AddPower$ 1 | AddToughness$ 1 | AddKeyword$ Trample",
        ),
        (
            "role_cursed",
            "Name:Cursed\nManaCost:no cost\nTypes:Enchantment Aura Role\nK:Enchant:Creature\n\
             S:Mode$ Continuous | Affected$ Creature.EnchantedBy | SetPower$ 1 | SetToughness$ 1",
        ),
        (
            "w_1_1_kor_soldier",
            "Name:Kor Soldier\nTypes:Creature Kor Soldier\nColors:white\nPT:1/1",
        ),
        (
            "mask",
            "Name:Mask\nColors:white\nTypes:Enchantment Aura\n\
             K:Enchant:Permanent:permanent\nK:Umbra armor",
        ),
    ])
}

fn read27b(text: &str) -> CardBody {
    let parsed = parse(text);
    let c = wave27b_cats();
    let t = wave27b_tokens();
    transcode(&parsed, &c, Some(&t)).unwrap_or_else(|| {
        panic!(
            "expected wave27b script to transcode, refused: {:?}",
            refusal_reason(&parsed, &c, Some(&t))
        )
    })
}

fn refused27b(text: &str) -> bool {
    let parsed = parse(text);
    let c = wave27b_cats();
    let t = wave27b_tokens();
    transcode(&parsed, &c, Some(&t)).is_none()
}

#[test]
fn token_attached_role_and_aura_tokens_and_refusals() {
    // Targeted Role token (Betroth the Beast / Charmed Clothier shape).
    let targeted = read27b(
        "Name:Betroth\nManaCost:W\nTypes:Sorcery\n\
         A:SP$ Token | TokenScript$ role_royal | TokenOwner$ You | AttachedTo$ Targeted | ValidTgts$ Creature.YouCtrl",
    );
    assert!(
        targeted
            .abilities
            .iter()
            .any(|a| a.contains("Effect::CreateTokenAttached") && a.contains("AttachHost::Spec(")),
        "got: {:?}",
        targeted.abilities
    );

    // Up to one target Role token (`TargetMin$ 0 | TargetMax$ 1`, Witch's Mark / Gadwick's First Duel shape).
    let up_to = read27b(
        "Name:Wicked Mark\nManaCost:1 R\nTypes:Sorcery\n\
         A:SP$ Token | TokenScript$ role_wicked | TokenOwner$ You | AttachedTo$ Targeted | ValidTgts$ Creature.YouCtrl | TargetMin$ 0 | TargetMax$ 1",
    );
    assert!(
        up_to.abilities.iter().any(|a| {
            a.contains("Effect::CreateTokenAttached") && a.contains("TargetReq::up_to_one(")
        }),
        "got: {:?}",
        up_to.abilities
    );

    // Self-attached Role token (Cursed Courtier shape).
    let self_att = read27b(
        "Name:Cursed Courtier\nManaCost:2 W\nTypes:Creature Human Noble\nPT:3/3\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigToken\n\
         SVar:TrigToken:DB$ Token | TokenScript$ role_cursed | AttachedTo$ Self",
    );
    assert!(
        self_att.abilities.iter().any(|a| {
            a.contains("Effect::CreateTokenAttached")
                && a.contains("AttachHost::Spec(TargetSpec::ThisObject)")
        }),
        "got: {:?}",
        self_att.abilities
    );

    // Event-object attached Role token (Gylwain shape).
    let event_att = read27b(
        "Name:Casting Director\nManaCost:1 G W\nTypes:Creature Human\nPT:2/3\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Creature.!token+Other+YouCtrl | Execute$ TrigToken\n\
         SVar:TrigToken:DB$ Token | TokenScript$ role_monster | TokenOwner$ You | AttachedTo$ TriggeredCardLKICopy",
    );
    assert!(
        event_att.abilities.iter().any(|a| {
            a.contains("Effect::CreateTokenAttached")
                && a.contains("AttachHost::Spec(TargetSpec::EventObject)")
        }),
        "got: {:?}",
        event_att.abilities
    );

    // Plain Aura token attached to target (Estrid Mask shape).
    let mask = read27b(
        "Name:Mask Maker\nManaCost:1 W\nTypes:Instant\n\
         A:SP$ Token | TokenScript$ mask | TokenOwner$ You | AttachedTo$ Targeted | ValidTgts$ Permanent",
    );
    assert!(
        mask.abilities
            .iter()
            .any(|a| a.contains("Effect::CreateTokenAttached")),
        "got: {:?}",
        mask.abilities
    );

    // Honest refusals: wrong owner, unsupported entry modifiers, RememberTokens$.
    assert!(refused27b(
        "Name:BadOwner\nManaCost:W\nTypes:Sorcery\n\
         A:SP$ Token | TokenScript$ role_royal | TokenOwner$ Opponent | AttachedTo$ Targeted | ValidTgts$ Creature"
    ));
    assert!(refused27b(
        "Name:BadTapped\nManaCost:W\nTypes:Sorcery\n\
         A:SP$ Token | TokenScript$ role_royal | TokenOwner$ You | TokenTapped$ True | AttachedTo$ Targeted | ValidTgts$ Creature"
    ));
    assert!(refused27b(
        "Name:BadRem\nManaCost:W\nTypes:Sorcery\n\
         A:SP$ Token | TokenScript$ role_royal | TokenOwner$ You | RememberTokens$ True | AttachedTo$ Targeted | ValidTgts$ Creature"
    ));
}

#[test]
fn put_onto_battlefield_attached_shapes_and_refusals() {
    // Dragon Breath / Dragon Fangs shape: Defined$ Self + AttachedTo$ TriggeredCardLKICopy.
    let dragon = read27b(
        "Name:Dragon Fangs\nManaCost:1 G\nTypes:Enchantment Aura\nK:Enchant:Creature\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Creature.cmcGE6 | Execute$ TrigReturn | TriggerZones$ Graveyard | OptionalDecider$ You\n\
         SVar:TrigReturn:DB$ ChangeZone | Defined$ Self | Origin$ Graveyard | Destination$ Battlefield | AttachedTo$ TriggeredCardLKICopy",
    );
    assert!(
        dragon.abilities.iter().any(|a| {
            a.contains("Effect::PutOntoBattlefieldAttached")
                && a.contains("subject: DefinedObjects::Spec(TargetSpec::ThisObject)")
                && a.contains("host: AttachHost::Spec(TargetSpec::EventObject)")
                && a.contains("owner_control: false")
        }),
        "got: {:?}",
        dragon.abilities
    );

    // Gryff's Boon / Eagle's Rescue shape: TargetOnly + Defined$ Self + AttachedTo$ ParentTarget.
    let gryff = read27b(
        "Name:Gryff's Boon\nManaCost:W\nTypes:Enchantment Aura\nK:Enchant:Creature\n\
         A:AB$ TargetOnly | Cost$ 3 W | ActivationZone$ Graveyard | ValidTgts$ Creature | SorcerySpeed$ True | SubAbility$ DBChange\n\
         SVar:DBChange:DB$ ChangeZone | Defined$ Self | Origin$ Graveyard | Destination$ Battlefield | AttachedTo$ ParentTarget",
    );
    assert!(
        gryff.abilities.iter().any(|a| {
            a.contains("Effect::PutOntoBattlefieldAttached")
                && a.contains("subject: DefinedObjects::Spec(TargetSpec::ThisObject)")
                && a.contains("host: AttachHost::Spec(TargetSpec::Object(&Filter::CREATURE))")
        }),
        "got: {:?}",
        gryff.abilities
    );

    // Iridescent Drake / Magnetic Snuffler shape: targeted graveyard Aura/Equipment -> AttachedTo$ Self.
    let drake = read27b(
        "Name:Iridescent Drake\nManaCost:3 U\nTypes:Creature\nPT:2/2\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigChangeZone\n\
         SVar:TrigChangeZone:DB$ ChangeZone | Origin$ Graveyard | Destination$ Battlefield | ValidTgts$ Aura | AttachedTo$ Self | GainControl$ True",
    );
    assert!(
        drake.abilities.iter().any(|a| {
            a.contains("Effect::PutOntoBattlefieldAttached")
                && a.contains("subject: DefinedObjects::Spec(TargetSpec::CardInGraveyard(")
                && a.contains("host: AttachHost::Spec(TargetSpec::ThisObject)")
                && a.contains("owner_control: false")
        }),
        "got: {:?}",
        drake.abilities
    );

    // Nomad Mythmaker shape: targeted graveyard Aura -> AttachedTo$ Creature.YouCtrl.
    let nomad = read27b(
        "Name:Nomad Mythmaker\nManaCost:2 W\nTypes:Creature Human\nPT:2/2\n\
         A:AB$ ChangeZone | Cost$ W T | Origin$ Graveyard | Destination$ Battlefield | ValidTgts$ Aura | GainControl$ True | AttachedTo$ Creature.YouCtrl",
    );
    assert!(
        nomad.abilities.iter().any(|a| {
            a.contains("Effect::PutOntoBattlefieldAttached")
                && a.contains("host: AttachHost::ChosenPermanent(")
        }),
        "got: {:?}",
        nomad.abilities
    );

    // Retether shape: Defined$ ValidGraveyard Aura.YouOwn | AttachedTo$ Creature.
    let retether = read27b(
        "Name:Retether\nManaCost:3 W\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Graveyard | Destination$ Battlefield | Defined$ ValidGraveyard Aura.YouOwn | AttachedTo$ Creature",
    );
    assert!(
        retether.abilities.iter().any(|a| {
            a.contains("Effect::PutOntoBattlefieldAttached")
                && a.contains("subject: DefinedObjects::AllInZone { zone: ZoneSel::GraveyardYou")
                && a.contains("host: AttachHost::ChosenPermanent(")
        }),
        "got: {:?}",
        retether.abilities
    );

    // Lynde delayed Curse return: Defined$ DelayTriggerRememberedLKI | AttachedToPlayer$ You.
    let lynde_return = read27b(
        "Name:Curse Returner\nManaCost:2 B\nTypes:Creature\nPT:2/2\n\
         T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | ValidCard$ Curse.YouOwn | Execute$ TrigDelayed\n\
         SVar:TrigDelayed:DB$ DelayedTrigger | Mode$ Phase | Phase$ End Of Turn | Execute$ TrigReturn | RememberObjects$ TriggeredNewCardLKICopy\n\
         SVar:TrigReturn:DB$ ChangeZone | Defined$ DelayTriggerRememberedLKI | Origin$ Graveyard | Destination$ Battlefield | AttachedToPlayer$ You",
    );
    assert!(
        lynde_return.abilities.iter().any(|a| {
            a.contains("Effect::PutOntoBattlefieldAttached")
                && a.contains("subject: DefinedObjects::Spec(TargetSpec::EventObject)")
                && a.contains("host: AttachHost::Spec(TargetSpec::Player(PlayerRel::You))")
                && a.contains("owner_control: true")
        }),
        "got: {:?}",
        lynde_return.abilities
    );

    // Honest refusals: Library search with AttachedTo$/AttachedToPlayer$ (Arachnus Spinner / Bitterheart Witch),
    // Transformed$ True (Radiant Grace), Alchemy Perpetual (Craving of Yeenoghu), RememberChanged$.
}

#[test]
fn put_onto_battlefield_attached_refusals() {
    assert!(refused27b(
        "Name:Arachnus Spinner\nManaCost:5 G\nTypes:Creature Spider\nPT:5/7\n\
         A:AB$ TargetOnly | Cost$ T | ValidTgts$ Creature | SubAbility$ DBChange\n\
         SVar:DBChange:DB$ ChangeZone | Hidden$ True | Origin$ Library | OriginAlternative$ Graveyard | Destination$ Battlefield | ChangeType$ Aura | AttachedTo$ ParentTarget"
    ));
    assert!(refused27b(
        "Name:Bitterheart Witch\nManaCost:4 B\nTypes:Creature Human\nPT:1/2\n\
         T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | ValidCard$ Card.Self | Execute$ TrigTarget\n\
         SVar:TrigTarget:DB$ TargetOnly | ValidTgts$ Player | SubAbility$ DBChange\n\
         SVar:DBChange:DB$ ChangeZone | Origin$ Library | Destination$ Battlefield | ChangeType$ Card.Curse | ChangeNum$ 1 | ShuffleNonMandatory$ True | AttachedToPlayer$ Targeted"
    ));
    assert!(refused27b(
        "Name:TransformedCurse\nManaCost:W\nTypes:Enchantment Aura\nK:Enchant:Creature\n\
         T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | ValidCard$ Card.EnchantedBy | Execute$ TrigCurse\n\
         SVar:TrigCurse:DB$ TargetOnly | ValidTgts$ Opponent | SubAbility$ DBChangeZone\n\
         SVar:DBChangeZone:DB$ ChangeZone | Defined$ Self | Origin$ Graveyard | Destination$ Battlefield | AttachedToPlayer$ ParentTarget | Transformed$ True"
    ));
    assert!(refused27b(
        "Name:RememberAttached\nManaCost:2 W\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Graveyard | Destination$ Battlefield | ValidTgts$ Aura.YouOwn | AttachedTo$ Creature.YouCtrl | RememberChanged$ True"
    ));
}

#[test]
fn put_onto_battlefield_attacking_shapes_and_refusals() {
    // Targeted graveyard reanimation tapped & attacking (Dauntless Avenger / Alesha shape).
    let avenger = read27b(
        "Name:Dauntless Avenger\nManaCost:2 W\nTypes:Creature Human Soldier\nPT:3/2\n\
         T:Mode$ Attacks | ValidCard$ Card.Self | Execute$ TrigChangeZone | TriggerZones$ Battlefield\n\
         SVar:TrigChangeZone:DB$ ChangeZone | Origin$ Graveyard | Destination$ Battlefield | ValidTgts$ Creature.YouCtrl+cmcLE2 | Tapped$ True | Attacking$ True",
    );
    assert!(
        avenger.abilities.iter().any(|a| {
            a.contains("Effect::PutOntoBattlefieldAttacking")
                && a.contains("subject: DefinedObjects::Spec(TargetSpec::CardInGraveyard(")
                && a.contains("tapped: true")
        }),
        "got: {:?}",
        avenger.abilities
    );

    // Self return from graveyard tapped & attacking (Warcry Phoenix / Interceptor shape).
    let phoenix = read27b(
        "Name:Warcry Phoenix\nManaCost:3 R\nTypes:Creature\nPT:2/2\n\
         T:Mode$ AttackersDeclared | ValidAttackers$ Creature | ValidAttackersAmount$ GE3 | Execute$ TrigReturn | TriggerZones$ Graveyard | AttackingPlayer$ You\n\
         SVar:TrigReturn:AB$ ChangeZone | Cost$ 2 R | Defined$ Self | Origin$ Graveyard | Destination$ Battlefield | Tapped$ True | Attacking$ True",
    );
    assert!(
        phoenix.abilities.iter().any(|a| {
            a.contains("Effect::PutOntoBattlefieldAttacking")
                && a.contains("subject: DefinedObjects::Spec(TargetSpec::ThisObject)")
                && a.contains("tapped: true")
        }),
        "got: {:?}",
        phoenix.abilities
    );

    // Hand to battlefield tapped & attacking (Preeminent Captain shape).
    let captain = read27b(
        "Name:Preeminent Captain\nManaCost:2 W\nTypes:Creature Soldier\nPT:2/2\n\
         T:Mode$ Attacks | ValidCard$ Card.Self | Execute$ TrigChange | TriggerZones$ Battlefield\n\
         SVar:TrigChange:DB$ ChangeZone | Origin$ Hand | Destination$ Battlefield | ChangeType$ Creature.Soldier+YouCtrl | Tapped$ True | Attacking$ True\n\
         Oracle:Whenever Preeminent Captain attacks, you may put a Soldier creature card from your hand onto the battlefield tapped and attacking.",
    );
    assert!(
        captain.abilities.iter().any(|a| {
            a.contains("Effect::ChooseCardEx")
                && a.contains("Effect::PutOntoBattlefieldAttacking")
                && a.contains("subject: DefinedObjects::Found")
                && a.contains("tapped: true")
        }),
        "got: {:?}",
        captain.abilities
    );

    // Honest refusals: specific defender (Kaalia / Shark Shredder), WithCountersType$ (Grim Reaper),
    // AtEOT$ (Ilharg), RememberChanged$ (Nick Fury).
    assert!(refused27b(
        "Name:Kaalia\nManaCost:1 R W B\nTypes:Creature Human\nPT:2/2\n\
         T:Mode$ Attacks | ValidCard$ Card.Self | Execute$ TrigChange | TriggerZones$ Battlefield\n\
         SVar:TrigChange:DB$ ChangeZone | Origin$ Hand | Destination$ Battlefield | ChangeType$ Creature.Dragon+YouCtrl | Tapped$ True | Attacking$ TriggeredDefender\n\
         Oracle:Whenever Kaalia attacks, you may put a Dragon creature card from your hand onto the battlefield tapped and attacking that opponent."
    ));
    assert!(refused27b(
        "Name:FinalityReanimate\nManaCost:3 B\nTypes:Creature\nPT:3/4\n\
         T:Mode$ Attacks | ValidCard$ Card.Self | Execute$ TrigChangeZone\n\
         SVar:TrigChangeZone:DB$ ChangeZone | Origin$ Graveyard | Destination$ Battlefield | ValidTgts$ Creature.YouOwn | Tapped$ True | Attacking$ True | WithCountersType$ FINALITY"
    ));
    assert!(refused27b(
        "Name:Ilharg\nManaCost:3 R R\nTypes:Creature\nPT:6/6\n\
         T:Mode$ Attacks | ValidCard$ Card.Self | Execute$ TrigChange\n\
         SVar:TrigChange:DB$ ChangeZone | Origin$ Hand | Destination$ Battlefield | ChangeType$ Creature.YouCtrl | Tapped$ True | Attacking$ True | AtEOT$ Hand\n\
         Oracle:Whenever Ilharg attacks, you may put a creature card from your hand onto the battlefield tapped and attacking."
    ));
    assert!(refused27b(
        "Name:RememberAttack\nManaCost:4 W\nTypes:Creature\nPT:4/4\n\
         T:Mode$ Attacks | ValidCard$ Card.Self | Execute$ DBChangeZone\n\
         SVar:DBChangeZone:DB$ ChangeZone | Origin$ Hand | Destination$ Battlefield | ChangeType$ Creature.YouOwn | Optional$ You | Tapped$ True | Attacking$ True | RememberChanged$ True"
    ));
}

#[test]
fn search_distinct_names_and_attach_after_and_refusals() {
    // Three Dreams.
    let three_dreams = read27b(
        "Name:Three Dreams\nManaCost:4 W\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Library | Destination$ Hand | ChangeType$ Aura | ChangeNum$ 3 | DifferentNames$ True\n\
         Oracle:Search your library for up to three Aura cards with different names, reveal them, put them into your hand, then shuffle.",
    );
    assert!(
        three_dreams.abilities.iter().any(|a| {
            a.contains("Effect::SearchLibraryDistinctNames")
                && a.contains("count: Amount::Fixed(3)")
                && a.contains("find: &Find::HAND")
        }),
        "got: {:?}",
        three_dreams.abilities
    );

    // Deathbellow War Cry (to Battlefield) & Reach the Horizon (tapped).
    let war_cry = read27b(
        "Name:Deathbellow War Cry\nManaCost:5 R R R\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Library | Destination$ Battlefield | ChangeType$ Creature.Minotaur | ChangeNum$ 4 | DifferentNames$ True\n\
         Oracle:Search your library for up to four Minotaur creature cards with different names, put them onto the battlefield, then shuffle.",
    );
    assert!(
        war_cry.abilities.iter().any(|a| {
            a.contains("Effect::SearchLibraryDistinctNames")
                && a.contains("count: Amount::Fixed(4)")
                && a.contains("find: &Find::BATTLEFIELD")
        }),
        "got: {:?}",
        war_cry.abilities
    );

    // Stonehewer Giant / Quest for the Holy Relic (`AttachAfter$` on Equipment).
    let stonehewer = read27b(
        "Name:Stonehewer Giant\nManaCost:3 W W\nTypes:Creature Warrior\nPT:4/4\n\
         A:AB$ ChangeZone | Cost$ 1 W T | Origin$ Library | Destination$ Battlefield | ChangeType$ Card.Equipment+YouOwn | ChangeNum$ 1 | AttachAfter$ Creature.YouCtrl",
    );
    assert!(
        stonehewer.abilities.iter().any(|a| {
            a.contains("Effect::CaptureAffected")
                && a.contains("Effect::SearchLibrary")
                && a.contains("Effect::MoveAttachment")
        }),
        "got: {:?}",
        stonehewer.abilities
    );

    // Honest refusals: Gifts Ungiven (`RememberChanged$ True | Shuffle$ False`), `Exactly$ True`,
    // `AttachAfter$` on Aura, `DifferentNames$` on opponent's library.
    assert!(refused27b(
        "Name:Gifts\nManaCost:3 U\nTypes:Instant\n\
         A:SP$ ChangeZone | Origin$ Library | Destination$ Library | ChangeType$ Card | ChangeNum$ 4 | RememberChanged$ True | Reveal$ True | Shuffle$ False | DifferentNames$ True"
    ));
    assert!(refused27b(
        "Name:ExactSearch\nManaCost:4 B\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Library | Destination$ Hand | ChangeType$ Creature | ChangeNum$ 2 | DifferentNames$ True | Exactly$ True"
    ));
    assert!(refused27b(
        "Name:AuraAttachAfter\nManaCost:3 W\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Library | Destination$ Battlefield | ChangeType$ Aura | ChangeNum$ 1 | AttachAfter$ Creature.YouCtrl"
    ));
    assert!(refused27b(
        "Name:OppDistinct\nManaCost:3 B\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Library | Destination$ Exile | ValidTgts$ Opponent | DefinedPlayer$ Targeted | ChangeType$ Creature | ChangeNum$ 2 | DifferentNames$ True"
    ));
}

#[test]
fn choose_card_constrained_random_and_distinct_and_refusals() {
    // Eerie Ultimatum: Graveyard -> Battlefield with DifferentNames$ True.
    let eerie = read27b(
        "Name:Eerie Ultimatum\nManaCost:W W B B B G G\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Graveyard | Destination$ Battlefield | ChangeType$ Permanent.YouOwn | ChangeNumDesc$ any number of | ChangeTypeDesc$ permanent cards with different names | DifferentNames$ True | ChangeNum$ X | Hidden$ True\n\
         SVar:X:Count$ValidGraveyard Permanent.YouOwn$DifferentCardNames",
    );
    assert!(
        eerie.abilities.iter().any(|a| {
            a.contains("Effect::ChooseCardConstrained")
                && a.contains("distinct_names: true")
                && a.contains("at_random: false")
                && a.contains("max: Amount::Fixed(255)")
        }),
        "got: {:?}",
        eerie.abilities
    );

    // Cinderheart Giant: ChooseCard AtRandom$ True -> DealDamage.
    let cinderheart = read27b(
        "Name:Cinderheart Giant\nManaCost:5 R R\nTypes:Creature\nPT:7/6\n\
         T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | ValidCard$ Card.Self | Execute$ TrigChoose\n\
         SVar:TrigChoose:DB$ ChooseCard | Defined$ You | Choices$ Creature.OppCtrl | AtRandom$ True | SubAbility$ DBDamageChosen\n\
         SVar:DBDamageChosen:DB$ DealDamage | Defined$ ChosenCard | NumDmg$ 7 | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearChosenCard$ True",
    );
    assert!(
        cinderheart.abilities.iter().any(|a| {
            a.contains("Effect::ChooseCardConstrained")
                && a.contains("at_random: true")
                && a.contains("Effect::DealDamage { amount: Amount::Fixed(7), target: TargetSpec::EventObject }")
        }),
        "got: {:?}",
        cinderheart.abilities
    );

    // Deadbridge Chant: ChooseCard Graveyard AtRandom$ True -> conditional split.
    let deadbridge = read27b(
        "Name:Deadbridge Chant\nManaCost:4 B G\nTypes:Enchantment\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | Execute$ TrigChoose | TriggerZones$ Battlefield\n\
         SVar:TrigChoose:DB$ ChooseCard | ChoiceZone$ Graveyard | AtRandom$ True | Choices$ Card.YouOwn | SubAbility$ DBChange\n\
         SVar:DBChange:DB$ ChangeZone | Origin$ Graveyard | Destination$ Battlefield | Defined$ ChosenCard | ConditionDefined$ ChosenCard | ConditionPresent$ Creature | ConditionCompare$ GE1 | SubAbility$ DBChange2\n\
         SVar:DBChange2:DB$ ChangeZone | Origin$ Graveyard | Destination$ Hand | Defined$ ChosenCard | ConditionDefined$ ChosenCard | ConditionPresent$ Creature | ConditionCompare$ EQ0",
    );
    assert!(
        deadbridge.abilities.iter().any(|a| {
            a.contains("Effect::ChooseCardConstrained")
                && a.contains("at_random: true")
                && a.contains("Effect::IfEventObjectMatches")
                && a.contains("Effect::reanimate(TargetSpec::EventObject)")
                && a.contains("Effect::GraveyardToHand { target: TargetSpec::EventObject }")
        }),
        "got: {:?}",
        deadbridge.abilities
    );

    // Last One Standing: ChooseCard AtRandom$ True -> DestroyAll nonChosenCard.
    let last_one = read27b(
        "Name:Last One Standing\nManaCost:1 B R\nTypes:Sorcery\n\
         A:SP$ ChooseCard | Defined$ You | Amount$ 1 | Choices$ Creature | AtRandom$ True | SubAbility$ DBDestroyAll\n\
         SVar:DBDestroyAll:DB$ DestroyAll | ValidCards$ Creature.nonChosenCard",
    );
    assert!(
        last_one.abilities.iter().any(|a| {
            a.contains("Effect::ChooseCardConstrained")
                && a.contains("at_random: true")
                && a.contains("Effect::DestroyAll")
        }),
        "got: {:?}",
        last_one.abilities
    );

    // Honest refusals: Alchemy Perpetual, AllCards$ True, IncludeSpellsOnStack$, ChoiceZone$ Library,
    // RememberChanged$ True on random ChangeZone.
    assert!(refused27b(
        "Name:Consuming Oni\nManaCost:2 B B\nTypes:Creature\nPT:6/6\n\
         T:Mode$ Phase | Phase$ End of Turn | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ TrigRandom\n\
         SVar:TrigRandom:DB$ ChooseCard | Choices$ Card.nonLand+YouOwn | ChoiceZone$ Hand | AtRandom$ True | Amount$ 1 | SubAbility$ DBPump\n\
         SVar:DBPump:DB$ Pump | Defined$ ChosenCard | PumpZone$ Hand | NumAtt$ +1 | Duration$ Perpetual"
    ));
    assert!(refused27b(
        "Name:AllCardsBad\nManaCost:5\nTypes:Artifact\n\
         A:AB$ ChooseCard | Cost$ 3 T | Choices$ Creature | AtRandom$ True | AllCards$ True | SubAbility$ DBDestroy\n\
         SVar:DBDestroy:DB$ Destroy | Defined$ ChosenCard"
    ));
    assert!(refused27b(
        "Name:LibRandomBad\nManaCost:4 U\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Library | Destination$ Graveyard | ChangeType$ Instant.YouOwn | AtRandom$ True | Hidden$ True | NoShuffle$ True"
    ));
    assert!(refused27b(
        "Name:CapriciousBad\nManaCost:3 R R R\nTypes:Creature\nPT:4/4\n\
         A:SP$ ChangeZone | Origin$ Graveyard | Destination$ Exile | AtRandom$ True | Hidden$ True | ChangeType$ Card.YouOwn | ChangeNum$ 3 | RememberChanged$ True"
    ));
    assert!(refused27b(
        "Name:UnscopedGraveyardBad\nManaCost:2 B\nTypes:Sorcery\n\
         A:SP$ ChangeZone | ChangeType$ Creature | ChangeNum$ 1 | Hidden$ True | Origin$ Graveyard | Destination$ Battlefield | AtRandom$ True | GainControl$ True"
    ));
}

#[test]
fn attach_chosen_and_token_then_attach_and_refusals() {
    // Resolute Strike: conditional optional AttachChosen onto ParentTarget.
    let resolute = read27b(
        "Name:Resolute Strike\nManaCost:W\nTypes:Instant\n\
         A:SP$ Pump | ValidTgts$ Creature | NumAtt$ +2 | NumDef$ +2 | SubAbility$ DBAttach\n\
         SVar:DBAttach:DB$ Attach | ConditionDefined$ ParentTarget | ConditionPresent$ Warrior | ConditionCompare$ EQ1 | Choices$ Equipment.YouCtrl | ChoiceTitle$ Choose an Equipment you control | Optional$ True | Defined$ ParentTarget",
    );
    assert!(
        resolute
            .abilities
            .iter()
            .any(|a| { a.contains("Effect::AttachChosen") && a.contains("optional: true") }),
        "got: {:?}",
        resolute.abilities
    );

    // Nahiri, the Lithomancer [+2]: CreateToken + RememberTokens -> optional AttachChosen.
    let nahiri = read27b(
        "Name:Nahiri Test\nManaCost:3 W W\nTypes:Sorcery\n\
         A:SP$ Token | TokenScript$ w_1_1_kor_soldier | TokenOwner$ You | RememberTokens$ True | SubAbility$ DBAttach\n\
         SVar:DBAttach:DB$ Attach | Optional$ True | Choices$ Equipment.YouCtrl | ChoiceTitle$ Choose an Equipment you control | Defined$ Remembered | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        nahiri.abilities.iter().any(|a| {
            a.contains("Effect::CaptureAffected")
                && a.contains("Effect::ForEachRemembered")
                && a.contains("Effect::AttachChosen")
                && a.contains("to: TargetSpec::EventObject")
                && a.contains("optional: true")
        }),
        "got: {:?}",
        nahiri.abilities
    );

    // Beatrix, Loyal General: Object$ Valid Equipment.YouCtrl | Defined$ Targeted | Optional$ True -> max 255.
    let beatrix = read27b(
        "Name:Beatrix\nManaCost:4 W W\nTypes:Creature Human Soldier\nPT:4/4\n\
         T:Mode$ Phase | Phase$ BeginCombat | ValidPlayer$ You | Execute$ TrigAttach | TriggerZones$ Battlefield | OptionalDecider$ You\n\
         SVar:TrigAttach:DB$ Attach | Defined$ Targeted | ValidTgts$ Creature.YouCtrl | Object$ Valid Equipment.YouCtrl | Optional$ True",
    );
    assert!(
        beatrix.abilities.iter().any(|a| {
            a.contains("Effect::AttachChosen")
                && a.contains("max: Amount::Fixed(255)")
                && a.contains("optional: true")
        }),
        "got: {:?}",
        beatrix.abilities
    );

    // Battlefield Improvisation: Defined$ ParentTarget.attacking | Object$ Valid Equipment.YouCtrl.
    let improv = read27b(
        "Name:Battlefield Improvisation\nManaCost:1 W\nTypes:Instant\n\
         A:SP$ Pump | ValidTgts$ Creature | NumAtt$ +2 | NumDef$ +2 | SubAbility$ DBAttach\n\
         SVar:DBAttach:DB$ Attach | Defined$ ParentTarget.attacking | Object$ Valid Equipment.YouCtrl | Optional$ True",
    );
    assert!(
        improv.abilities.iter().any(|a| {
            a.contains("Effect::IfTargetMatches")
                && a.contains("Effect::AttachChosen")
                && a.contains("max: Amount::Fixed(255)")
        }),
        "got: {:?}",
        improv.abilities
    );

    // Armed and Armored: Object$ Valid Equipment.YouCtrl | Defined$ Valid Dwarf.YouCtrl.
    let armed = read27b(
        "Name:Armed and Armored\nManaCost:1 W\nTypes:Instant\n\
         A:SP$ Attach | Object$ Valid Equipment.YouCtrl | Defined$ Valid Dwarf.YouCtrl | Optional$ True",
    );
    assert!(
        armed.abilities.iter().any(|a| {
            a.contains("Effect::ChooseCard")
                && a.contains("Effect::AttachChosen")
                && a.contains("to: TargetSpec::EventObject")
                && a.contains("max: Amount::Fixed(255)")
        }),
        "got: {:?}",
        armed.abilities
    );

    // Honest refusals: RememberAttached$, PlayerChoices$, Move$ True without valid host.
    assert!(refused27b(
        "Name:RemAttach\nManaCost:1 W\nTypes:Instant\n\
         A:SP$ Attach | ValidTgts$ Creature | Defined$ Targeted | Choices$ Equipment.YouCtrl | Optional$ True | RememberAttached$ True"
    ));
    assert!(refused27b(
        "Name:PlayerChoiceAttach\nManaCost:2 B\nTypes:Enchantment Aura\nK:Enchant:Player\n\
         A:AB$ Attach | Cost$ 1 | Object$ Self | PlayerChoices$ Opponent"
    ));
}

#[test]
fn discard_defined_shapes_and_refusals() {
    // Monomania: ChooseCard in Hand -> Discard Mode$ Defined unchosen cards.
    let monomania = read27b(
        "Name:Monomania\nManaCost:3 B B\nTypes:Sorcery\n\
         A:SP$ ChooseCard | ValidTgts$ Player | Mandatory$ True | Choices$ Card | SubAbility$ DBDiscard | ControlledByPlayer$ Chooser | ChoiceZone$ Hand\n\
         SVar:DBDiscard:DB$ Discard | Defined$ Targeted | Mode$ Defined | DefinedCards$ ValidHand Card.!ChosenCard+OwnedBy Targeted",
    );
    assert!(
        monomania.abilities.iter().any(|a| {
            a.contains("Effect::ChooseCardEx")
                && a.contains(
                    "for_unchosen: &[Effect::DiscardDefined { cards: DefinedObjects::Found }]",
                )
        }),
        "got: {:?}",
        monomania.abilities
    );

    // Direct Mode$ Defined | DefinedCards$ ValidHand Card.nonLand+YouOwn.
    let defined_hand = read27b(
        "Name:Purge Nonlands\nManaCost:1 B\nTypes:Sorcery\n\
         A:SP$ Discard | Mode$ Defined | Defined$ You | DefinedCards$ ValidHand Card.nonLand+YouOwn",
    );
    assert!(
        defined_hand.abilities.iter().any(|a| {
            a.contains("Effect::DiscardDefined")
                && a.contains("DefinedObjects::AllInZone { zone: ZoneSel::HandYou")
        }),
        "got: {:?}",
        defined_hand.abilities
    );

    // Honest refusals: Remembered / Imprinted / stray ChosenCard / unknown key.
    assert!(refused27b(
        "Name:BadRemDiscard\nManaCost:1 B\nTypes:Sorcery\n\
         A:SP$ Discard | Mode$ Defined | Defined$ You | DefinedCards$ Remembered"
    ));
    assert!(refused27b(
        "Name:BadImpDiscard\nManaCost:1 B\nTypes:Sorcery\n\
         A:SP$ Discard | Mode$ Defined | Defined$ You | DefinedCards$ Imprinted"
    ));
    assert!(refused27b(
        "Name:BadStrayChosenDiscard\nManaCost:1 B\nTypes:Sorcery\n\
         A:SP$ Discard | Mode$ Defined | Defined$ You | DefinedCards$ ChosenCard"
    ));
}
