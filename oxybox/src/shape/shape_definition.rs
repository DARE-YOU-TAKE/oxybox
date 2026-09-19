use crate::mirrors_layout;
use crate::opaque::Internal;
use crate::{Filter, SurfaceMaterial};

/// A shape definition holds all the data needed to construct a shape.
///
/// ```
/// # use glam::Vec2;
/// # use oxybox::{BodyDefinition, ShapeDefinition, SurfaceMaterial, World};
/// # let mut world = World::default();
/// # let body = world.create_body(BodyDefinition::new());
/// let shape = body.attach_circle(
///     Vec2::ZERO,
///     5.0,
///     ShapeDefinition {
///         density: 2.0,
///         material: SurfaceMaterial {
///             friction: 0.3,
///             restitution: 0.0,
///             ..SurfaceMaterial::new()
///         },
///         ..ShapeDefinition::new()
///     },
/// );
/// ```
///
/// You can safely re-use shape definitions. Shapes are added to a body after construction.
/// Shape definitions are temporary objects used to bundle creation parameters.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ShapeDefinition {
    /// Use this to store application specific shape data.
    ///
    /// Box2D stores this as a pointer-sized value, so this is a `usize` rather than a `u64`.
    pub user_data: usize,

    /// The surface material for this shape. See [`SurfaceMaterial`].
    pub material: SurfaceMaterial,

    /// The density, usually in kg/m^2.
    ///
    /// This is not part of the surface material because this is for the interior, which may have
    /// other considerations, such as being hollow. For example a wood barrel may be hollow or full
    /// of water.
    pub density: f32,

    /// Collision filtering data. See [`Filter`].
    pub filter: Filter,

    /// A sensor shape generates overlap events but never generates a collision response.
    ///
    /// Sensors do not have continuous collision. Instead, use a ray or shape cast for those
    /// scenarios. Sensors still contribute to the body mass if they have non-zero density. Sensor
    /// events are disabled by default.
    pub is_sensor: bool,

    /// Enable sensor events for this shape. This applies to sensors and non-sensors.
    ///
    /// False by default, even for sensors.
    pub enable_sensor_events: bool,

    /// Enable contact events for this shape.
    ///
    /// Only applies to kinematic and dynamic bodies. Ignored for sensors. False by default.
    pub enable_contact_events: bool,

    /// Enable hit events for this shape.
    ///
    /// Only applies to kinematic and dynamic bodies. Ignored for sensors. False by default.
    pub enable_hit_events: bool,

    /// Enable pre-solve contact events for this shape.
    ///
    /// Only applies to dynamic bodies. These are expensive and must be carefully handled due to
    /// threading. Ignored for sensors.
    pub enable_pre_solve_events: bool,

    /// When shapes are created they will scan the environment for collision the next time step.
    ///
    /// This can significantly slow down static body creation when there are many static shapes.
    /// This flag is ignored for dynamic and kinematic shapes, which always invoke contact creation.
    pub invoke_contact_creation: bool,

    /// Should the body update its mass properties when this shape is created. Default is true.
    pub update_body_mass: bool,

    /// Box2D's own validity cookie. See [`Internal`].
    pub internal: Internal,
}

mirrors_layout! {
    ShapeDefinition => sys::b2ShapeDef {
        user_data => userData,
        material => material,
        density => density,
        filter => filter,
        is_sensor => isSensor,
        enable_sensor_events => enableSensorEvents,
        enable_contact_events => enableContactEvents,
        enable_hit_events => enableHitEvents,
        enable_pre_solve_events => enablePreSolveEvents,
        invoke_contact_creation => invokeContactCreation,
        update_body_mass => updateBodyMass,
        internal => internalValue,
    }
}

impl ShapeDefinition {
    /// Creates a new ShapeDefinition filled in with the Box2D defaults
    pub fn new() -> Self {
        // safety: `ShapeDefinition` and `b2ShapeDef` have the same layout, which the assertions
        // above check field by field at compile time.
        unsafe { std::mem::transmute(sys::b2DefaultShapeDef()) }
    }

    /// This definition as the Box2D struct it is already laid out as. Borrowing rather than
    /// converting is the reason for the layout assertions above.
    pub(crate) fn as_b2(&self) -> *const sys::b2ShapeDef {
        (self as *const ShapeDefinition).cast()
    }
}

impl Default for ShapeDefinition {
    fn default() -> Self {
        Self::new()
    }
}
