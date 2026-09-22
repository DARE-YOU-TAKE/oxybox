/// A rotation stored as the cosine and sine of the angle.
///
/// Box2D assumes a rotation is unit length. Setting [`cosine`](Rotation::cosine) and
/// [`sine`](Rotation::sine) yourself to something that is not will make the simulation misbehave.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rotation {
    /// The cosine of the angle.
    ///
    /// Box2D assumes a rotation is unit length. Setting [`cosine`](Rotation::cosine) and
    /// [`sine`](Rotation::sine) yourself to something that is not will make the simulation misbehave.
    pub cosine: f32,

    /// The sine of the angle.
    ///
    /// Box2D assumes a rotation is unit length. Setting [`cosine`](Rotation::cosine) and
    /// [`sine`](Rotation::sine) yourself to something that is not will make the simulation misbehave.
    pub sine: f32,
}

crate::mirrors_layout! {
    Rotation => sys::b2Rot {
        cosine => c,
        sine => s,
    }
}

impl Rotation {
    /// No rotation at all.
    pub const IDENTITY: Rotation = Rotation { cosine: 1.0, sine: 0.0 };

    /// Creates a rotation from an angle in radians.
    pub fn from_radians(radians: f32) -> Self {
        Self {
            cosine: radians.cos(),
            sine: radians.sin(),
        }
    }

    /// This rotation as an angle in radians.
    pub fn radians(self) -> f32 {
        self.sine.atan2(self.cosine)
    }
}

impl Default for Rotation {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl From<f32> for Rotation {
    fn from(value: f32) -> Self {
        Self::from_radians(value)
    }
}

impl From<Rotation> for f32 {
    fn from(value: Rotation) -> Self {
        value.radians()
    }
}

impl From<sys::b2Rot> for Rotation {
    fn from(value: sys::b2Rot) -> Self {
        Self {
            cosine: value.c,
            sine: value.s,
        }
    }
}

impl From<Rotation> for sys::b2Rot {
    fn from(value: Rotation) -> Self {
        sys::b2Rot {
            c: value.cosine,
            s: value.sine,
        }
    }
}
