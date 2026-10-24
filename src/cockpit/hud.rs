//! F-16 HUD display calculations, in boresight-relative 1/650-radian units.
use crate::math::*;

/// Both altitude repeaters resolve ten feet, without filling unused digits.
pub(crate) fn altitude(feet: f64) -> String {
    ((feet / 10.0).round() as i64 * 10).to_string()
}

/// Keep the complete velocity symbol within the usable combiner field.
/// A limited symbol is crossed out so it cannot imply a false flight path.
pub(super) fn flight_path(velocity_body: V3) -> Option<(f64, f64, bool)> {
    if velocity_body.len() < 1.0 {
        return None;
    }
    let z = (-velocity_body.z).max(0.001);
    let (x, y) = (650.0 * velocity_body.x / z, -650.0 * velocity_body.y / z);
    let (cx, cy) = (clamp(x, -61.0, 61.0), clamp(y, -73.0, 88.0));
    Some((cx, cy, velocity_body.z >= 0.0 || x != cx || y != cy))
}

/// The ladder follows inertial ground track, including lateral drift.
pub(super) fn ladder_heading(velocity: V3, fallback_deg: f64) -> f64 {
    if velocity.x.hypot(velocity.z) > 1.0 {
        velocity.x.atan2(-velocity.z)
    } else {
        fallback_deg * DEG
    }
}

pub(super) fn ladder_pitch(pitch: i32) -> bool {
    pitch.abs() <= 60 || pitch % 10 == 0
}

/// Top 11 degrees, middle 13, bottom 15 (BMS Dash-1 17.14.3).
pub(super) fn aoa_bracket(alpha: f64, fpm_y: f64) -> (f64, f64, f64) {
    let y = |reference: f64| fpm_y + (reference - alpha) * DEG * 650.0;
    (y(11.0), y(13.0), y(15.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn altitude_has_ten_foot_resolution_and_preserves_sign() {
        for (input, expected) in [(0.0, "0"), (8.0, "10"), (1234.0, "1230"), (1236.0, "1240"), (9996.0, "10000"), (-124.0, "-120")] {
            assert_eq!(altitude(input), expected);
        }
    }

    #[test]
    fn fpm_flags_clipping_and_rearward_or_stationary_motion() {
        assert_eq!(flight_path(v3(0.0, 0.0, -100.0)), Some((0.0, 0.0, false)));
        assert_eq!(flight_path(v3(100.0, -100.0, -100.0)), Some((61.0, 88.0, true)));
        assert_eq!(flight_path(v3(-100.0, 100.0, -100.0)), Some((-61.0, -73.0, true)));
        assert!(flight_path(v3(0.0, 0.0, 100.0)).unwrap().2);
        assert_eq!(flight_path(V3::ZERO), None);
    }

    #[test]
    fn approach_references_cross_the_fpm_at_the_documented_aoa() {
        assert_eq!(aoa_bracket(11.0, 40.0).0, 40.0);
        assert_eq!(aoa_bracket(13.0, 40.0).1, 40.0);
        assert_eq!(aoa_bracket(15.0, 40.0).2, 40.0);
        let (top, middle, bottom) = aoa_bracket(13.0, 40.0);
        assert!(top < middle && middle < bottom);
    }

    #[test]
    fn ladder_follows_track_and_declutters_above_sixty_degrees() {
        assert!((ladder_heading(v3(100.0, 0.0, -100.0), 0.0) - 45.0 * DEG).abs() < 1e-9);
        assert_eq!(ladder_heading(V3::ZERO, 90.0), 90.0 * DEG);
        assert!(ladder_pitch(55) && ladder_pitch(60) && ladder_pitch(70));
        assert!(!ladder_pitch(65) && !ladder_pitch(-65));
        assert!(!ladder_pitch(-75));
    }
}
