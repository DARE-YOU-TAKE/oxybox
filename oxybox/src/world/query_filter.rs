/// The query filter is used to filter collisions between queries and shapes. For example,
/// you may want a ray-cast representing a projectile to hit players and the static environment
/// but not debris.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct QueryFilter {
    /// The collision category bits of this query. Normally you just set one bit.
    pub category_bits: u64,

    /// The collision mask bits. This states the shape categories that this query would accept
    /// for collision.
    pub mask_bits: u64,
}

impl QueryFilter {
    /// Creates a new QueryFilter which has a category of `1` and a mask of
    /// every bit, which matches any shape left on the default [`ShapeDefinition`](crate::ShapeDefinition)
    /// filter.
    pub fn new() -> Self {
        // safety: these two structs have the same memory layout
        unsafe { std::mem::transmute::<sys::b2QueryFilter, QueryFilter>(sys::b2DefaultQueryFilter()) }
    }

    // we can safety do this cast since we are the same in memory representation
    pub(crate) fn as_b2(self) -> sys::b2QueryFilter {
        // safety: these two structs have the same memory layout
        unsafe { std::mem::transmute(self) }
    }
}

impl Default for QueryFilter {
    fn default() -> Self {
        Self::new()
    }
}

crate::mirrors_layout! {
    QueryFilter => sys::b2QueryFilter {
        category_bits => categoryBits,
        mask_bits => maskBits,
    }
}
