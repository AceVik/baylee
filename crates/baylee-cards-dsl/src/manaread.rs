//! Reading an ability as "n mana, of one of these colours".
//!
//! Two readers need this answer and must give the same one. A client's mana
//! planner asks it of a printed `{T}: Add {G}` so it knows what tapping a
//! Forest buys; `baylee-gamehost` asks it of the ability a Chromatic Lantern
//! *grants* a land, because the client cannot — a granted ability is not
//! printed on the card, so there is nothing in the registry to look up. Two
//! copies of the rule would be two answers, and the one that disagreed would
//! be a land the planner counts on and the engine refuses.
//!
//! The bar is deliberately high, and the reasons differ per clause. An
//! ability that costs mana to activate would make a plan recursive. An
//! ability that also does something else is one a player should decide about
//! themselves. Restricted mana is refused because what a Cavern of Souls'
//! mana may be spent on is a rules question, and answering it outside the
//! engine is exactly the guess this exists to avoid.
//!
//! **The middle clause keeps a named exception, and it is about the caller
//! and not about the ability.** It is still the default and it is still
//! true; what follows is its scope, not its reversal, and the difference
//! matters to whoever reads this next — a policy that acquires a stated
//! exception is stronger than one that quietly stops applying.
//!
//! Three parts, and all three hold at once. [`mana_shape`] **still refuses**
//! an ability that does anything besides add; nothing about the strict
//! reading moved. A label and a mana bubble call it and see exactly what
//! they saw before, so the sentence above is the whole truth **for them**.
//! And a planner calls [`mana_with_riders`], because it is the one caller
//! that can hold the same policy with a weaker reading: it *ranks* what it
//! accepts and puts a tap with something beside the mana behind every clean
//! tap it has, so it reaches one **only where nothing else can pay** —
//! which is the case in which the player would have tapped it by hand
//! anyway. That is the whole of the exception, and it buys the lands whose
//! *only* mana ability has a rider, which were not sources at all.
//!
//! One more door, and it is a different **question** rather than a further
//! exception. [`mana_bundle`] reads an ability that says "add" more than
//! once — a Karoo's `{T}: Add {W}{U}` — which every reader above refuses,
//! and refuses correctly: one triple is a claim about the whole ability, so
//! two of them would be a gap wearing an answer's shape. Nothing about that
//! clause moves. What moves is the **type** of the answer: a list of
//! colours, so nothing has to pretend two manas are one.
//!
//! What it will not read is anything still undecided — a choice, a computed
//! amount, a restriction. A bundle reaches the planner as several units each
//! of a known colour, so a source that is really a choice arriving here
//! would be counted once per colour it might have made. That is the
//! direction of failure rather than taste: over-counting strands a
//! half-tapped board mid-cast, where under-counting only makes the client
//! shy.
//!
//! Where the ranking happens is the planner's, and it stays there:
//! `manasources::priced` weighs a cost part and a rider into
//! `manaplan::Source::priced`, and the matcher reaches for a priced tap only
//! where no clean one fits the pip. Weighing them is a planner's judgement;
//! this module reports a shape and never a price.

use crate::cost::{Cost, CostPart};
use crate::effect::{Amount, Effect, ManaSource};
use baylee_core::mana::{ManaColor, ManaCost};

/// What a simple mana ability makes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SimpleMana {
    /// The colours it may make. More than one means the ability asks.
    pub colors: Vec<ManaColor>,
    /// How much, of whichever colour is chosen.
    pub amount: u8,
}

/// Whether a cost is the bare `{T}` and nothing else.
///
/// The question only a **granted** mana ability has to answer, and it is
/// about the *view* rather than about the rules. `GrantedMana` carries a
/// slot, the colours and the amount and has no field for a price, because an
/// ability that is printed on no card has nowhere else to put one. So a
/// projection that reported a priced grant would be telling a planner it may
/// tap for free, and the payment would fail at the moment it is spent —
/// which is worse than reporting nothing, since the ability is offered in
/// `LegalActions` either way and a player can still press it.
///
/// A *printed* ability needs no such guard: its cost is on the card the
/// client already has, so `{1}, {T}: Add {W}{U}` reads correctly through
/// [`simple_mana`] and must keep doing so.
#[must_use]
pub fn tap_only(cost: &Cost) -> bool {
    cost.mana == ManaCost::ZERO
        && cost
            .parts
            .iter()
            .all(|part| matches!(part, CostPart::TapSelf))
}

