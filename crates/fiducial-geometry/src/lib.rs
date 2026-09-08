//! 2D/3D geometry primitives and tolerance profiles — `no_std`.
//!
//! Provides the geometric building blocks for the Fiducial EDA → enclosure pipeline:
//!
//! - [`Point2`] / [`Vec2`] — 2D coordinate arithmetic
//! - [`Point3`] / [`Vec3`] — 3D coordinate arithmetic
//! - [`Polygon`] — closed 2D boundary (not yet consumed by the mesh pipeline)
//! - [`BoardOutline`] — rectangular width/height + PCB thickness + tolerance class
//! - [`BoundingBox`] — axis-aligned 2D envelope
//! - [`ToleranceProfile`] — per-process manufacturing tolerances (FDM, Resin)

#![no_std]
#![deny(unsafe_code)]
#![warn(missing_docs)]

// ── 2D primitives ─────────────────────────────────────────────────────────────

/// A 2D point.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point2 {
    /// X coordinate in millimetres.
    pub x: f32,
    /// Y coordinate in millimetres.
    pub y: f32,
}

impl Point2 {
    /// Create a new 2D point.
    #[inline]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// Vector from this point to `other`.
    #[inline]
    pub fn to(self, other: Self) -> Vec2 {
        Vec2::new(other.x - self.x, other.y - self.y)
    }
}

/// A 2D vector.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec2 {
    /// X component.
    pub x: f32,
    /// Y component.
    pub y: f32,
}

impl Vec2 {
    /// Create a new 2D vector.
    #[inline]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// Squared Euclidean length.
    #[inline]
    pub fn length_sq(self) -> f32 {
        self.x * self.x + self.y * self.y
    }

    /// Euclidean length (uses `core::f32::sqrt` via libm equivalent).
    #[inline]
    pub fn length(self) -> f32 {
        // libm-free: rely on the hardware sqrt exposed by core.
        #[allow(clippy::suboptimal_flops)]
        libm::sqrtf(self.x * self.x + self.y * self.y)
    }

    /// 2D cross product (scalar z component of the 3D cross).
    #[inline]
    pub fn cross(self, other: Self) -> f32 {
        self.x * other.y - self.y * other.x
    }
}

impl core::ops::Add for Vec2 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl core::ops::Sub for Vec2 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl core::ops::Mul<f32> for Vec2 {
    type Output = Self;
    fn mul(self, s: f32) -> Self {
        Self::new(self.x * s, self.y * s)
    }
}

// ── 3D primitives ─────────────────────────────────────────────────────────────

/// A 3D point.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point3 {
    /// X coordinate in millimetres.
    pub x: f32,
    /// Y coordinate in millimetres.
    pub y: f32,
    /// Z coordinate in millimetres.
    pub z: f32,
}

impl Point3 {
    /// Create a new 3D point.
    #[inline]
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }
}

/// A 3D vector.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec3 {
    /// X component.
    pub x: f32,
    /// Y component.
    pub y: f32,
    /// Z component.
    pub z: f32,
}

impl Vec3 {
    /// Create a new 3D vector.
    #[inline]
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    /// Dot product.
    #[inline]
    pub fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    /// Cross product.
    #[inline]
    pub fn cross(self, other: Self) -> Self {
        Self::new(
            self.y * other.z - self.z * other.y,
            self.z * other.x - self.x * other.z,
            self.x * other.y - self.y * other.x,
        )
    }

    /// Squared length.
    #[inline]
    pub fn length_sq(self) -> f32 {
        self.dot(self)
    }

    /// Euclidean length.
    #[inline]
    pub fn length(self) -> f32 {
        libm::sqrtf(self.length_sq())
    }

    /// Normalised vector, or zero on degenerate input.
    #[inline]
    pub fn normalise(self) -> Self {
        let l = self.length();
        if l < 1e-9 {
            Self::new(0.0, 0.0, 0.0)
        } else {
            Self::new(self.x / l, self.y / l, self.z / l)
        }
    }
}

impl core::ops::Add for Vec3 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl core::ops::Sub for Vec3 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl core::ops::Mul<f32> for Vec3 {
    type Output = Self;
    fn mul(self, s: f32) -> Self {
        Self::new(self.x * s, self.y * s, self.z * s)
    }
}

// ── Polygon ───────────────────────────────────────────────────────────────────

/// Axis-aligned 2D bounding box.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoundingBox {
    /// Minimum corner.
    pub min: Point2,
    /// Maximum corner.
    pub max: Point2,
}

impl BoundingBox {
    /// Width (X extent) in millimetres.
    #[inline]
    pub fn width(&self) -> f32 {
        self.max.x - self.min.x
    }

    /// Height (Y extent) in millimetres.
    #[inline]
    pub fn height(&self) -> f32 {
        self.max.y - self.min.y
    }

    /// Area in mm².
    #[inline]
    pub fn area(&self) -> f32 {
        self.width() * self.height()
    }
}

