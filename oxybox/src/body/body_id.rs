/// Body id references a body instance. This should be treated as an opaque handle.
///
/// You get a `BodyId` from [`BodyRef::id`](crate::BodyRef::id). To do anything with one, hand it back
/// to the world it came from -- see [`World::body`](crate::World::body), which re-checks that the
/// id names a live body in that world before vending a [`Body`](crate::Body) handle.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct BodyId(pub(crate) sys::b2BodyId);

impl BodyId {
    /// Creates a [`BodyId`] from a [`sys::b2BodyId`].
    pub fn from_b2(input: sys::b2BodyId) -> Self {
        Self(input)
    }

    /// Body identifier validation. Can be used to detect orphaned ids. Provides validation for up to 64K allocations.
    pub fn is_valid(self) -> bool {
        unsafe { sys::b2Body_IsValid(self.0) }
    }
}

impl From<sys::b2BodyId> for BodyId {
    fn from(value: sys::b2BodyId) -> Self {
        Self::from_b2(value)
    }
}

impl From<BodyId> for sys::b2BodyId {
    fn from(value: BodyId) -> Self {
        value.0
    }
}

impl std::fmt::Debug for BodyId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.pad(&format!("{}@{}v{}", self.0.world0, self.0.index1, self.0.generation))
    }
}