/// Reads a free, single-effect mana ability, or decides it is not one a
/// planner can use.
#[must_use]
pub fn simple_mana(cost: &Cost, effects: &[Effect]) -> Option<SimpleMana> {
    match mana_made(cost, effects) {
        Some((mana, false)) => Some(mana),
        // Restricted, and this is the caller that must refuse it — see the
        // module header, and [`mana_made`] for the caller that must not.
        _ => None,
    }
}

/// The same reading, restricted mana included, saying which it found.
///
/// The filter [`simple_mana`] applies on top of this is the entire difference
/// between two questions that look alike. A *planner* has to refuse restricted
/// mana: what a Cavern of Souls' mana may pay for is a rules question, and
/// answering it outside the engine is the guess this module exists to avoid.
/// A *label* must not refuse it — the button still has to say what tapping the
/// land does, and Jasmine Dragon Tea Shop shipped with its Ally ability drawn
/// as a bare "{T}" beside a "Tap for {C}", which is two offers a player cannot
/// tell apart and one of them is the reason the land is in the deck.
#[must_use]
pub fn mana_made(cost: &Cost, effects: &[Effect]) -> Option<(SimpleMana, bool)> {
    let (source, amount, restricted) = mana_shape(cost, effects)?;
    // An amount only a board can count is no label's to write either: "Tap
    // for {G}" would be a claim about *how much*, and how much is the one
    // thing this reading does not know. A caller that wants to say the
    // colours anyway takes [`mana_offer`], which reports the hole instead of
    // falling into it.
    let amount = amount?;
    let colors = match source {
        ManaSource::Fixed(color) => vec![color],
        ManaSource::Choice(colors) => colors.to_vec(),
        // All four depend on something outside the card — a commander's
        // identity, what someone else's lands can make, or the colour a
        // player named as this very permanent entered — so none of them has
        // an answer here, where there is no board to read. A caller that
        // *has* one takes [`mana_shape`] and resolves them itself.
        //
        // The chosen colour is the one of the four that is not a property of
        // the board at all but of the *object*: two Thriving Moors side by
        // side are two different answers, so even a caller holding the whole
        // game has to name which permanent it is asking about.
        ManaSource::CommanderIdentity
        | ManaSource::LandColor { .. }
        | ManaSource::Chosen
        | ManaSource::ChosenOr(_) => return None,
    };
    Some((SimpleMana { colors, amount }, restricted))
}

/// The same reading one step earlier: the ability's own words, before any
/// board is consulted.
///
/// [`mana_made`] answers "which colours", and for two of the four sources
/// there is no answer without a game — a Command Tower's colours are its
/// controller's commanders' identity (CR 903.4), an Exotic Orchard's are
/// read off somebody else's lands. Both used to end the reading, so a client
/// holding a `PlayerView` — which carries a seat's commanders, and is the
/// same thing the engine reads — had no way to ask the question it *could*
/// answer. It got no source at all, and a five-colour deck's Command Tower
/// counted for nothing in the mana plan.
///
/// This is the reading it needs, and the split is the same one the module
/// header describes: the shape is the card's, the resolution is the board's,
/// and the two are not the same job. Everything that makes the bar high —
/// a free cost, one `AddMana` and nothing beside it — is still enforced here,
/// so a caller resolving the source itself cannot slip past any of it.
///
/// **The amount is an `Option` and the hole is the point.** Harabaz Druid's
/// "add X mana of any one color, where X is the number of Allies you control"
/// is a mana ability in every other respect and has no number in it, and the
/// three readings above want three different things from that: a plan must
/// refuse it (counting a guess leaves a board half tapped), a label must say
/// the colours without claiming an amount, and a mana bubble's pip must be
/// the *last* tap offered for a colour rather than no tap at all. A reading
/// that ended at `Amount::Fixed` could serve only the first, and the card
/// then vanished from the sheet entirely.
///
/// It is [`mana_with_riders`] with one clause put back — **nothing beside
/// the mana** — and it is written that way rather than as its own walk over
/// the effects, so the two cannot drift into different answers about one
/// ability.
#[must_use]
pub fn mana_shape(cost: &Cost, effects: &[Effect]) -> Option<(ManaSource, Option<u8>, bool)> {
    let read = mana_with_riders(cost, effects)?;
    (effects.len() == 1).then_some(read)
}

