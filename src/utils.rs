use crate::types::Point;
use std::f32::consts::PI;

/// Linear interpolation between two f32 values
pub fn lerp(start: f32, end: f32, alpha: f32) -> f32 {
    start + (end - start) * alpha
}

/// Linear interpolation between two f64 values
pub fn lerp_f64(start: f64, end: f64, alpha: f64) -> f64 {
    start + (end - start) * alpha
}

/// Linear interpolation between two Point values
pub fn lerp_point(start: Point, end: Point, alpha: f64) -> Point {
    Point {
        x: lerp_f64(start.x, end.x, alpha),
        y: lerp_f64(start.y, end.y, alpha),
    }
}

/// Angular interpolation (handles wraparound)
/// Takes degrees, converts to radians for math, returns degrees
pub fn angle_lerp(start_deg: f64, end_deg: f64, alpha: f64) -> f64 {
    let start_rad = start_deg.to_radians();
    let end_rad = end_deg.to_radians();

    // Calculate difference, accounting for wrap around PI (-180 to 180)
    let mut diff = end_rad - start_rad;
    while diff <= -PI as f64 {
        diff += 2.0 * PI as f64;
    }
    while diff > PI as f64 {
        diff -= 2.0 * PI as f64;
    }

    let interpolated_rad = start_rad + diff * alpha;
    interpolated_rad.to_degrees().rem_euclid(360.0) // Convert back and wrap 0-360
}

/// Convert from degrees to radians
#[allow(dead_code)]
pub fn deg_to_rad(degrees: f64) -> f64 {
    degrees.to_radians()
}

/// Convert from radians to degrees
#[allow(dead_code)]
pub fn rad_to_deg(radians: f64) -> f64 {
    radians.to_degrees()
}

/// Constrain a value between min and max
#[allow(dead_code)]
pub fn clamp<T: PartialOrd>(value: T, min: T, max: T) -> T {
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_approx_eq::assert_approx_eq;

    #[test]
    fn test_lerp() {
        assert_approx_eq!(lerp(0.0, 10.0, 0.5), 5.0);
        assert_approx_eq!(lerp(0.0, 10.0, 0.0), 0.0);
        assert_approx_eq!(lerp(0.0, 10.0, 1.0), 10.0);
        assert_approx_eq!(lerp(5.0, 10.0, 0.5), 7.5);
    }

    #[test]
    fn test_lerp_f64() {
        assert_approx_eq!(lerp_f64(0.0, 10.0, 0.5), 5.0);
        assert_approx_eq!(lerp_f64(0.0, 10.0, 0.0), 0.0);
        assert_approx_eq!(lerp_f64(0.0, 10.0, 1.0), 10.0);
        assert_approx_eq!(lerp_f64(5.0, 10.0, 0.5), 7.5);
    }

    #[test]
    fn test_lerp_point() {
        let start = Point { x: 0.0, y: 0.0 };
        let end = Point { x: 10.0, y: 20.0 };
        let result = lerp_point(start, end, 0.5);
        assert_approx_eq!(result.x, 5.0);
        assert_approx_eq!(result.y, 10.0);
    }

    #[test]
    fn test_angle_lerp() {
        // Simple case
        assert_approx_eq!(angle_lerp(0.0, 90.0, 0.5), 45.0);

        // Wrapping cases - use higher epsilon due to floating point issues
        let result = angle_lerp(350.0, 10.0, 0.5);
        assert!(
            (result - 0.0).abs() < 0.01,
            "Expected approximately 0.0, got {}",
            result
        );

        // Moving clockwise vs counterclockwise should take shortest path
        let result2 = angle_lerp(0.0, 270.0, 0.5);
        assert!(
            (result2 - 315.0).abs() < 0.01,
            "Expected approximately 315.0, got {}",
            result2
        );
    }

    #[test]
    fn test_clamp() {
        assert_eq!(clamp(5, 0, 10), 5);
        assert_eq!(clamp(-5, 0, 10), 0);
        assert_eq!(clamp(15, 0, 10), 10);

        assert_approx_eq!(clamp(5.0f32, 0.0f32, 10.0f32), 5.0f32);
        assert_approx_eq!(clamp(-5.0f32, 0.0f32, 10.0f32), 0.0f32);
        assert_approx_eq!(clamp(15.0f32, 0.0f32, 10.0f32), 10.0f32);
    }
}
