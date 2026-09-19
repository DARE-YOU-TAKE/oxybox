use glam::Vec2;

use crate::{mirrors_layout, opaque};

/// A world definition holds all the data needed to construct a world.
///
/// ```
/// # use glam::Vec2;
/// # use oxybox::{World, WorldDefinition};
/// let world = World::new(WorldDefinition {
///     gravity: Vec2::new(0.0, -10.0),
///     enable_sleep: false,
///     ..WorldDefinition::new()
/// });
/// ```
///
/// You can safely re-use world definitions. World definitions are temporary objects used to
/// bundle creation parameters.
///
/// **NOTE: several defaults here are scaled by the global length units, so
/// [`set_length_units_per_meter`](crate::set_length_units_per_meter) must be called before you
/// build a definition, not merely before you build a [`World`].**
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct WorldDefinition {
    /// Gravity vector. Box2D has no up-vector defined. Usually in m/s^2.
    pub gravity: Vec2,

    /// Restitution speed threshold, usually in m/s. Collisions above this speed have restitution
    /// applied (will bounce).
    pub restitution_threshold: f32,

    /// Threshold speed for hit events. Usually meters per second.
    pub hit_event_threshold: f32,

    /// Contact stiffness. Cycles per second.
    ///
    /// Increasing this increases the speed of overlap recovery, but can introduce jitter.
    pub contact_hertz: f32,

    /// Contact bounciness. Non-dimensional.
    ///
    /// You can speed up overlap recovery by decreasing this with the trade-off that overlap
    /// resolution becomes more energetic.
    pub contact_damping_ratio: f32,

    /// This parameter controls how fast overlap is resolved and usually has units of meters per
    /// second. This only puts a cap on the resolution speed. The resolution speed is increased by
    /// increasing the hertz and/or decreasing the damping ratio.
    pub max_contact_push_speed: f32,

    /// Maximum linear speed. Usually meters per second.
    pub maximum_linear_speed: f32,

    /// Optional mixing callbacks for friction and restitution. See [`MixingCallbacks`].
    pub mixing_callbacks: MixingCallbacks,

    /// Can bodies go to sleep to improve performance.
    pub enable_sleep: bool,

    /// Enable continuous collision.
    pub enable_continuous: bool,

    /// Number of workers given to the task system. Currently does nothing because a `task_system` cannot be set.
    pub worker_count: std::os::raw::c_int,

    /// The hooks Box2D uses to hand work to your task system. See [`TaskSystem`].
    pub task_system: TaskSystem,

    /// Use this to store application specific world data.
    ///
    /// Box2D stores this as a pointer-sized value, so this is a `usize` rather than a `u64`.
    pub user_data: usize,

    /// Box2D's own validity cookie. See [`Internal`].
    pub internal: opaque::Internal,
}

/// Box2D's optional mixing callbacks for friction and restitution.
///
/// This is opaque: `oxybox` does not yet support overriding the mixing callbacks.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct MixingCallbacks([u8; 16]);

/// The hooks Box2D uses to hand work to your task system. Opaque and cannot be constructed yet.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TaskSystem([u8; 24]);

mirrors_layout! {
    WorldDefinition => sys::b2WorldDef {
        gravity => gravity,
        restitution_threshold => restitutionThreshold,
        hit_event_threshold => hitEventThreshold,
        contact_hertz => contactHertz,
        contact_damping_ratio => contactDampingRatio,
        max_contact_push_speed => maxContactPushSpeed,
        maximum_linear_speed => maximumLinearSpeed,
        mixing_callbacks: MixingCallbacks => frictionCallback .. enableSleep,
        enable_sleep => enableSleep,
        enable_continuous => enableContinuous,
        worker_count => workerCount,
        task_system: TaskSystem => enqueueTask .. userData,
        user_data => userData,
        internal => internalValue,
    }
}

impl WorldDefinition {
    /// Creates a new WorldDefinition filled in with the Box2D defaults
    pub fn new() -> Self {
        // safety: `WorldDefinition` and `b2WorldDef` have the same layout, which the assertions
        // above check field by field at compile time.
        unsafe { std::mem::transmute(sys::b2DefaultWorldDef()) }
    }

    /// This definition as the Box2D struct it is already laid out as. Borrowing rather than
    /// converting is the reason for the layout assertions above.
    pub(crate) fn as_b2(&self) -> *const sys::b2WorldDef {
        (self as *const WorldDefinition).cast()
    }
}

impl Default for WorldDefinition {
    fn default() -> Self {
        Self::new()
    }
}
