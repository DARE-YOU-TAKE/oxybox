use glam::Vec2;

use crate::{BodyId, ShapeId};

/// A physics world.
///
/// A world contains bodies, shapes, and constraints. You make create up to 128 worlds.
/// Each world is completely independent and may be simulated in parallel.
///
/// Dropping a world destroys it, along with every body and shape inside it. Any [`BodyId`]
/// or [`ShapeId`] taken from a world is left dangling once that world is dropped.
#[derive(Debug)]
pub struct World {
    pub(crate) id: sys::b2WorldId,
}

impl World {
    const SUBSTEPS: i32 = 4;

    /// Create a world for rigid body simulation.
    pub fn new(world_definition: &WorldDefinition) -> Self {
        let id = unsafe { sys::b2CreateWorld(&world_definition.0) };
        Self { id }
    }

    /// The raw id of the world.
    pub fn id(&self) -> sys::b2WorldId {
        self.id
    }

    /// Simulate a world for one time step.
    /// This performs collision detection, integration, and constraint solution.
    ///
    /// `delta_time` is the amount of time to simulate. This should be a fixed number, usually
    /// `1.0 / 60.0` -- a varying time step will make the simulation non-deterministic and can
    /// hurt stability.
    pub fn step(&mut self, delta_time: f32) {
        unsafe {
            sys::b2World_Step(self.id, delta_time, Self::SUBSTEPS);
        }
    }

    /// Set the gravity vector for the entire world. Box2D has no concept of an up direction and
    /// this is left as a decision for the application. Usually in m/s^2.
    pub fn set_gravity(&mut self, gravity: Vec2) {
        unsafe { sys::b2World_SetGravity(self.id, gravity.into()) }
    }

    /// Create a rigid body given a definition.
    pub fn create_body(&mut self, body_definition: &crate::BodyDefinition) -> BodyId {
        let body_id = unsafe { sys::b2CreateBody(self.id, &body_definition.0) };

        BodyId::from_b2(body_id)
    }

    /// Overlap test for circles.
    ///
    /// The callback will be called for each shape which overlaps with the provided circle. If the callback
    /// returns `Some(r)`, then we will stop iterating early and return `r`.
    ///
    /// Only shapes which pass `filter` are considered -- see [`QueryFilter`].
    /// If query stats are desired, call [`World::overlap_circle_with_stats`].
    pub fn overlap_circle<OverlapFn, R>(
        &self,
        circle_position: Vec2,
        radius: f32,
        filter: QueryFilter,
        overlap: OverlapFn,
    ) -> Option<R>
    where
        OverlapFn: FnMut(ShapeId) -> Option<R>,
    {
        self.overlap_circle_with_stats(circle_position, radius, filter, overlap)
            .1
    }

    /// Overlap test for circles.
    ///
    /// The callback will be called for each shape which overlaps with the provided circle. If the callback
    /// returns `Some(r)`, then we will stop iterating early and return `r` alongside the query stats.
    ///
    /// Only shapes which pass `filter` are considered -- see [`QueryFilter`].
    pub fn overlap_circle_with_stats<OverlapFn, R>(
        &self,
        circle_position: Vec2,
        radius: f32,
        filter: QueryFilter,
        overlap: OverlapFn,
    ) -> (OverlapStats, Option<R>)
    where
        OverlapFn: FnMut(ShapeId) -> Option<R>,
    {
        // safety: we are copying all data and we know that glam::Vec2 is the exact same as b2Vec2 so we
        // can make a pointer to it. Additionally, it survives this function entirely.
        let hit_circle = unsafe { sys::b2MakeProxy(&circle_position as *const Vec2 as *const sys::b2Vec2, 1, radius) };

        struct OverlapCtx<OverlapFn, R> {
            overlap: OverlapFn,
            result: Option<R>,
        }

        let mut ctx = OverlapCtx { overlap, result: None };

        extern "C" fn overlap_trampoline<OverlapFn, R>(shape: sys::b2ShapeId, cback: *mut std::ffi::c_void) -> bool
        where
            OverlapFn: FnMut(ShapeId) -> Option<R>,
        {
            // safety: Rust's type system promises that this is the same type of context
            // which we are passing. We *are* passing this context as an `&mut OverlapCtx<OverlapFn, R>`
            // when we call `sys::b2World_OverlapShape`
            let ctx: &mut OverlapCtx<OverlapFn, R> = unsafe { &mut *(cback as *mut OverlapCtx<OverlapFn, R>) };

            // call the guy! stop iterating (return false) once we have a result
            match (ctx.overlap)(ShapeId::from_b2(shape)) {
                Some(r) => {
                    ctx.result = Some(r);
                    false
                }
                None => true,
            }
        }

        // safety: the context is owned by us, and we can make a pointer to it, which we can cast to
        // `std::ffi::c_void`, which will get the context back eventually.
        let performance_stats = unsafe {
            sys::b2World_OverlapShape(
                self.id,
                &hit_circle,
                filter.0,
                Some(overlap_trampoline::<OverlapFn, R>),
                &mut ctx as *mut OverlapCtx<OverlapFn, R> as *mut std::ffi::c_void,
            )
        };

        let stats = OverlapStats {
            node_visits: performance_stats.nodeVisits,
            leaf_visits: performance_stats.leafVisits,
        };

        (stats, ctx.result)
    }