/// A closed 2D polygon (board outline, cutout, etc.).
///
/// Vertices are stored in counter-clockwise order. The closing edge from last to
/// first vertex is implicit.
#[cfg(feature = "alloc")]
pub mod polygon {
    extern crate alloc;
    use alloc::vec::Vec;

    use super::{BoundingBox, Point2};

    /// A closed 2D polygon.
    pub struct Polygon {
        vertices: Vec<Point2>,
    }

    impl Polygon {
        /// Create from a list of vertices (minimum 3).
        pub fn new(vertices: Vec<Point2>) -> Option<Self> {
            if vertices.len() < 3 {
                None
            } else {
                Some(Self { vertices })
            }
        }

        /// Rectangle with the given width and height (origin at 0,0).
        pub fn rect(width: f32, height: f32) -> Self {
            Self {
                vertices: alloc::vec![
                    Point2::new(0.0, 0.0),
                    Point2::new(width, 0.0),
                    Point2::new(width, height),
                    Point2::new(0.0, height),
                ],
            }
        }

        /// Borrow the vertex list.
        pub fn vertices(&self) -> &[Point2] {
            &self.vertices
        }

        /// Axis-aligned bounding box.
        pub fn bounding_box(&self) -> BoundingBox {
            let mut min = self.vertices[0];
            let mut max = self.vertices[0];
            for v in &self.vertices[1..] {
                if v.x < min.x {
                    min.x = v.x;
                }
                if v.y < min.y {
                    min.y = v.y;
                }
                if v.x > max.x {
                    max.x = v.x;
                }
                if v.y > max.y {
                    max.y = v.y;
                }
            }
            BoundingBox { min, max }
        }

        /// Signed area (positive = CCW, negative = CW).
        pub fn signed_area(&self) -> f32 {
            let n = self.vertices.len();
            let mut sum = 0.0f32;
            for i in 0..n {
                let j = (i + 1) % n;
                sum += self.vertices[i].x * self.vertices[j].y;
                sum -= self.vertices[j].x * self.vertices[i].y;
            }
            sum * 0.5
        }
    }
}

#[cfg(feature = "alloc")]
pub use polygon::Polygon;

// ── Board outline ─────────────────────────────────────────────────────────────

/// Manufacturing tolerance class — links geometry to process capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToleranceClass {
    /// Standard FDM/FFF printing.
    Fdm,
    /// SLA / MSLA resin printing.
    Resin,
    /// CNC machining.
    Cnc,
}

/// Per-process manufacturing tolerance profile.
///
/// Values are in millimetres.
#[derive(Debug, Clone, Copy)]
pub struct ToleranceProfile {
    /// Short process identifier.
    pub name: &'static str,
    /// Human-readable description.
    pub description: &'static str,
    /// XY accuracy (±mm).
    pub xy_accuracy_mm: f32,
    /// Z (layer) accuracy (±mm).
    pub z_accuracy_mm: f32,
    /// Minimum wall thickness (mm).
    pub min_wall_thickness_mm: f32,
    /// Minimum printable feature size (mm).
    pub min_feature_mm: f32,
}

/// Standard FDM (fused deposition modelling) tolerance profile.
pub const TOLERANCE_FDM: ToleranceProfile = ToleranceProfile {
    name: "fdm",
    description: "Fused deposition modelling — standard FDM/FFF desktop printer",
    xy_accuracy_mm: 0.2,
    z_accuracy_mm: 0.1,
    min_wall_thickness_mm: 1.2,
    min_feature_mm: 0.8,
};

/// SLA/MSLA resin tolerance profile.
pub const TOLERANCE_RESIN: ToleranceProfile = ToleranceProfile {
    name: "resin",
    description: "SLA / MSLA resin printing — high detail",
    xy_accuracy_mm: 0.05,
    z_accuracy_mm: 0.025,
    min_wall_thickness_mm: 0.4,
    min_feature_mm: 0.3,
};

/// CNC machining tolerance profile.
pub const TOLERANCE_CNC: ToleranceProfile = ToleranceProfile {
    name: "cnc",
    description: "CNC milling — metal or engineering plastics",
    xy_accuracy_mm: 0.02,
    z_accuracy_mm: 0.01,
    min_wall_thickness_mm: 0.8,
    min_feature_mm: 0.5,
};

/// Look up a tolerance profile by name.
pub fn tolerance_by_name(name: &str) -> Option<&'static ToleranceProfile> {
    match name {
        "fdm" => Some(&TOLERANCE_FDM),
        "resin" => Some(&TOLERANCE_RESIN),
        "cnc" => Some(&TOLERANCE_CNC),
        _ => None,
    }
}

/// A board outline: a rectangular PCB footprint + thickness + process profile.
///
/// Currently rectangular only. The `tolerance` field is carried for downstream
/// enclosure generation, which is not yet implemented — `fiducial_mesh` reads
/// only `width_mm`, `height_mm`, and `thickness_mm`.
pub struct BoardOutline {
    /// PCB width in millimetres.
    pub width_mm: f32,
    /// PCB height in millimetres.
    pub height_mm: f32,
    /// PCB thickness in millimetres (standard FR4 = 1.6 mm).
    pub thickness_mm: f32,
    /// Manufacturing tolerance class for the generated enclosure.
    pub tolerance: ToleranceClass,
}

