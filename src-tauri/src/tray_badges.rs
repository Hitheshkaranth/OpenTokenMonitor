//! Menu-bar badge icons: the provider logo wrapped in usage rings, one per
//! provider, laid side by side in a single strip image. A single status item
//! keeps the footprint small — separate items get pushed behind the MacBook
//! notch on a crowded menu bar.
//!
//! Mirrors the in-app `WidgetGauge`: the outer ring is the provider's primary
//! window, the inner ring its secondary one, and both use the same
//! green / amber / red scale (`arcColor` in WidgetGauge.tsx). Rendered on the
//! CPU into a small RGBA buffer, so no font or vector dependency is needed.

use std::collections::HashMap;
use std::sync::OnceLock;

use image::imageops::{self, FilterType};
use image::{Rgba, RgbaImage};

use crate::usage::models::ProviderId;

/// Rendered edge length in pixels (2× the ~16pt menu-bar icon slot).
pub const BADGE_SIZE: u32 = 44;
/// Horizontal gap between badges in the strip, in pixels.
pub const STRIP_GAP: u32 = 8;

const OUTER_RING: (f32, f32) = (18.6, 22.0); // inner/outer radius
const INNER_RING: (f32, f32) = (14.8, 17.6);
const LOGO_RADIUS: f32 = 13.4;
/// Track opacity; higher than the in-app 0.18 so the empty ring stays
/// visible on both light and dark menu bars.
const TRACK_ALPHA: f32 = 0.34;

fn logo_bytes(provider: ProviderId) -> &'static [u8] {
    match provider {
        ProviderId::Claude => include_bytes!("../../public/providers/claude-ai-icon.png"),
        ProviderId::Codex => include_bytes!("../../public/providers/chatgpt-icon.png"),
        ProviderId::Antigravity => include_bytes!("../../public/providers/antigravity-icon.png"),
    }
}

/// Logos decoded and scaled to the badge's centre disc, once per process.
fn logos() -> &'static HashMap<ProviderId, RgbaImage> {
    static LOGOS: OnceLock<HashMap<ProviderId, RgbaImage>> = OnceLock::new();
    LOGOS.get_or_init(|| {
        let side = (LOGO_RADIUS * 2.0).round() as u32;
        ProviderId::all()
            .into_iter()
            .filter_map(|p| {
                let img =
                    image::load_from_memory_with_format(logo_bytes(p), image::ImageFormat::Png)
                        .ok()?
                        .to_rgba8();
                Some((p, fit_square(&img, side)))
            })
            .collect()
    })
}

/// Scale into a `side`×`side` square, preserving aspect ratio, centred.
fn fit_square(img: &RgbaImage, side: u32) -> RgbaImage {
    let (w, h) = img.dimensions();
    let scale = side as f32 / w.max(h) as f32;
    let (nw, nh) = (
        ((w as f32 * scale).round() as u32).max(1),
        ((h as f32 * scale).round() as u32).max(1),
    );
    let resized = imageops::resize(img, nw, nh, FilterType::Lanczos3);
    let mut out = RgbaImage::new(side, side);
    imageops::overlay(
        &mut out,
        &resized,
        ((side - nw) / 2) as i64,
        ((side - nh) / 2) as i64,
    );
    out
}

/// Same thresholds as `arcColor` in WidgetGauge.tsx.
pub fn arc_color(pct: f64) -> [u8; 3] {
    if pct > 80.0 {
        [0xef, 0x44, 0x44]
    } else if pct >= 50.0 {
        [0xf5, 0x9e, 0x0b]
    } else {
        [0x22, 0xc5, 0x5e]
    }
}

/// Source-over blend of `color` at `alpha` (0..1) onto `px`.
fn blend(px: &mut Rgba<u8>, color: [u8; 3], alpha: f32) {
    if alpha <= 0.0 {
        return;
    }
    let src_a = alpha.min(1.0);
    let dst_a = px[3] as f32 / 255.0;
    let out_a = src_a + dst_a * (1.0 - src_a);
    for i in 0..3 {
        let c = (color[i] as f32 * src_a + px[i] as f32 * dst_a * (1.0 - src_a)) / out_a.max(1e-6);
        px[i] = c.round().clamp(0.0, 255.0) as u8;
    }
    px[3] = (out_a * 255.0).round() as u8;
}

