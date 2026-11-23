//! Confirm rendered images were actually saved by the owned client, including on
//! the passive display where macOS can temporarily withhold a drawable.
use image::GenericImageView;
use std::path::PathBuf;

pub(super) fn check_captures() {
    let ready = PathBuf::from(std::env::var_os("BAYLEE_CENTRAL_UI_READY").unwrap());
    let dir = ready.parent().unwrap();
    for frame in [
        "01-entry",
        "02-calendar",
        "03-puzzle",
        "04-solved",
        "05-reward",
        "06-equipped",
        "07-entry-resolved",
    ] {
        let path = dir.join(format!("advent-{frame}.png"));
        let decoded =
            image::open(&path).unwrap_or_else(|e| panic!("missing native frame {frame}: {e}"));
        let (width, height) = decoded.dimensions();
        assert!(width >= 800 && height >= 500, "partial frame: {frame}");
        let mut dark = 0;
        let mut light = 0;
        for y in (0..height).step_by(7) {
            for x in (0..width).step_by(7) {
                let px = decoded.get_pixel(x, y);
                let sum = px.0[..3].iter().copied().map(u16::from).sum::<u16>();
                if sum < 220 {
                    dark += 1;
                }
                if sum > 440 {
                    light += 1;
                }
            }
        }
        assert!(dark > 60 && light > 60, "no rendered interface in {frame}");
    }
}
