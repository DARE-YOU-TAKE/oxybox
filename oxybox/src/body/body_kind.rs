#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum BodyKind {
    /// Positive mass, velocity determined by forces, moved by solver
    Dynamic = sys::b2BodyType_b2_dynamicBody,

    /// Zero mass, velocity set by user, moved by solver
    Kinematic = sys::b2BodyType_b2_kinematicBody,

    /// Zero mass, zero velocity, may be manually moved
    #[default]
    Static = sys::b2BodyType_b2_staticBody,
}

impl BodyKind {
    /// Returns if this body matches [`BodyKind::Dynamic`].
    pub fn is_dynamic(self) -> bool {
        self == BodyKind::Dynamic
    }

    /// Returns if this body matches [`BodyKind::Static`].
    pub fn is_static(self) -> bool {
        self == BodyKind::Static
    }

    /// Returns if this body matches [`BodyKind::Kinematic`].
    pub fn is_kinematic(self) -> bool {
        self == BodyKind::Kinematic
    }
}
