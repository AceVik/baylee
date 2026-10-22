//! Illicit Auction — {3}{R}{R} — Sorcery
//! Oracle: Each player may bid life for control of target creature. You start the bidding with a bid of 0. In turn order, each player may top the high bid. The bidding ends if the high bid stands. The high bidder loses life equal to the high bid and gains control of the creature. (This effect lasts indefinitely.)
//! Set: 6ED #190 — Classic Sixth Edition | Scryfall ID: 97d68c39-cfdd-4883-9058-d648d073ae36 | Oracle ID: dea8c725-455e-4668-877a-df388fe22255
// NOT SUPPORTED: bidding life in turn order and causing the high bidder to lose life equal to the high bid and gain control; gains control of target creature indefinitely instead.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ILLICIT_AUCTION,
    oracle_id = "dea8c725-455e-4668-877a-df388fe22255",
    scryfall_id = "97d68c39-cfdd-4883-9058-d648d073ae36",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "gains control of target creature indefinitely; bidding life in turn order and causing the high bidder to lose life equal to the high bid and gain control are not supported",
    ),
    faces = &[face!(
        name = "Illicit Auction",
        mana_cost = mana!("{3}{R}{R}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[Effect::ModifyTargets {
            modifier: Modifier::GainControl,
            duration: Duration::Indefinitely,
        }],
        targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))),
    )],
);