/// The **colour question** an ability is about to ask, and how much it pours
/// where that is a number.
///
/// The fourth reading here, and it relaxes exactly one clause of
/// [`mana_made`]: **how much**. A plan insists on a number because it has to
/// count what it is buying; Harabaz Druid's "add X mana of any one color,
/// where X is the number of Allies you control" has no such number and still
/// asks a player which colour, which is the whole of what a mana bubble draws.
/// Counting it would be the over-count that leaves a board half tapped —
/// drawing its five pips costs nothing.
///
/// The amount comes back all the same, and it comes back **not to count
/// with**. A pip is a promise of that colour, so what a bubble needs to know
/// about the amount is only whether there *is* one: a permanent with two mana
/// taps that both make green has to put the predictable one behind the pip
/// and leave the other its sentence, and it cannot tell them apart without
/// this. Harabaz Druid under a Great Divide Guide is exactly that permanent,
/// and it is what the owner reported as its own ability having "verloren
/// gegangen" — both taps made all five colours, the pips collapsed them into
/// one, and the tap the pips did not stand for was deleted from the sheet.
/// `baylee_client_core::manaplan::Offer` is the half that acts on it.
///
/// Restricted mana is refused, and that one is worth saying out loud because
/// a bubble spends nothing and looks as though it could take it. It cannot: a
/// bubble's whole label is the pip, and a pip can say "white" but not "white,
/// and only on Ally spells". Jasmine Dragon Tea Shop prints both taps —
/// `{T}: Add {C}` beside an any-colour one restricted to Allies — and drawing
/// six indistinguishable discs for it would be the bug that card was already
/// reported for once (`a_restricted_mana_ability_says_what_it_makes`).
/// Restricted mana wants the sheet's words.
///
/// The rest is what makes any of these readings safe: a free cost, and one
/// `AddMana` and nothing beside it. An ability that also does something else
/// is one a player should read before activating.
#[must_use]
pub fn mana_offer(cost: &Cost, effects: &[Effect]) -> Option<(ManaSource, Option<u8>)> {
    match mana_shape(cost, effects)? {
        (source, amount, false) => Some((source, amount)),
        (_, _, true) => None,
    }
}

/// What an ability's **words** say it adds, ignoring what it charges for it.
///
/// The fourth door, and the one that takes no [`Cost`] at all. Everything
/// above answers a question about the ability *as a source of mana* — can a
/// planner count on it, and for how much — so all three refuse an ability
/// that charges mana of its own (a filter land is not a source, it is a
/// trade) and an ability that does anything besides add (a painland's damage
/// is a rules event, not a detail).
///
/// A **label** asks neither question. It asks what a player is choosing
/// between, and that is the colours: a row reading `{1}, {T}` beside a row
/// reading `Tap for {C}` is two offers a player cannot tell apart, which is
/// the fault [`mana_made`]'s comment describes on a card Jasmine Dragon Tea
/// Shop happened to reach first. Mystic Gate and Yavimaya Coast reach it the
/// other two ways, and between the three of them 126 of the pool's 374
/// written rows drew their own cost where their effect belongs.
///
/// So the cost is not a parameter. It is drawn in its own column a few pixels
/// away, and repeating it is the whole defect.
///
/// Two additions in one ability answer `None` rather than the first of them:
/// a row saying "adds {G}" about an ability that adds `{G}` *and* `{U}` is
/// worse than a row saying nothing, because it is a claim rather than a gap.
#[must_use]
pub fn mana_written(effects: &[Effect]) -> Option<(ManaSource, Option<u8>, bool)> {
    let mut written = None;
    for effect in effects {
        let Effect::AddMana {
            source,
            amount,
            restriction,
            ..
        } = effect
        else {
            continue;
        };
        if written.is_some() {
            return None;
        }
        let amount = match amount {
            Amount::Fixed(amount) => Some(u8::try_from(*amount).unwrap_or(u8::MAX)),
            _ => None,
        };
        // A rider alone restricts nothing: that mana is ordinary (#232).
        written = Some((
            *source,
            amount,
            restriction.is_some_and(|restriction| restriction.restricts),
        ));
    }
    written
}

