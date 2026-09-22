/// The kind of geometry a shape holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ShapeKind {
    /// A circle with an offset
    Circle = sys::b2ShapeType_b2_circleShape,

    /// A capsule is an extruded circle
    Capsule = sys::b2ShapeType_b2_capsuleShape,

    /// A line segment
    Segment = sys::b2ShapeType_b2_segmentShape,

    /// A convex polygon. Often, this is a rectangle.
    Polygon = sys::b2ShapeType_b2_polygonShape,

    /// A line segment owned by a chain shape
    ChainSegment = sys::b2ShapeType_b2_chainSegmentShape,
}