impl BoardOutline {
    /// Standard 1.6 mm thick PCB with FDM enclosure tolerance.
    pub const fn new(width_mm: f32, height_mm: f32) -> Self {
        Self {
            width_mm,
            height_mm,
            thickness_mm: 1.6,
            tolerance: ToleranceClass::Fdm,
        }
    }

    /// Set PCB thickness.
    pub const fn with_thickness(mut self, thickness_mm: f32) -> Self {
        self.thickness_mm = thickness_mm;
        self
    }

    /// Set manufacturing tolerance class.
    pub const fn with_tolerance(mut self, tolerance: ToleranceClass) -> Self {
        self.tolerance = tolerance;
        self
    }

    /// Active tolerance profile for this outline.
    pub fn tolerance_profile(&self) -> &'static ToleranceProfile {
        match self.tolerance {
            ToleranceClass::Fdm => &TOLERANCE_FDM,
            ToleranceClass::Resin => &TOLERANCE_RESIN,
            ToleranceClass::Cnc => &TOLERANCE_CNC,
        }
    }

    /// Bounding box of the board.
    pub fn bounding_box(&self) -> BoundingBox {
        BoundingBox {
            min: Point2::new(0.0, 0.0),
            max: Point2::new(self.width_mm, self.height_mm),
        }
    }

    /// The board boundary as a closed polygon, origin at (0, 0).
    ///
    /// Rectangular today. Downstream code (enclosure generation) sizes itself
    /// from this polygon's bounding box rather than from `width_mm`/`height_mm`
    /// directly, so a non-rectangular outline becomes a change to this method
    /// alone.
    #[cfg(feature = "alloc")]
    pub fn to_polygon(&self) -> Polygon {
        Polygon::rect(self.width_mm, self.height_mm)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point2_to_vec2() {
        let a = Point2::new(1.0, 2.0);
        let b = Point2::new(4.0, 6.0);
        let v = a.to(b);
        assert_eq!(v.x, 3.0);
        assert_eq!(v.y, 4.0);
    }

    #[test]
    fn vec2_length() {
        let v = Vec2::new(3.0, 4.0);
        assert!((v.length() - 5.0).abs() < 1e-5);
    }

    #[test]
    fn vec2_cross() {
        let a = Vec2::new(1.0, 0.0);
        let b = Vec2::new(0.0, 1.0);
        assert_eq!(a.cross(b), 1.0);
        assert_eq!(b.cross(a), -1.0);
    }

    #[test]
    fn vec3_cross_unit_axes() {
        let x = Vec3::new(1.0, 0.0, 0.0);
        let y = Vec3::new(0.0, 1.0, 0.0);
        let z = x.cross(y);
        assert!((z.x).abs() < 1e-6);
        assert!((z.y).abs() < 1e-6);
        assert!((z.z - 1.0).abs() < 1e-6);
    }

    #[test]
    fn vec3_normalise() {
        let v = Vec3::new(3.0, 0.0, 4.0).normalise();
        assert!((v.length() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn vec3_normalise_zero_is_zero() {
        let v = Vec3::new(0.0, 0.0, 0.0).normalise();
        assert_eq!(v, Vec3::new(0.0, 0.0, 0.0));
    }

    #[test]
    fn bounding_box_dimensions() {
        let bb = BoundingBox {
            min: Point2::new(1.0, 2.0),
            max: Point2::new(5.0, 8.0),
        };
        assert_eq!(bb.width(), 4.0);
        assert_eq!(bb.height(), 6.0);
        assert_eq!(bb.area(), 24.0);
    }

    #[test]
    fn board_outline_defaults() {
        let b = BoardOutline::new(100.0, 60.0);
        assert_eq!(b.width_mm, 100.0);
        assert_eq!(b.height_mm, 60.0);
        assert_eq!(b.thickness_mm, 1.6);
        assert!(matches!(b.tolerance, ToleranceClass::Fdm));
    }

    #[test]
    fn board_outline_bounding_box() {
        let b = BoardOutline::new(50.0, 30.0);
        let bb = b.bounding_box();
        assert_eq!(bb.width(), 50.0);
        assert_eq!(bb.height(), 30.0);
    }

    #[test]
    fn tolerance_profiles_are_ordered() {
        let resin = TOLERANCE_RESIN.xy_accuracy_mm;
        let fdm = TOLERANCE_FDM.xy_accuracy_mm;
        let cnc = TOLERANCE_CNC.xy_accuracy_mm;
        assert!(resin < fdm, "resin should be tighter than fdm");
        assert!(cnc < resin, "cnc should be tighter than resin");
    }

    #[test]
    fn tolerance_by_name_lookup() {
        assert!(tolerance_by_name("fdm").is_some());
        assert!(tolerance_by_name("resin").is_some());
        assert!(tolerance_by_name("cnc").is_some());
        assert!(tolerance_by_name("laser").is_none());
    }
}