    /// Get contact events for this current time step.
    ///
    /// Note that contact events are opt-in per shape: a shape must be created with
    /// [`ShapeDefinition::enable_contact_events(true)`](crate::ShapeDefinition::enable_contact_events)
    /// or it will never appear here. Box2D leaves this off by default.
    pub fn contact_events(&self) -> impl Iterator<Item = (BodyId, BodyId)> + '_ {
        // safety: Box2D hands us its internal event buffer, which lives until the next step. The
        // buffer pointer is null when the world is locked, and a null pointer is not a valid empty
        // slice, so we check for it.
        let begin_events: &[sys::b2ContactBeginTouchEvent] = unsafe {
            let contact_events = sys::b2World_GetContactEvents(self.id);

            if contact_events.beginEvents.is_null() {
                &[]
            } else {
                std::slice::from_raw_parts(contact_events.beginEvents, contact_events.beginCount as usize)
            }
        };

        begin_events.iter().filter_map(|e| unsafe {
            if !sys::b2Shape_IsValid(e.shapeIdA) || !sys::b2Shape_IsValid(e.shapeIdB) {
                None
            } else {
                Some((
                    sys::b2Shape_GetBody(e.shapeIdA).into(),
                    sys::b2Shape_GetBody(e.shapeIdB).into(),
                ))
            }
        })
    }
}

impl Drop for World {
    fn drop(&mut self) {
        unsafe { sys::b2DestroyWorld(self.id) }
    }
}

/// A world definition holds all the data needed to construct a world.
///
/// You can safely re-use world definitions. World definitions are temporary objects used to
/// bundle creation parameters.
///
/// **NOTE: several defaults here are scaled by the global length units, so
/// [`set_length_units_per_meter`](crate::set_length_units_per_meter) must be called before you
/// build a definition, not merely before you build a [`World`].**
#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct WorldDefinition(sys::b2WorldDef);

impl WorldDefinition {
    /// Creates a new WorldDefinition, which is used to create a world.
    pub fn new() -> Self {
        Self(unsafe { sys::b2DefaultWorldDef() })
    }

    /// Gravity vector. Box2D has no up-vector defined. Usually in m/s^2.
    pub fn gravity(mut self, gravity: Vec2) -> Self {
        self.0.gravity = gravity.into();
        self
    }

    /// Restitution speed threshold, usually in m/s. Collisions above this speed have restitution
    /// applied (will bounce).
    pub fn restitution_threshold(mut self, restitution_threshold: f32) -> Self {
        self.0.restitutionThreshold = restitution_threshold;
        self
    }

    /// Threshold speed for hit events. Usually meters per second.
    pub fn hit_event_threshold(mut self, hit_event_threshold: f32) -> Self {
        self.0.hitEventThreshold = hit_event_threshold;
        self
    }

    /// Contact stiffness. Cycles per second.
    ///
    /// Increasing this increases the speed of overlap recovery, but can introduce jitter.
    pub fn contact_hertz(mut self, contact_hertz: f32) -> Self {
        self.0.contactHertz = contact_hertz;
        self
    }

    /// Contact bounciness. Non-dimensional.
    ///
    /// You can speed up overlap recovery by decreasing this with the trade-off that overlap
    /// resolution becomes more energetic.
    pub fn contact_damping_ratio(mut self, contact_damping_ratio: f32) -> Self {
        self.0.contactDampingRatio = contact_damping_ratio;
        self
    }

    /// This parameter controls how fast overlap is resolved and usually has units of meters per
    /// second. This only puts a cap on the resolution speed. The resolution speed is increased by
    /// increasing the hertz and/or decreasing the damping ratio.
    pub fn max_contact_push_speed(mut self, max_contact_push_speed: f32) -> Self {
        self.0.maxContactPushSpeed = max_contact_push_speed;
        self
    }

    /// Maximum linear speed. Usually meters per second.
    pub fn maximum_linear_speed(mut self, maximum_linear_speed: f32) -> Self {
        self.0.maximumLinearSpeed = maximum_linear_speed;
        self
    }

    /// Can bodies go to sleep to improve performance.
    pub fn enable_sleep(mut self, enable_sleep: bool) -> Self {
        self.0.enableSleep = enable_sleep;
        self
    }

    /// Enable continuous collision.
    pub fn enable_continuous(mut self, enable_continuous: bool) -> Self {
        self.0.enableContinuous = enable_continuous;
        self
    }

    /// Use this to store application specific world data.
    pub fn user_data(mut self, user_data: u64) -> Self {
        self.0.userData = user_data as _;
        self
    }
}

impl Default for WorldDefinition {
    fn default() -> Self {
        Self::new()
    }
}

/// Limits which shapes a world query considers.
///
/// A shape is only reported by a query when the shape's category is in the query's mask *and* the
/// query's category is in the shape's mask.
#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct QueryFilter(sys::b2QueryFilter);

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

/// These are performance results returned by dynamic tree queries."]
#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct OverlapStats {
    /// Number of internal nodes visited during the query"]
    pub node_visits: i32,

    /// Number of leaf nodes visited during the query"]
    pub leaf_visits: i32,
}
