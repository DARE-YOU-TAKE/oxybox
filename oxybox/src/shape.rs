mod filter;
mod shape_definition;
mod shape_id;
mod shape_kind;
mod surface_material;

use std::{marker::PhantomData, ops::Deref};

pub use filter::Filter;
pub use shape_definition::ShapeDefinition;
pub use shape_id::*;
pub use shape_kind::ShapeKind;
pub use surface_material::SurfaceMaterial;

use crate::{BodyId, BodyRef, Rotation, World};

/// A live shape in a [`World`](crate::World), borrowed from it for reading only.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct ShapeRef<'a>(pub(crate) sys::b2ShapeId, PhantomData<&'a World>);

impl<'a> ShapeRef<'a> {
    /// Creates a new [`ShapeRef`] out of an existing *valid* [`ShapeId`].
    pub(crate) fn new(shape_id: ShapeId) -> Self {
        Self(shape_id.0, PhantomData)
    }

    /// The raw Box2D id backing this handle.
    pub(crate) fn raw(self) -> sys::b2ShapeId {
        self.0
    }

    /// The [`ShapeId`] naming this shape.
    pub fn id(&self) -> ShapeId {
        ShapeId::from_b2(self.0)
    }

    /// The [`BodyId`] of the body this shape is attached to.
    pub fn body_id(&self) -> BodyId {
        // safety: a shape is always attached to a body in the same world, and holding this
        // `ShapeRef` is proof that that world is still borrowed.
        BodyId::from_b2(unsafe { sys::b2Shape_GetBody(self.0) })
    }

    /// The body this shape is attached to, for reading.
    ///
    /// This is a [`BodyRef`] rather than a [`Body`](crate::Body) on purpose: it is reachable from
    /// inside a query callback, where moving the body would corrupt the traversal in progress.
    pub fn body(&self) -> BodyRef<'a> {
        BodyRef::new(self.body_id())
    }

    /// Get the kind of a shape.
    pub fn kind(&self) -> ShapeKind {
        let shape = unsafe { sys::b2Shape_GetType(self.0) };
        match shape {
            sys::b2ShapeType_b2_circleShape => ShapeKind::Circle,
            sys::b2ShapeType_b2_capsuleShape => ShapeKind::Capsule,
            sys::b2ShapeType_b2_segmentShape => ShapeKind::Segment,
            sys::b2ShapeType_b2_polygonShape => ShapeKind::Polygon,
            sys::b2ShapeType_b2_chainSegmentShape => ShapeKind::ChainSegment,
            _ => unreachable!("unknown shape kind: {:?}", shape),
        }
    }

    /// Gets the dimensions of the shape. This can be imagined as a box which will
    /// fully enclose the given shape.
    ///
    /// These are the shape's own dimensions, in the local space of the body it is attached to --
    /// the body's rotation is not applied.
    pub fn dimensions(&self) -> glam::Vec2 {
        // an identity transform, so that we measure the shape in its own body-local space
        let identity = sys::b2Transform {
            p: sys::b2Vec2 { x: 0.0, y: 0.0 },
            q: Rotation::IDENTITY.into(),
        };

        let aabb = unsafe {
            match self.kind() {
                ShapeKind::Circle => sys::b2ComputeCircleAABB(&sys::b2Shape_GetCircle(self.0), identity),
                ShapeKind::Capsule => sys::b2ComputeCapsuleAABB(&sys::b2Shape_GetCapsule(self.0), identity),
                ShapeKind::Segment => sys::b2ComputeSegmentAABB(&sys::b2Shape_GetSegment(self.0), identity),
                ShapeKind::Polygon => sys::b2ComputePolygonAABB(&sys::b2Shape_GetPolygon(self.0), identity),
                ShapeKind::ChainSegment => {
                    sys::b2ComputeSegmentAABB(&sys::b2Shape_GetChainSegment(self.0).segment, identity)
                }
            }
        };

        glam::Vec2::new(
            aabb.upperBound.x - aabb.lowerBound.x,
            aabb.upperBound.y - aabb.lowerBound.y,
        )
    }

    /// Gets the width of the given shape. See [`ShapeRef::dimensions`].
    pub fn width(&self) -> f32 {
        self.dimensions().x
    }

    /// Gets the height of the given shape. See [`ShapeRef::dimensions`].
    pub fn height(&self) -> f32 {
        self.dimensions().y
    }

    /// Get the user data stored in a shape, if any. By default, all user data has `0` stored
    /// within it.
    pub fn user_data(&self) -> usize {
        unsafe { sys::b2Shape_GetUserData(self.0) as usize }
    }

    /// Get the density of a shape, usually in kg/m^2. See [`Shape::set_density`].
    pub fn density(&self) -> f32 {
        unsafe { sys::b2Shape_GetDensity(self.0) }
    }

    /// Get the Coulomb (dry) friction coefficient of a shape.
    pub fn friction(&self) -> f32 {
        unsafe { sys::b2Shape_GetFriction(self.0) }
    }

    /// Get the restitution (bounciness) of a shape.
    pub fn restitution(&self) -> f32 {
        unsafe { sys::b2Shape_GetRestitution(self.0) }
    }

    /// Get the [`Filter`] deciding which shapes this one collides with.
    pub fn filter(&self) -> Filter {
        // safety: `Filter` and `b2Filter` have the same layout, checked at compile time where
        // `Filter` is defined.
        unsafe { std::mem::transmute(sys::b2Shape_GetFilter(self.0)) }
    }

    /// Whether this shape reports contact events. See [`Shape::enable_contact_events`].
    pub fn contact_events_enabled(&self) -> bool {
        unsafe { sys::b2Shape_AreContactEventsEnabled(self.0) }
    }
}

