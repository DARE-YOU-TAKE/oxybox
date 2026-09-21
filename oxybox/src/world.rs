use std::{
    cell::Cell,
    marker::PhantomData,
    sync::{Mutex, MutexGuard},
};

use glam::Vec2;

mod overlap_stats;
mod query_filter;
mod world_definition;

pub use overlap_stats::OverlapStats;
pub use query_filter::QueryFilter;
pub use world_definition::{MixingCallbacks, TaskSystem, WorldDefinition};

use crate::{Body, BodyId, Shape, ShapeId, ShapeRef};

/// Box2D keeps every world in one global array and claims slots without synchronization:
/// `b2CreateWorld` scans for the first entry with `inUse == false` and sets it, and
/// `b2DestroyWorld` clears it. Two threads doing that at once can claim the same slot, and one
/// world is then silently reinitialized underneath the other. Every world creation and
/// destruction holds this lock.
static WORLD_LOCK: Mutex<()> = Mutex::new(());

pub(crate) fn world_lock() -> MutexGuard<'static, ()> {
    // the lock guards no data, so a poisoned lock has nothing broken to report -- and this is
    // taken in `World::drop`, which must not panic.
    WORLD_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// A physics world.
///
/// A world contains bodies, shapes, and constraints. You may create up to 128
/// worlds. Each world is completely independent and may be simulated in parallel.
///
/// # Thread safety
///
/// A `World` is [`Send`] but not [`Sync`]: one may be moved to another thread and simulated
/// there, and two worlds may be stepped in parallel, but a single world must only ever be touched
/// by one thread at a time. Worlds may be created and dropped from any number of threads at once.
///
/// We allocate a global mutex which we use to sync World creation, since Box2D stores Worlds in its own
/// global array. This prevents making two Worlds at once on two different threads, which may both be assigned
/// to the same location in memory, leading to UB later. This mutex is only accessed on World creation and
/// when World drops, so if you only make one World and only drop it at the end of the program's life, then
/// this mutex will not be a serious concern.
#[derive(Debug)]
pub struct World {
    pub(crate) id: sys::b2WorldId,

    /// Box2D does not synchronize access to a world, and every method here takes `&self`, so two
    /// threads sharing a `&World` could race inside Box2D. This makes `World` `!Sync` while
    /// leaving it `Send`, since moving a whole world between threads is fine.
    not_sync: PhantomData<Cell<()>>,
}

impl World {
    /// The default number of sub_steps to do in [`World::step`].
    pub const SUB_STEPS: u32 = 4;

    /// Create a world for rigid body simulation.
    ///
    /// # Panics
    ///
    /// Panics if more than 128 worlds exist at once. You can use [`World::try_new`] to handle
    /// this panic manually.
    pub fn new(world_definition: WorldDefinition) -> Self {
        Self::try_new(world_definition).unwrap()
    }

    /// Create a world for rigid body simulation.
    pub fn try_new(world_definition: WorldDefinition) -> Result<Self, TooManyWorlds> {
        let _guard = world_lock();

        // safety: `WorldDefinition` is laid out exactly like `b2WorldDef`
        let id = unsafe { sys::b2CreateWorld(world_definition.as_b2()) };

        let is_valid = unsafe { sys::b2World_IsValid(id) };
        if is_valid {
            Ok(Self {
                id,
                not_sync: PhantomData,
            })
        } else {
            Err(TooManyWorlds)
        }
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
    pub fn set_gravity(&self, gravity: Vec2) {
        unsafe { sys::b2World_SetGravity(self.id, gravity.into()) }
    }

    /// Create a rigid body given a definition.
    pub fn create_body(&self, body_definition: crate::BodyDefinition) -> Body<'_> {
        // safety: `BodyDefinition` is laid out exactly like `b2BodyDef` (checked at compile time
        // where it is defined), so Box2D can read it in place -- nothing is copied or converted.
        let body_id = unsafe { sys::b2CreateBody(self.id, body_definition.as_b2()) };

        Body::new(BodyId::from_b2(body_id))
    }

