/// Limits which shapes a world query considers.
///
/// A shape is only reported by a query when the shape's category is in the query's mask *and* the
/// query's category is in the shape's mask.
#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct QueryFilter(pub(crate) sys::b2QueryFilter);

impl QueryFilter {
    /// Creates a new QueryFilter which has a category of `1` and a mask of
    /// every bit, which matches any shape left on the default [`ShapeDefinition`](crate::ShapeDefinition)
    /// filter.
    pub fn new() -> Self {
        Self(unsafe { sys::b2DefaultQueryFilter() })
    }

    /// The collision category bits of this query. Normally you just set one bit.
    pub fn category(mut self, category: u64) -> Self {
        self.0.categoryBits = category;
        self
    }

    /// The collision mask bits. This states the shape categories that this query would accept
    /// for collision.
    pub fn mask(mut self, mask: u64) -> Self {
        self.0.maskBits = mask;
        self
    }
}

impl Default for QueryFilter {
    fn default() -> Self {
        Self::new()
    }
}