/// The **planner's** reading: a free cost and one `AddMana`, with whatever
/// else the ability does reported rather than refused.
///
/// The sixth door. Read against [`mana_shape`] it relaxes exactly one
/// clause — what else the ability does — but the honest description is the
/// other way round: it is [`mana_written`] with the free-cost clause put
/// back. The walk over the effects is that one's, and all this adds is the
/// cost, because a planner does care what a tap charges where a label does
/// not.
///
/// **What it does not relax is the count.** Two `AddMana` in one ability is
/// still `None`, from [`mana_written`] and for its reason: "adds {G}" about
/// an ability that adds `{G}` *and* `{U}` is a claim rather than a gap, and
/// a plan built on it strands a board mid-cast. That is #150 and it needs
/// `manaplan::Source` to carry bundles, not a looser reading here.
///
/// The gap this closes: an ability with something beside the mana was not a
/// source at all, so a land whose *only* mana ability has a rider counted
/// for nothing. Ancient Tomb — `{T}: Add {C}{C}`, two damage to you — was
/// invisible to the plan, and a player tapped it by hand every time.
#[must_use]
pub fn mana_with_riders(cost: &Cost, effects: &[Effect]) -> Option<(ManaSource, Option<u8>, bool)> {
    if cost.mana != ManaCost::ZERO {
        return None;
    }
    mana_written(effects)
}

