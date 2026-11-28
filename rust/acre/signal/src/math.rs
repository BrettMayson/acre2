//! The bits of glm the C++ used, with the same `f32` operation order so results match.

use std::ops::{Add, Mul, Sub};

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    #[must_use]
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    #[must_use]
    pub fn dot(self, o: Self) -> f32 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }

    #[must_use]
    pub fn length(self) -> f32 {
        self.dot(self).sqrt()
    }

    /// `glm::normalize`: `v * inversesqrt(dot(v, v))`.
    #[must_use]
    pub fn normalize(self) -> Self {
        self * (1.0 / self.dot(self).sqrt())
    }

    /// `glm::distance`.
    #[must_use]
    pub fn distance(self, o: Self) -> f32 {
        (o - self).length()
    }

    /// `glm::distance2`.
    #[must_use]
    pub fn distance2(self, o: Self) -> f32 {
        let d = o - self;
        d.dot(d)
    }

    /// Distance in the horizontal plane (`glm::distance` of two `vec2`).
    #[must_use]
    pub fn distance_2d(self, o: Self) -> f32 {
        let (dx, dy) = (o.x - self.x, o.y - self.y);
        (dx * dx + dy * dy).sqrt()
    }
}

impl Add for Vec3 {
    type Output = Self;
    fn add(self, o: Self) -> Self {
        Self::new(self.x + o.x, self.y + o.y, self.z + o.z)
    }
}

impl Sub for Vec3 {
    type Output = Self;
    fn sub(self, o: Self) -> Self {
        Self::new(self.x - o.x, self.y - o.y, self.z - o.z)
    }
}

impl Mul<f32> for Vec3 {
    type Output = Self;
    fn mul(self, s: f32) -> Self {
        Self::new(self.x * s, self.y * s, self.z * s)
    }
}

impl From<[f32; 3]> for Vec3 {
    fn from([x, y, z]: [f32; 3]) -> Self {
        Self::new(x, y, z)
    }
}