impl std::fmt::Debug for ShapeRef<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.id().fmt(f)
    }
}

/// A live shape in a [`World`](crate::World), borrowed from it for reading and writing.
///
/// Get one with [`World::shape`](crate::World::shape). This also has access to all the functions
/// on [`ShapeRef`] via deref.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Shape<'a>(pub(crate) ShapeRef<'a>);

impl<'a> Shape<'a> {
    /// Creates a new [`Shape`] out of an existing *valid* [`ShapeId`].
    pub(crate) fn new(shape_id: ShapeId) -> Self {
        Self(ShapeRef::new(shape_id))
    }

    /// Sets arbitrary user data on the shape.
    pub fn set_user_data(&self, data: usize) {
        unsafe { sys::b2Shape_SetUserData(self.raw(), data as *mut std::ffi::c_void) }
    }

    /// Set the mass density of a shape, usually in kg/m^2.
    ///
    /// `update_body_mass` recomputes the parent body's mass properties from its shapes. Pass
    /// `false` when setting the density of several shapes in a row, then update once at the end.
    pub fn set_density(&self, density: f32, update_body_mass: bool) {
        unsafe { sys::b2Shape_SetDensity(self.raw(), density, update_body_mass) }
    }

    /// Set the Coulomb (dry) friction coefficient, usually in the range `0.0..=1.0`.
    pub fn set_friction(&self, friction: f32) {
        unsafe { sys::b2Shape_SetFriction(self.raw(), friction) }
    }

    /// Set the restitution (bounciness), usually in the range `0.0..=1.0`.
    pub fn set_restitution(&self, restitution: f32) {
        unsafe { sys::b2Shape_SetRestitution(self.raw(), restitution) }
    }

    /// Set the [`Filter`] deciding which shapes this one collides with.
    ///
    /// This wakes both bodies of any contact this shape is part of.
    pub fn set_filter(&self, filter: Filter) {
        // safety: `Filter` and `b2Filter` have the same layout, checked at compile time where
        // `Filter` is defined.
        unsafe { sys::b2Shape_SetFilter(self.raw(), std::mem::transmute::<Filter, sys::b2Filter>(filter)) }
    }

    /// Enable or disable contact events for this shape.
    ///
    /// Contact events are opt-in per shape, and Box2D leaves them off by default -- a shape that
    /// never enables them will never appear in [`World::contact_events`](crate::World::contact_events).
    pub fn enable_contact_events(&self, enabled: bool) {
        unsafe { sys::b2Shape_EnableContactEvents(self.raw(), enabled) }
    }
}

impl<'a> Deref for Shape<'a> {
    type Target = ShapeRef<'a>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::fmt::Debug for Shape<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.id().fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use glam::Vec2;

    use crate::*;

    #[test]
    fn shape_dimensions() {
        let world = World::new(WorldDefinition::new());

        let body = world.create_body(BodyDefinition::new());
        let rect = body.attach_rectangle(
            Vec2::new(3.0, 7.0),
            Vec2::ZERO,
            Rotation::IDENTITY,
            ShapeDefinition::default(),
        );
        let circle = body.attach_circle(Vec2::new(100.0, 100.0), 2.0, ShapeDefinition::default());

        // half dimensions in, full dimensions out
        assert_eq!(rect.kind(), ShapeKind::Polygon);
        assert!((rect.width() - 6.0).abs() < 1e-3, "{}", rect.width());
        assert!((rect.height() - 14.0).abs() < 1e-3, "{}", rect.height());

        // the circle's offset moves it, but must not change its extents
        assert_eq!(circle.kind(), ShapeKind::Circle);
        assert_eq!(circle.dimensions(), Vec2::new(4.0, 4.0));
    }

    #[test]
    fn a_shape_finds_its_body() {
        let world = World::new(WorldDefinition::new());

        let body = world.create_body(BodyDefinition {
            position: Vec2::new(3.0, 4.0),
            ..BodyDefinition::new()
        });
        let body_id = body.id();
        let shape = body.attach_circle(Vec2::ZERO, 1.0, ShapeDefinition::new());

        assert_eq!(shape.body().id(), body_id);
    }

    #[test]
    fn sibling_shape_handles_coexist() {
        let world = World::new(WorldDefinition::new());

        let body = world.create_body(BodyDefinition::new());

        // two handles to different shapes of one body, live at the same time and both mutable.
        // nothing in the API is allowed to be more restrictive than Box2D is in C, and C lets you
        // hold as many shape ids as you like.
        let left = body.attach_circle(Vec2::new(-1.0, 0.0), 1.0, ShapeDefinition::new());
        let right = body.attach_circle(Vec2::new(1.0, 0.0), 1.0, ShapeDefinition::new());

        left.set_friction(0.25);
        right.set_friction(0.75);

        assert_eq!(left.friction(), 0.25, "the left shape's friction did not stick");
        assert_eq!(right.friction(), 0.75, "the right shape's friction did not stick");

        // and the edits are still there when the shapes are fetched back out of the world
        let (left, right) = (left.id(), right.id());
        assert_eq!(world.shape(left).unwrap().friction(), 0.25);
        assert_eq!(world.shape(right).unwrap().friction(), 0.75);

        // and the handles taken at attach time are still the live ones
        assert_eq!(world.shape(left).unwrap().id(), left);
    }
}