    /// Overlap test for circles.
    ///
    /// The callback will be called for each shape which overlaps with the provided circle. If the callback
    /// returns `Some(r)`, then we will stop iterating early and return `r`.
    ///
    /// Only shapes which pass `filter` are considered -- see [`QueryFilter`].
    /// If query stats are desired, call [`World::overlap_circle_with_stats`].
    pub fn overlap_circle<OverlapFn, R>(
        &mut self,
        circle_position: Vec2,
        radius: f32,
        filter: QueryFilter,
        overlap: OverlapFn,
    ) -> Option<R>
    where
        OverlapFn: FnMut(ShapeRef<'_>) -> Option<R>,
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
        &mut self,
        circle_position: Vec2,
        radius: f32,
        filter: QueryFilter,
        overlap: OverlapFn,
    ) -> (OverlapStats, Option<R>)
    where
        OverlapFn: FnMut(ShapeRef<'_>) -> Option<R>,
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
            OverlapFn: FnMut(ShapeRef<'_>) -> Option<R>,
        {
            // safety: Rust's type system promises that this is the same type of context
            // which we are passing. We *are* passing this context as an `&mut OverlapCtx<OverlapFn, R>`
            // when we call `sys::b2World_OverlapShape`
            let ctx: &mut OverlapCtx<OverlapFn, R> = unsafe { &mut *(cback as *mut OverlapCtx<OverlapFn, R>) };

            // Box2D only hands the callback shapes it just found in the tree, so this one is live,
            // and the world outlives the query it is running inside of.
            let shape_ref = ShapeRef::new(ShapeId::from_b2(shape));
            match (ctx.overlap)(shape_ref) {
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

    /// Gets a given [`Shape`] from an existing [`ShapeId`].
    ///
    /// Returns `None` if the shape has been destroyed, or belongs to a different world.
    pub fn shape(&self, shape_id: ShapeId) -> Option<Shape<'_>> {
        self.owns_shape(shape_id).then(|| Shape::new(shape_id))
    }

    /// Gets a given [`Body`] from an existing [`BodyId`].
    ///
    /// Returns `None` if the body has been destroyed, or belongs to a different world.
    pub fn body(&self, body_id: BodyId) -> Option<Body<'_>> {
        self.owns_body(body_id).then(|| Body::new(body_id))
    }

    /// Destroy a rigid body. This destroys all shapes and joints attached to the body.
    ///
    /// Returns `false` if the body was already destroyed, or belongs to a different world.
    pub fn destroy_body(&mut self, body_id: BodyId) -> bool {
        if !self.owns_body(body_id) {
            return false;
        }

        unsafe { sys::b2DestroyBody(body_id.0) };
        true
    }

    /// Whether `body_id` names a live body in *this* world.
    ///
    /// If you make and destroy a world, the old body ids from the past world may overlap
    /// (ie, break the A-B-A problem) from bodies in the new world -- Box2d only exposes world
    /// slot index, but not generation data. If you never or rarely delete worlds, you don't have
    /// to worry about it.
    pub fn owns_body(&self, body_id: BodyId) -> bool {
        self.id.index1.wrapping_sub(1) == body_id.0.world0 && body_id.is_valid()
    }

    /// Whether `shape_id` names a live shape in *this* world.
    ///
    /// If you make and destroy a world, the old shape ids from the past world may overlap
    /// (ie, break the A-B-A problem) from shapes in the new world -- Box2d only exposes world
    /// slot index, but not generation data. If you never or rarely delete worlds, you don't have
    /// to worry about it.
    pub fn owns_shape(&self, shape_id: ShapeId) -> bool {
        self.id.index1.wrapping_sub(1) == shape_id.0.world0 && shape_id.is_valid()
    }
}

impl Drop for World {
    fn drop(&mut self) {
        // Box2D frees the world's slot here, which would race another thread claiming one
        let _guard = world_lock();

        unsafe { sys::b2DestroyWorld(self.id) }
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new(WorldDefinition::default())
    }
}

#[derive(Debug, thiserror::Error)]
#[error("could not make new world; only 128 may exist at once")]
pub struct TooManyWorlds;