/// Coverage (0..1) of a pixel at radius `r` inside the band `[inner, outer]`,
/// with a 1px anti-aliased edge.
fn band_coverage(r: f32, (inner, outer): (f32, f32)) -> f32 {
    ((r - inner + 0.5).clamp(0.0, 1.0)).min((outer - r + 0.5).clamp(0.0, 1.0))
}

/// Draw one ring: a tinted track, then the progress arc clockwise from 12 o'clock.
fn draw_ring(img: &mut RgbaImage, band: (f32, f32), pct: Option<f64>) {
    let c = BADGE_SIZE as f32 / 2.0;
    let (color, fraction) = match pct {
        Some(p) => (arc_color(p), (p.clamp(0.0, 100.0) / 100.0) as f32),
        None => ([0x94, 0xa3, 0xb8], 0.0),
    };
    let sweep = fraction * std::f32::consts::TAU;
    let mid_r = (band.0 + band.1) / 2.0;
    for (x, y, px) in img.enumerate_pixels_mut() {
        let (dx, dy) = (x as f32 + 0.5 - c, y as f32 + 0.5 - c);
        let r = (dx * dx + dy * dy).sqrt();
        let coverage = band_coverage(r, band);
        if coverage <= 0.0 {
            continue;
        }
        blend(px, color, coverage * TRACK_ALPHA);
        if fraction <= 0.0 {
            continue;
        }
        // Angle clockwise from 12 o'clock, 0..TAU.
        let angle = dx.atan2(-dy).rem_euclid(std::f32::consts::TAU);
        let arc = if fraction >= 1.0 {
            1.0
        } else {
            // Soften the arc's end over ~1px of arc length.
            ((sweep - angle) * mid_r + 0.5).clamp(0.0, 1.0)
        };
        blend(px, color, coverage * arc);
    }
}

/// Composite the logo, masked to a circle, at the badge centre.
fn draw_logo(img: &mut RgbaImage, provider: ProviderId) {
    let Some(logo) = logos().get(&provider) else {
        return;
    };
    let c = BADGE_SIZE as f32 / 2.0;
    let offset = (c - LOGO_RADIUS).round() as u32;
    for (lx, ly, src) in logo.enumerate_pixels() {
        let (x, y) = (lx + offset, ly + offset);
        if x >= BADGE_SIZE || y >= BADGE_SIZE {
            continue;
        }
        let (dx, dy) = (x as f32 + 0.5 - c, y as f32 + 0.5 - c);
        let mask = (LOGO_RADIUS - (dx * dx + dy * dy).sqrt() + 0.5).clamp(0.0, 1.0);
        let alpha = src[3] as f32 / 255.0 * mask;
        blend(img.get_pixel_mut(x, y), [src[0], src[1], src[2]], alpha);
    }
}

/// Render the badge. `secondary` is `None` for providers with one window,
/// which leaves the inner ring as an empty neutral track.
pub fn render_badge(provider: ProviderId, primary: f64, secondary: Option<f64>) -> RgbaImage {
    let mut img = RgbaImage::new(BADGE_SIZE, BADGE_SIZE);
    draw_ring(&mut img, OUTER_RING, Some(primary));
    draw_ring(&mut img, INNER_RING, secondary);
    draw_logo(&mut img, provider);
    img
}

/// Lay `badges` (provider, primary %, secondary %) out left to right in one
/// image, `BADGE_SIZE` tall.
pub fn render_strip(badges: &[(ProviderId, f64, Option<f64>)]) -> RgbaImage {
    let n = badges.len() as u32;
    let width = (n * BADGE_SIZE + n.saturating_sub(1) * STRIP_GAP).max(1);
    let mut strip = RgbaImage::new(width, BADGE_SIZE);
    for (i, (provider, primary, secondary)) in badges.iter().enumerate() {
        let badge = render_badge(*provider, *primary, *secondary);
        let x = i as u32 * (BADGE_SIZE + STRIP_GAP);
        imageops::overlay(&mut strip, &badge, x as i64, 0);
    }
    strip
}

