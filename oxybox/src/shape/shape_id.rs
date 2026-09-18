use glam::Vec2;

use crate::{BodyId, ShapeDefinition, ShapeKind};

/// Shape id references a shape instance. This should be treated as an opaque handle.
///
/// It is possible to hold a shape which has been destroyed -- you should run [`ShapeId::is_valid`]
/// to check that.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ShapeId(sys::b2ShapeId);

impl ShapeId {
    /// The maximum number of points a polygon can have.
    pub const MAX_POLYGON_POINTS: usize = 8;

    /// Creates a [`ShapeId`] from a [`sys::b2ShapeId`].
    pub fn from_b2(input: sys::b2ShapeId) -> Self {
        Self(input)
    }

    /// Get the type of a shape
    pub fn shape_kind(&self) -> ShapeKind {
        let shape = unsafe { sys::b2Shape_GetType(self.0) };
        match shape {
            sys::b2ShapeType_b2_circleShape => ShapeKind::Circle,
            sys::b2ShapeType_b2_capsuleShape => ShapeKind::Capsule,
            sys::b2ShapeType_b2_segmentShape => ShapeKind::Segment,
            sys::b2ShapeType_b2_polygonShape => ShapeKind::Polygon,
            sys::b2ShapeType_b2_chainSegmentShape => ShapeKind::ChainSegment,
            _ => unreachable!("unknown shape_kind: {:?}", shape),
        }
    }

    /// Gets the dimensions of the shape. This can be imagined as a box which will
    /// fully enclose the given shape.
    ///
    /// These are the shape's own dimensions, in the local space of the body it is attached to --
    /// the body's rotation is not applied.
    pub fn dimensions(&self) -> Vec2 {
        // an identity transform, so that we measure the shape in its own body-local space
        let identity = sys::b2Transform {
            p: sys::b2Vec2 { x: 0.0, y: 0.0 },
            q: sys::b2Rot { c: 1.0, s: 0.0 },
        };

        let aabb = unsafe {
            match self.shape_kind() {
                ShapeKind::Circle => sys::b2ComputeCircleAABB(&sys::b2Shape_GetCircle(self.0), identity),
                ShapeKind::Capsule => sys::b2ComputeCapsuleAABB(&sys::b2Shape_GetCapsule(self.0), identity),
                ShapeKind::Segment => sys::b2ComputeSegmentAABB(&sys::b2Shape_GetSegment(self.0), identity),
                ShapeKind::Polygon => sys::b2ComputePolygonAABB(&sys::b2Shape_GetPolygon(self.0), identity),
                ShapeKind::ChainSegment => {
                    sys::b2ComputeSegmentAABB(&sys::b2Shape_GetChainSegment(self.0).segment, identity)
                }
            }
        };

        Vec2::new(
            aabb.upperBound.x - aabb.lowerBound.x,
            aabb.upperBound.y - aabb.lowerBound.y,
        )
    }

    /// Gets the width of the given shape. See [`ShapeId::dimensions`].
    pub fn width(&self) -> f32 {
        self.dimensions().x
    }

    /// Gets the height of the given shape. See [`ShapeId::dimensions`].
    pub fn height(&self) -> f32 {
        self.dimensions().y
    }

    /// Shape identifier validation. Provides validation for up to 64K allocations.
    pub fn is_valid(self) -> bool {
        unsafe { sys::b2Shape_IsValid(self.0) }
    }

    /// Gets the [`BodyId`] which this shape is attached to.
    pub fn get_body(self) -> BodyId {
        // safety: we aren't giving any pointers, so this should be trivially safe,
        // but the global scoped nature of this function worries me.
        let raw = unsafe { sys::b2Shape_GetBody(self.0) };
        BodyId::from_b2(raw)
    }

    /// Create a circle given a definition.
    ///
    /// The `center` is the local offset from the body, and the `radius` is the radius of the circle.
    pub fn create_circle(body_id: BodyId, center: Vec2, radius: f32, shape_def: ShapeDefinition) -> Self {
        let shape_id = unsafe {
            sys::b2CreateCircleShape(
                body_id.into(),
                shape_def.as_b2(),
                &sys::b2Circle {
                    center: center.into(),
                    radius,
                },
            )
        };

        Self::from_b2(shape_id)
    }

    /// Make a box (rectangle) polygon, bypassing the need for a convex hull.
    ///
    /// `half_dims` are the half dimensions of the rectangle, `offset` is the offset relative to the body,
    /// and `rotation` is the rotation amount in radians.
    pub fn create_rectangle(
        body_id: BodyId,
        half_dims: Vec2,
        offset: Vec2,
        rotation: f32,
        shape_def: ShapeDefinition,
    ) -> Self {
        let shape_id = unsafe {
            sys::b2CreatePolygonShape(
                body_id.into(),
                shape_def.as_b2(),
                &sys::b2MakeOffsetBox(
                    half_dims.x,
                    half_dims.y,
                    offset.into(),
                    sys::b2Rot {
                        c: rotation.cos(),
                        s: rotation.sin(),
                    },
                ),
            )
        };

        Self::from_b2(shape_id)
    }

    /// Create a polygon shape and attach it to a body.
    ///
    /// Some failure cases:
    /// - All points very close together
    /// - All points on a line
    /// - Less than 3 points
    /// - More than [`ShapeId::MAX_POLYGON_POINTS`].
    ///
    /// We weld close points and remove collinear points.
    ///
    /// If a hull would be made empty, no polygon is attached.
    #[must_use]
    pub fn create_polygon(body_id: BodyId, polygon_points: &[Vec2], shape_def: ShapeDefinition) -> Option<Self> {
        if polygon_points.len() > Self::MAX_POLYGON_POINTS || polygon_points.len() < 3 {
            return None;
        }

        // safety: glam::Vec2 and b2Vec2 are identical in memory.
        let hull = unsafe {
            sys::b2ComputeHull(
                polygon_points.as_ptr() as *const sys::b2Vec2,
                polygon_points.len() as i32,
            )
        };
        if hull.count == 0 {
            return None;
        }

        let shape_id =
            unsafe { sys::b2CreatePolygonShape(body_id.into(), shape_def.as_b2(), &sys::b2MakePolygon(&hull, 0.0)) };
        Some(Self::from_b2(shape_id))
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
