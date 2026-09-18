use glam::Vec2;

mod overlap_stats;
mod query_filter;
mod world_definition;

pub use overlap_stats::OverlapStats;
pub use query_filter::QueryFilter;
pub use world_definition::{MixingCallbacks, TaskSystem, WorldDefinition};

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
    /// The default number of sub_steps to do in [`World::step`].
    pub const SUB_STEPS: u32 = 4;

    /// Create a world for rigid body simulation.
    pub fn new(world_definition: WorldDefinition) -> Self {
        // safety: `WorldDefinition` is laid out exactly like `b2WorldDef` (checked at compile time
        // where it is defined), so Box2D can read it in place -- nothing is copied or converted.
        let id = unsafe { sys::b2CreateWorld(world_definition.as_b2()) };
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
    ///
    /// `sub_steps`: Increasing the sub-step count can increase accuracy. Usually [`World::SUB_STEPS`]
    pub fn step(&mut self, delta_time: f32, sub_steps: u32) {
        unsafe {
            sys::b2World_Step(self.id, delta_time, sub_steps as i32);
        }
    }

    /// Set the gravity vector for the entire world. Box2D has no concept of an up direction and
    /// this is left as a decision for the application. Usually in m/s^2.
    pub fn set_gravity(&mut self, gravity: Vec2) {
        unsafe { sys::b2World_SetGravity(self.id, gravity.into()) }
    }

    /// Create a rigid body given a definition.
    pub fn create_body(&mut self, body_definition: crate::BodyDefinition) -> BodyId {
        // safety: `BodyDefinition` is laid out exactly like `b2BodyDef` (checked at compile time
        // where it is defined), so Box2D can read it in place -- nothing is copied or converted.
        let body_id = unsafe { sys::b2CreateBody(self.id, body_definition.as_b2()) };

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
                filter.as_b2(),
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
    /// [`ShapeDefinition::enable_contact_events`](crate::ShapeDefinition::enable_contact_events)
    /// set to `true` or it will never appear here. Box2D leaves this off by default.
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

impl Default for World {
    fn default() -> Self {
        Self::new(WorldDefinition::default())
    }
}