/// Which badge a horizontal position falls on, given the strip's on-screen
/// width. Clicks in a gap snap to the nearer badge.
pub fn badge_index_at(fraction: f64, count: usize) -> Option<usize> {
    if count == 0 || !fraction.is_finite() {
        return None;
    }
    Some(((fraction.clamp(0.0, 0.999_999)) * count as f64) as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn alpha_at(img: &RgbaImage, x: u32, y: u32) -> u8 {
        img.get_pixel(x, y)[3]
    }

    /// Pixel on the outer ring's centre line at `deg` clockwise from 12 o'clock.
    fn outer_ring_px(img: &RgbaImage, deg: f32) -> Rgba<u8> {
        let c = BADGE_SIZE as f32 / 2.0;
        let r = (OUTER_RING.0 + OUTER_RING.1) / 2.0;
        let a = deg.to_radians();
        let (x, y) = (c + r * a.sin(), c - r * a.cos());
        *img.get_pixel(x as u32, y as u32)
    }

    #[test]
    fn colors_match_the_in_app_gauge() {
        assert_eq!(arc_color(20.0), [0x22, 0xc5, 0x5e]);
        assert_eq!(arc_color(50.0), [0xf5, 0x9e, 0x0b]);
        assert_eq!(arc_color(92.0), [0xef, 0x44, 0x44]);
    }

    #[test]
    fn progress_arc_covers_only_its_share() {
        let img = render_badge(ProviderId::Claude, 25.0, None);
        // Inside the first quarter: solid ring colour.
        let filled = outer_ring_px(&img, 45.0);
        assert_eq!(filled[3], 255);
        // Past it: only the faint track.
        let track = outer_ring_px(&img, 180.0);
        assert!(track[3] < 120, "track alpha {}", track[3]);
    }

    #[test]
    fn corners_stay_transparent_and_logo_is_drawn() {
        for provider in ProviderId::all() {
            let img = render_badge(provider, 60.0, Some(10.0));
            assert_eq!(img.dimensions(), (BADGE_SIZE, BADGE_SIZE));
            assert_eq!(alpha_at(&img, 0, 0), 0);
            assert_eq!(alpha_at(&img, BADGE_SIZE - 1, BADGE_SIZE - 1), 0);
            let c = BADGE_SIZE / 2;
            assert!(
                alpha_at(&img, c, c) > 0,
                "{} logo missing",
                provider.as_str()
            );
        }
    }

    #[test]
    fn full_ring_is_closed() {
        let img = render_badge(ProviderId::Codex, 100.0, None);
        for deg in [0.0, 90.0, 180.0, 270.0, 359.0] {
            assert_eq!(outer_ring_px(&img, deg)[3], 255, "gap at {deg}°");
        }
    }

    #[test]
    fn strip_places_badges_side_by_side() {
        let strip = render_strip(&[
            (ProviderId::Claude, 20.0, Some(92.0)),
            (ProviderId::Codex, 0.0, Some(13.0)),
            (ProviderId::Antigravity, 0.0, None),
        ]);
        assert_eq!(
            strip.dimensions(),
            (3 * BADGE_SIZE + 2 * STRIP_GAP, BADGE_SIZE)
        );
        let mid = BADGE_SIZE / 2;
        for i in 0..3 {
            let cx = i * (BADGE_SIZE + STRIP_GAP) + mid;
            assert!(strip.get_pixel(cx, mid)[3] > 0, "badge {i} missing");
        }
        // The gap between badges stays transparent.
        assert_eq!(strip.get_pixel(BADGE_SIZE + STRIP_GAP / 2, mid)[3], 0);
    }

    #[test]
    fn click_position_maps_to_badge() {
        assert_eq!(badge_index_at(0.1, 3), Some(0));
        assert_eq!(badge_index_at(0.5, 3), Some(1));
        assert_eq!(badge_index_at(1.0, 3), Some(2));
        assert_eq!(badge_index_at(-0.2, 3), Some(0));
        assert_eq!(badge_index_at(0.5, 0), None);
    }
}
