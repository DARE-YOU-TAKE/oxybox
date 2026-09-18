use glam::Vec2;

use crate::mirrors_layout;
use crate::opaque::Internal;
use crate::{BodyKind, Rotation};

/// A body definition holds all the data needed to construct a rigid body.
///
/// ```
/// # use glam::Vec2;
/// # use oxybox::{BodyDefinition, BodyKind, World};
/// # let mut world = World::default();
/// let body = world.create_body(BodyDefinition {
///     kind: BodyKind::Dynamic,
///     position: Vec2::new(0.0, 20.0),
///     ..BodyDefinition::new()
/// });
/// ```
///
/// You can safely re-use body definitions. Shapes are added to a body after construction.
/// Body definitions are temporary objects used to bundle creation parameters.
///
/// **NOTE: the default sleep threshold is scaled by the global length units, so
/// [`set_length_units_per_meter`](crate::set_length_units_per_meter) must be called before you
/// build a definition, not merely before you create a body.**
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct BodyDefinition<'a> {
    /// The body type: static, kinematic, or dynamic. The default is [`BodyKind::Static`].
    pub kind: BodyKind,

    /// The initial world position of the body. Bodies should be created with the desired position.
    ///
    /// Creating bodies at the origin and then moving them nearly doubles the cost of body
    /// creation, especially if the body is moved after shapes have been added.
    pub position: Vec2,

    /// The initial world rotation of the body. See [`Rotation`].
    pub rotation: Rotation,

    /// The initial linear velocity of the body's origin. Usually in meters per second.
    pub linear_velocity: Vec2,

    /// The initial angular velocity of the body. Radians per second.
    pub angular_velocity: f32,

    /// Linear damping is used to reduce the linear velocity.
    ///
    /// The damping parameter can be larger than 1.0 but the damping effect becomes sensitive to
    /// the time step when the damping parameter is large. Generally linear damping is undesirable
    /// because it makes objects move slowly as if they are floating.
    pub linear_damping: f32,

    /// Angular damping is used to reduce the angular velocity.
    ///
    /// The damping parameter can be larger than 1.0 but the damping effect becomes sensitive to
    /// the time step when the damping parameter is large. Angular damping can be used to slow down
    /// rotating bodies.
    pub angular_damping: f32,

    /// Scale the gravity applied to this body. Non-dimensional.
    pub gravity_scale: f32,

    /// Sleep speed threshold. The default is 0.05 meters per second.
    pub sleep_threshold: f32,

    /// Optional body name for debugging. See [`BodyName`].
    pub name: Option<BodyName<'a>>,

    /// Use this to store application specific body data.
    ///
    /// Box2D stores this as a pointer-sized value, so this is a `usize` rather than a `u64`.
    pub user_data: usize,

    /// Set this to false if this body should never fall asleep.
    pub enable_sleep: bool,

    /// Is this body initially awake or sleeping?
    pub is_awake: bool,

    /// Should this body be prevented from rotating? Useful for characters.
    pub fixed_rotation: bool,

    /// Treat this body as a high speed object that performs continuous collision detection against
    /// dynamic and kinematic bodies, but not other bullet bodies.
    ///
    /// Bullets should be used sparingly. They are not a solution for general dynamic-versus-dynamic
    /// continuous collision. They may interfere with joint constraints.
    pub is_bullet: bool,

    /// Used to disable a body. A disabled body does not move or collide.
    pub is_enabled: bool,

    /// This allows this body to bypass rotational speed limits. Should only be used for circular
    /// objects, like wheels.
    pub allow_fast_rotation: bool,

    /// Box2D's own validity cookie. See [`Internal`].
    pub internal: Internal,
}

/// Box2D's optional name for a body, used when debugging. Box2D copies the name into the body when
/// the body is created, keeping only the first 31 characters.
///
/// The name is borrowed until the definition is consumed, which is why [`BodyDefinition`] carries a
/// lifetime. Box2D only copies the name during [`World::create_body`](crate::World::create_body),
/// so a name that dies before then is refused.
///
/// ```compile_fail,E0597
/// # use oxybox::{BodyDefinition, BodyName, World};
/// # use std::ffi::CString;
/// let definition = {
///     let name = CString::new("dies too soon").unwrap();
///     BodyDefinition {
///         name: Some(BodyName::new(&name)),
///         ..BodyDefinition::new()
///     }
/// };
///
/// let mut world = World::default();
/// world.create_body(definition);
/// ```
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct BodyName<'a> {
    ptr: std::ptr::NonNull<std::os::raw::c_char>,
    _lifetime: std::marker::PhantomData<&'a std::ffi::CStr>,
}

// make sure that we're getting niche optimization, which I wasn't confident in since we're going
// through a repr(transparent) and there are two fields (though one is a ZST)
const _: () = {
    assert!(size_of::<Option<BodyName<'static>>>() == size_of::<*const std::os::raw::c_char>());
};

impl<'a> BodyName<'a> {
    /// Creates a new BodyName borrowing a CStr. Note that Box2D will only copy the first 31 characters.
    pub fn new(name: &'a std::ffi::CStr) -> Self {
        Self {
            ptr: std::ptr::NonNull::from(name).cast(),
            _lifetime: std::marker::PhantomData,
        }
    }

    /// This name as a [`CStr`](std::ffi::CStr).
    pub fn as_cstr(&self) -> &'a std::ffi::CStr {
        // safety: `'a` outlives this borrow, so the pointer still points at the string it was
        // built from.
        unsafe { std::ffi::CStr::from_ptr(self.ptr.as_ptr()) }
    }
}

impl std::fmt::Debug for BodyName<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.as_cstr())
    }
}

mirrors_layout! {
    BodyDefinition => sys::b2BodyDef {
        kind => type_,
        position => position,
        rotation => rotation,
        linear_velocity => linearVelocity,
        angular_velocity => angularVelocity,
        linear_damping => linearDamping,
        angular_damping => angularDamping,
        gravity_scale => gravityScale,
        sleep_threshold => sleepThreshold,
        name => name,
        user_data => userData,
        enable_sleep => enableSleep,
        is_awake => isAwake,
        fixed_rotation => fixedRotation,
        is_bullet => isBullet,
        is_enabled => isEnabled,
        allow_fast_rotation => allowFastRotation,
        internal => internalValue,
    }
}

impl<'a> BodyDefinition<'a> {
    /// Creates a new BodyDefinition filled in with the Box2D defaults.
    pub fn new() -> BodyDefinition<'static> {
        // safety: `BodyDefinition` and `b2BodyDef` have the same layout, which the assertions
        // above check field by field at compile time.
        //
        // This may claim `'static` only because `b2DefaultBodyDef` leaves `name` null, so the
        // definition holds no borrow. Never return a definition with a real name from here: the
        // caller picks the lifetime, and could pick one that outlives the string.
        unsafe { std::mem::transmute(sys::b2DefaultBodyDef()) }
    }

    /// This definition as the Box2D struct it is already laid out as. Borrowing rather than
    /// converting is the reason for the layout assertions above.
    pub(crate) fn as_b2(&self) -> *const sys::b2BodyDef {
        (self as *const BodyDefinition).cast()
    }
}

impl<'a> Default for BodyDefinition<'a> {
    fn default() -> Self {
        Self::new()
    }
}
