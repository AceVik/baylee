//! Read-only preparation: list fully tested cards matching a color and text filter.
#![forbid(unsafe_code)]
use baylee_cards_pool as _;

fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let colors = args
        .first()
        .map_or("WUBRG", String::as_str)
        .chars()
        .filter_map(baylee_core::color::Color::from_symbol)
        .fold(0, |bits, color| bits | color.bit());
    let terms: Vec<_> = args.iter().skip(1).map(|arg| arg.to_lowercase()).collect();
    for card in baylee_cards::all() {
        if card.coverage != baylee_cards::dsl::Coverage::Implemented
            || card.color_identity.bits() & !colors != 0
        {
            continue;
        }
        let face = &card.faces[0];
        let text = format!("{} {:?}", face.name, card.abilities).to_lowercase();
        if terms.is_empty() || terms.iter().any(|term| text.contains(term)) {
            println!("{} | {:?}", face.name, face.mana_cost);
        }
    }
}
