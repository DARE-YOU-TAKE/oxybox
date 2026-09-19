use glam::Vec2;
use std::os::raw::c_void;

use crate::{BodyKind, ShapeDefinition, ShapeId};

/// Body id references a body instance. This should be treated as an opaque handle.
///
/// You can create a `BodyId` with [`World::create_body`](crate::World::create_body).
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct BodyId(sys::b2BodyId);

impl BodyId {
    /// Creates a [`BodyId`] from a [`sys::b2BodyId`].
    pub fn from_b2(input: sys::b2BodyId) -> Self {
        Self(input)
    }

    /// Get the world position of a body. This is the location of the body origin.
    pub fn position(&self) -> Vec2 {
        unsafe { sys::b2Body_GetPosition(self.0).into() }
    }

    /// Get the body kind.
    pub fn kind(&self) -> BodyKind {
        let b2body_type = unsafe { sys::b2Body_GetType(self.0) };

        match b2body_type {
            sys::b2BodyType_b2_dynamicBody => BodyKind::Dynamic,
            sys::b2BodyType_b2_kinematicBody => BodyKind::Kinematic,
            sys::b2BodyType_b2_staticBody => BodyKind::Static,
            _ => unreachable!("Box2D returned unknown BodyKind"),
        }
    }

    /// Sets arbitrary user data on the body.
    pub fn set_user_data(&self, data: usize) {
        unsafe {
            sys::b2Body_SetUserData(self.0, data as *mut c_void);
        }
    }

    /// Get the user data stored in a body, if any. By default, all user data has `0` stored
    /// within it.
    pub fn user_data(&self) -> usize {
        unsafe { sys::b2Body_GetUserData(self.0) as usize }
    }

    /// Get the world rotation of a body in radians.
    pub fn rotation(&self) -> f32 {
        let r = unsafe { sys::b2Body_GetRotation(self.0) };
        r.s.atan2(r.c)
    }

    /// Get the linear velocity of a body’s center of mass. Usually in meters per second.
    pub fn linear_velocity(&self) -> Vec2 {
        unsafe { sys::b2Body_GetLinearVelocity(self.0).into() }
    }

    /// Set the linear velocity of a body. Usually in meters per second.
    pub fn set_linear_velocity(&self, linear_velocity: Vec2) {
        unsafe {
            sys::b2Body_SetLinearVelocity(self.0, linear_velocity.into());
        }
    }

    /// Set the world transform of a body. This acts as a teleport and is fairly expensive.
    /// Generally you should create a body with the intended transform.
    ///
    /// `rotation` is in radians.
    pub fn set_transform(&self, position: Vec2, rotation: f32) {
        unsafe {
            sys::b2Body_SetTransform(
                self.0,
                position.into(),
                sys::b2Rot {
                    c: rotation.cos(),
                    s: rotation.sin(),
                },
            );
        }
    }

    /// Apply an impulse to the center of mass. This immediately modifies the velocity.
    /// The impulse is ignored if the body is not awake.
    ///
    /// `impulse` is the world impulse vector, usually in Ns or kgm/s.
    /// `wake` will also wake up the body.
    pub fn apply_impulse(&self, impulse: Vec2, wake: bool) {
        unsafe { sys::b2Body_ApplyLinearImpulseToCenter(self.0, impulse.into(), wake) }
    }

    /// Apply an impulse at a point. This immediately modifies the velocity.
    /// It also modifies the angular velocity if the point of application is not at the center of mass.
    ///
    /// `impulse` is the world impulse vector, usually in Ns or kgm/s.
    /// `point` is the world position of the point of application.
    /// `wake` will also wake up the body.
    pub fn apply_impulse_at(&self, impulse: Vec2, point: Vec2, wake: bool) {
        unsafe { sys::b2Body_ApplyLinearImpulse(self.0, impulse.into(), point.into(), wake) }
    }

    /// Apply an angular impulse. The impulse is ignored if the body is not awake.
    ///
    /// `impulse` is the angular impulse, usually in units of kgmm/s.
    /// `wake` will also wake up the body.
    pub fn apply_angular_impulse(&self, impulse: f32, wake: bool) {
        unsafe { sys::b2Body_ApplyAngularImpulse(self.0, impulse, wake) }
    }

    /// Get the mass of the body, usually in kilograms.
    pub fn mass(&self) -> f32 {
        unsafe { sys::b2Body_GetMass(self.0) }
    }

    /// Destroy a rigid body given an id. This destroys all shapes and joints attached to the body.
    ///
    /// Do not keep references to the associated shapes and joints.
    pub fn destroy_body(self) {
        unsafe {
            sys::b2DestroyBody(self.0);
        }
    }

    /// Body identifier validation. Can be used to detect orphaned ids. Provides validation for up to 64K allocations.
    pub fn body_valid(&self) -> bool {
        unsafe { sys::b2Body_IsValid(self.0) }
    }

    /// Attaches a circle to the body.
    ///
    /// The `center` is the local offset from the body, and the `radius` is the radius of the circle.
    pub fn attach_circle(self, center: Vec2, radius: f32, shape_def: &ShapeDefinition) -> ShapeId {
        ShapeId::create_circle(self, center, radius, shape_def)
    }

    /// Attaches a rectangle to the body.
    ///
    /// Make a box (rectangle) polygon, bypassing the need for a convex hull.
    /// `half_dims` are the half dimensions of the rectangle, `offset` is the offset relative to the body,
    /// and `rotation` is the rotation amount in radians.
    pub fn attach_rectangle(
        self,
        half_dims: Vec2,
        offset: Vec2,
        rotation: f32,
        shape_def: &ShapeDefinition,
    ) -> ShapeId {
        ShapeId::create_rectangle(self, half_dims, offset, rotation, shape_def)
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
    pub fn attach_polygon(self, polygon_points: &[Vec2], shape_def: &ShapeDefinition) -> Option<ShapeId> {
        ShapeId::create_polygon(self, polygon_points, shape_def)
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
