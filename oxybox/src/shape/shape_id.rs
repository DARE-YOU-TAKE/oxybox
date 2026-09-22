/// Shape id references a shape instance. This should be treated as an opaque handle.
///
/// It is possible to hold a shape which has been destroyed -- you should run [`ShapeId::is_valid`]
/// to check that.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ShapeId(pub(crate) sys::b2ShapeId);

impl ShapeId {
    /// The maximum number of points a polygon can have.
    pub const MAX_POLYGON_POINTS: usize = 8;

    /// Creates a [`ShapeId`] from a [`sys::b2ShapeId`].
    pub fn from_b2(input: sys::b2ShapeId) -> Self {
        Self(input)
    }

    /// Shape identifier validation. Provides validation for up to 64K allocations.
    pub fn is_valid(self) -> bool {
        unsafe { sys::b2Shape_IsValid(self.0) }
    }
}

impl From<sys::b2ShapeId> for ShapeId {
    fn from(value: sys::b2ShapeId) -> Self {
        Self::from_b2(value)
    }
}

impl From<ShapeId> for sys::b2ShapeId {
    fn from(value: ShapeId) -> Self {
        value.0
    }
}

impl std::fmt::Debug for ShapeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.pad(&format!("{}@{}v{}", self.0.world0, self.0.index1, self.0.generation))
    }
}
