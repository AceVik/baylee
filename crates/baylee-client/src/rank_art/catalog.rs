//! Only these bundled filenames can reach the asset server. Remote labels never become paths.
use bevy::{asset::io::embedded::EmbeddedAssetRegistry, prelude::*};
use std::path::Path;

#[derive(Clone, Copy)]
pub(super) struct Style {
    pub slug: &'static str,
    pub english: &'static str,
    pub divisions: u8,
    pub period: u8,
    pub metal: [u8; 3],
    pub gem: [u8; 3],
}

const fn style(
    slug: &'static str,
    english: &'static str,
    divisions: u8,
    period: u8,
    metal: [u8; 3],
    gem: [u8; 3],
) -> Style {
    Style {
        slug,
        english,
        divisions,
        period,
        metal,
        gem,
    }
}

// Palettes follow the eleven approved paintings, from folded parchment to nacre.
pub(super) const UNRANKED: u8 = 11;
pub(super) const STYLES: [Style; 12] = [
    style("papier", "paper", 0, 0, [172, 154, 119], [234, 215, 169]),
    style("holz", "wood", 3, 0, [144, 88, 49], [214, 149, 79]),
    style("eisen", "iron", 3, 10, [121, 134, 146], [186, 208, 221]),
    style("bronze", "bronze", 3, 9, [186, 116, 68], [85, 154, 138]),
    style("silber", "silver", 3, 8, [188, 204, 222], [145, 202, 237]),
    style("gold", "gold", 4, 8, [233, 169, 66], [255, 222, 130]),
    style("platin", "platinum", 4, 9, [196, 224, 222], [90, 221, 192]),
    style("diamant", "diamond", 4, 7, [157, 209, 245], [67, 183, 252]),
    style("master", "master", 0, 10, [219, 171, 99], [178, 112, 248]),
    style(
        "grandmaster",
        "grandmaster",
        0,
        9,
        [224, 158, 131],
        [236, 68, 95],
    ),
    style(
        "champion",
        "champion",
        0,
        12,
        [242, 203, 124],
        [217, 234, 243],
    ),
    style(
        "unplatziert",
        "unranked",
        0,
        0,
        [120, 133, 143],
        [155, 166, 173],
    ),
];

pub(super) fn embed(app: &mut App) -> Handle<Shader> {
    let embedded = app.world().resource::<EmbeddedAssetRegistry>();
    macro_rules! painting {
        ($track:literal, $stem:literal) => {
            embedded.insert_asset(
                bevy::asset::io::embedded::watched_path(
                    file!(),
                    concat!("../../../../art/baylee/ranks/", $track, "/", $stem, ".png"),
                ),
                Path::new(concat!("baylee_client/ranks/", $track, "/", $stem, ".png")),
                include_bytes!(concat!(
                    "../../../../art/baylee/ranks/",
                    $track,
                    "/",
                    $stem,
                    ".png"
                )),
            );
        };
    }
    macro_rules! both {
        ($($stem:literal),+ $(,)?) => {
            $(painting!("player", $stem); painting!("deck", $stem);)+
        };
    }
    both!(
        "papier",
        "holz",
        "holz-iii",
        "holz-ii",
        "holz-i",
        "eisen",
        "eisen-iii",
        "eisen-ii",
        "eisen-i",
        "bronze",
        "bronze-iii",
        "bronze-ii",
        "bronze-i",
        "silber",
        "silber-iii",
        "silber-ii",
        "silber-i",
        "gold",
        "gold-iv",
        "gold-iii",
        "gold-ii",
        "gold-i",
        "platin",
        "platin-iv",
        "platin-iii",
        "platin-ii",
        "platin-i",
        "diamant",
        "diamant-iv",
        "diamant-iii",
        "diamant-ii",
        "diamant-i",
        "master",
        "grandmaster",
        "champion",
    );
    painting!("player", "champion-3");
    painting!("player", "champion-2");
    painting!("player", "champion-1");
    embedded.insert_asset(
        bevy::asset::io::embedded::watched_path(
            file!(),
            "../../../../art/baylee/ranks/finish.wgsl",
        ),
        Path::new("baylee_client/rank_art/finish.wesl"),
        include_bytes!("../../../../art/baylee/ranks/finish.wgsl"),
    );
    app.world()
        .resource::<AssetServer>()
        .load("embedded://baylee_client/rank_art/finish.wesl")
}