/// What a tap that says "add" **more than once** produces, one colour per
/// `AddMana`, in printed order.
///
/// The sixth door relaxed what an ability may *also* do. This one relaxes how
/// many times it may say "add", and nothing else — [`mana_with_riders`] is its
/// neighbour and the single clause between them is the count. A rider is
/// therefore still allowed here, deliberately: a rider cannot change which
/// mana comes out, so refusing one would move two clauses at once and put this
/// reader nowhere in the family.
///
/// Every `AddMana` must already be decided — a plain [`ManaSource::Fixed`]
/// colour, `Amount::Fixed(1)`, no restriction, no `combination` — and the cost
/// must ask for no mana. Each of those would make the *count* wrong rather
/// than merely vague, which is the one error a player cannot undo: a source
/// counted once per colour it might have made strands a half-tapped board
/// mid-cast.
///
/// A single `AddMana` returns `None`. That is the point of a separate door
/// rather than a flag on an existing one: a caller asking "is this several
/// manas at once" gets an answer about that question alone, and the readers
/// above keep the single-mana case to themselves.
#[must_use]
pub fn mana_bundle(cost: &Cost, effects: &[Effect]) -> Option<Vec<ManaColor>> {
    if cost.mana != ManaCost::ZERO {
        return None;
    }
    let mut colors = Vec::new();
    for effect in effects {
        let Effect::AddMana {
            source,
            amount,
            combination,
            restriction,
        } = effect
        else {
            continue;
        };
        if *combination
            || restriction.is_some_and(|restriction| restriction.restricts)
            || !matches!(amount, Amount::Fixed(1))
        {
            return None;
        }
        let ManaSource::Fixed(color) = source else {
            return None;
        };
        colors.push(*color);
    }
    (colors.len() > 1).then_some(colors)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cost::CostPart;
    use crate::effect::{ManaRestriction, SpendRider};
    use crate::filter::Filter;

    fn tap() -> Cost {
        Cost {
            mana: ManaCost::ZERO,
            parts: &[CostPart::TapSelf],
        }
    }

    #[test]
    fn a_plain_tap_for_one_colour_reads_the_same_through_both_doors() {
        let effects = [Effect::mana(ManaColor::Green, 1)];
        let expected = SimpleMana {
            colors: vec![ManaColor::Green],
            amount: 1,
        };
        assert_eq!(simple_mana(&tap(), &effects), Some(expected.clone()));
        assert_eq!(mana_made(&tap(), &effects), Some((expected, false)));
    }

    /// Jasmine Dragon Tea Shop's second ability, and the whole reason the two
    /// readings exist: the planner must refuse it, the button must not.
    #[test]
    fn restricted_mana_is_refused_by_the_planner_and_read_by_the_label() {
        static ALLY: Filter = Filter::CREATURE;
        let effects = [Effect::AddMana {
            source: crate::effect::ManaSource::Choice(&[
                ManaColor::White,
                ManaColor::Blue,
                ManaColor::Black,
                ManaColor::Red,
                ManaColor::Green,
            ]),
            amount: Amount::Fixed(1),
            combination: false,
            restriction: Some(ManaRestriction {
                filter: &ALLY,
                rider: SpendRider::None,
                restricts: true,
            }),
        }];
        assert_eq!(
            simple_mana(&tap(), &effects),
            None,
            "what restricted mana may pay for is the engine's question"
        );
        let (mana, restricted) = mana_made(&tap(), &effects).expect("the label can read it");
        assert!(restricted);
        assert_eq!(mana.colors.len(), 5, "five colours to choose between");
        assert_eq!(mana.amount, 1);
    }

    /// Mana with a rider alone is ordinary mana (CR 106.6, #232): the
    /// planner counts it, and the label does not call it restricted.
    #[test]
    fn mana_with_a_rider_alone_is_read_as_ordinary_mana() {
        static SPELLS: Filter = Filter::INSTANT_OR_SORCERY;
        let effects = [
            Effect::mana(ManaColor::Colorless, 1).when_spent(&SPELLS, SpendRider::Uncounterable)
        ];
        assert!(
            simple_mana(&tap(), &effects).is_some(),
            "the planner counts it"
        );
        let (_, restricted) = mana_made(&tap(), &effects).expect("the label can read it");
        assert!(!restricted);
    }

    /// The bar stays high for everything else: an ability that costs mana
    /// would make a plan recursive, whichever door it is read through.
    #[test]
    fn an_ability_that_costs_mana_is_read_by_neither() {
        let cost = Cost {
            mana: baylee_core::mana!("{1}"),
            parts: &[CostPart::TapSelf],
        };
        let effects = [Effect::mana(ManaColor::Blue, 1)];
        assert_eq!(simple_mana(&cost, &effects), None);
        assert_eq!(mana_made(&cost, &effects), None);
        assert_eq!(mana_shape(&cost, &effects), None, "the bar is in the shape");
    }

    /// Command Tower: the two colour-answering doors have nothing to say,
    /// and the third hands the source back so a caller with a board can.
    #[test]
    fn a_commanders_identity_is_a_shape_without_being_a_colour() {
        let effects = [Effect::mana_commander_identity()];
        assert_eq!(simple_mana(&tap(), &effects), None);
        assert_eq!(mana_made(&tap(), &effects), None);
        assert_eq!(
            mana_shape(&tap(), &effects),
            Some((ManaSource::CommanderIdentity, Some(1), false)),
            "one unrestricted mana, of colours only a board can name"
        );
    }

    /// Harabaz Druid, through all four doors at once.
    ///
    /// The amount is a count of the battlefield, so there is no number to
    /// read and the two planning doors have to refuse it — but the colour
    /// question is as plain as any other, and a mana bubble asks nothing
    /// else. The `None` is what lets a *pip* prefer a tap whose pour it can
    /// name while still standing for this one where nothing else covers the
    /// colour; a reading that stopped at `Amount::Fixed` said only "not a
    /// mana ability" and took the card off the sheet.
    #[test]
    fn an_amount_only_the_board_can_count_is_a_colour_question_all_the_same() {
        static ALLIES: Filter = Filter::CREATURE;
        let every = &[
            ManaColor::White,
            ManaColor::Blue,
            ManaColor::Black,
            ManaColor::Red,
            ManaColor::Green,
        ];
        let effects = [Effect::AddMana {
            source: ManaSource::Choice(every),
            amount: Amount::CountOf {
                filter: &ALLIES,
                zone: crate::effect::ZoneSel::Battlefield,
            },
            combination: true,
            restriction: None,
        }];
        assert_eq!(
            mana_shape(&tap(), &effects),
            Some((ManaSource::Choice(every), None, false)),
            "the shape is read; the amount is the hole"
        );
        assert_eq!(
            simple_mana(&tap(), &effects),
            None,
            "a plan cannot count it"
        );
        assert_eq!(mana_made(&tap(), &effects), None, "nor can a label");
        assert_eq!(
            mana_offer(&tap(), &effects),
            Some((ManaSource::Choice(every), None)),
            "a pip asks which colour, and is told there is no number"
        );
    }

    /// And the shape is not a way around the bar: Exotic Orchard's source
    /// comes back too, so a caller that cannot resolve it has to refuse it
    /// rather than never being told it was there.
    #[test]
    fn the_shape_names_the_source_it_cannot_answer_for() {
        let effects = [Effect::AddMana {
            source: ManaSource::LandColor { mine: false },
            amount: Amount::Fixed(1),
            combination: false,
            restriction: None,
        }];
        assert_eq!(
            mana_shape(&tap(), &effects),
            Some((ManaSource::LandColor { mine: false }, Some(1), false))
        );
    }
}
