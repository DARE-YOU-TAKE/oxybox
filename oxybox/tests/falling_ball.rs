use glam::Vec2;
use oxybox::*;

/// A fixed time step, as every simulation should use.
const DT: f32 = 1.0 / 60.0;

/// The ground sits with its top face at `y == 0`, so a ball of radius 5 comes to rest at `y == 5`.
fn ground_and_ball(world: &World, shape_def: ShapeDefinition) -> (BodyId, BodyId) {
    let ground = world.create_body(BodyDefinition {
        position: Vec2::new(0.0, -10.0),
        ..BodyDefinition::new()
    });
    ground.attach_rectangle(Vec2::new(50.0, 10.0), Vec2::ZERO, Rotation::IDENTITY, shape_def);

    let ball = world.create_body(BodyDefinition {
        position: Vec2::new(0.0, 20.0),
        kind: BodyKind::Dynamic,
        ..BodyDefinition::new()
    });
    ball.attach_circle(
        Vec2::ZERO,
        5.0,
        ShapeDefinition {
            material: SurfaceMaterial {
                restitution: 0.0,
                ..shape_def.material
            },
            ..shape_def
        },
    );

    (ground.id(), ball.id())
}

#[test]
fn falling_ball() {
    let mut world = World::new(WorldDefinition {
        gravity: Vec2::new(0.0, -10.0),
        ..WorldDefinition::new()
    });

    let (_ground, ball_id) = ground_and_ball(&world, ShapeDefinition::default());

    // the handle is taken once and read after stepping, exactly as a `b2BodyId` would be in C
    for _ in 0..120 {
        world.step(DT, World::SUB_STEPS);
    }

    let ball = world.body(ball_id).unwrap();
    let position = ball.position();
    assert!((5.0 - position.y).abs() < 1e-3, "ball at wrong position: {position:?}");
}

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
fn overlap_circle_respects_the_query_filter() {
    const CATEGORY: u64 = 0b10;

    let mut world = World::new(WorldDefinition::new());

    let body = world.create_body(BodyDefinition::new());
    let shape = body
        .attach_circle(
            Vec2::ZERO,
            1.0,
            ShapeDefinition {
                filter: Filter {
                    category_bits: CATEGORY,
                    mask_bits: CATEGORY,
                    ..Filter::new()
                },
                ..ShapeDefinition::new()
            },
        )
        .id();
    world.step(DT, World::SUB_STEPS);

    // the default query has a category of 1, which this shape's mask excludes, so it must not
    // be reported even though it plainly overlaps
    let hit = world.overlap_circle(Vec2::ZERO, 1.0, QueryFilter::default(), |_| Some(()));
    assert_eq!(hit, None, "shape was reported despite a non-matching filter");

    // a query that the shape's mask accepts finds it
    let filter = QueryFilter {
        category_bits: CATEGORY,
        mask_bits: CATEGORY,
    };
    let hit = world.overlap_circle(Vec2::ZERO, 1.0, filter, |s| (s.id() == shape).then_some(()));
    assert_eq!(hit, Some(()), "shape was not reported despite a matching filter");
}

#[test]
fn an_overlap_callback_can_read_what_it_is_handed() {
    const USER_DATA: usize = 0xBEEF;
    const BODY_POSITION: Vec2 = Vec2::new(3.0, 4.0);

    let mut world = World::new(WorldDefinition::new());

    let body = world.create_body(BodyDefinition {
        position: BODY_POSITION,
        ..BodyDefinition::new()
    });
    let shape = body.attach_circle(Vec2::ZERO, 2.0, ShapeDefinition::new());
    shape.set_user_data(USER_DATA);

    let body_id = body.id();
    let shape_id = shape.id();
    world.step(DT, World::SUB_STEPS);

    let found = world.overlap_circle(BODY_POSITION, 1.0, QueryFilter::default(), |s| {
        Some((s.id(), s.user_data(), s.dimensions(), s.body_id(), s.body().position()))
    });

    let (id, user_data, dimensions, hit_body, position) = found.expect("the shape overlaps the query circle");
    assert_eq!(id, shape_id);
    assert_eq!(user_data, USER_DATA);
    assert_eq!(dimensions, Vec2::new(4.0, 4.0), "a circle of radius 2 measures 4x4");
    assert_eq!(hit_body, body_id);
    assert_eq!(position, BODY_POSITION);
}

#[test]
fn contact_events_are_empty_before_stepping() {
    let world = World::new(WorldDefinition::new());

    assert_eq!(world.contact_events().count(), 0);
}

#[test]
fn contact_events_report_touching_bodies() {
    let mut world = World::new(WorldDefinition::new());
    world.set_gravity(Vec2::new(0.0, -10.0));

    let shape_def = ShapeDefinition {
        enable_contact_events: true,
        ..ShapeDefinition::new()
    };
    let (ground, ball) = ground_and_ball(&world, shape_def);

    let mut contacts = Vec::new();
    for _ in 0..120 {
        world.step(DT, World::SUB_STEPS);

        // collected per step: the events belong to the step that just ran
        contacts = world.contact_events().collect();
        if !contacts.is_empty() {
            break;
        }
    }

    assert_eq!(
        contacts.len(),
        1,
        "expected exactly one begin-touch event: {contacts:?}"
    );
    let (a, b) = contacts[0];
    assert!(
        (a == ground && b == ball) || (a == ball && b == ground),
        "unexpected contact pair: {a:?} / {b:?}"
    );
}

#[test]
fn names_reach_box2d() {
    let world = World::new(WorldDefinition::new());
    let name = std::ffi::CString::new("player").unwrap();

    let named = world.create_body(BodyDefinition {
        name: Some(BodyName::new(&name)),
        ..BodyDefinition::new()
    });
    let got = unsafe { std::ffi::CStr::from_ptr(sys::b2Body_GetName(named.id().into())) };
    assert_eq!(got.to_str().unwrap(), "player");

    // `None` must arrive as a null pointer, which Box2D stores as an empty name
    let anonymous = world.create_body(BodyDefinition::new());
    let got = unsafe { std::ffi::CStr::from_ptr(sys::b2Body_GetName(anonymous.id().into())) };
    assert_eq!(got.to_str().unwrap(), "");
}

#[test]
fn world_definition_is_applied() {
    // a gravity far stronger than the box2d default of -10, so that a world built from a default
    // definition could not produce this result
    let mut world = World::new(WorldDefinition {
        gravity: Vec2::new(0.0, -100.0),
        ..WorldDefinition::new()
    });

    let body = world.create_body(BodyDefinition {
        kind: BodyKind::Dynamic,
        ..BodyDefinition::new()
    });
    let _ = body.attach_circle(Vec2::ZERO, 1.0, ShapeDefinition::new());
    let body_id = body.id();
    world.step(DT, World::SUB_STEPS);

    let body = world.body(body_id).unwrap();
    let velocity = body.linear_velocity();
    assert!(
        (velocity.y - (-100.0 * DT)).abs() < 1e-3,
        "definition gravity was not applied: {velocity:?}"
    );
}

#[test]
fn ids_from_another_world_are_rejected() {
    let owner = World::new(WorldDefinition::new());
    let mut other = World::new(WorldDefinition::new());

    let body = owner.create_body(BodyDefinition::new());
    let shape_id = body.attach_circle(Vec2::ZERO, 1.0, ShapeDefinition::new()).id();

    let body_id = body.id();

    // the id is perfectly valid -- it just does not name anything in `other`
    assert!(body_id.is_valid());
    assert!(shape_id.is_valid());

    assert!(other.body(body_id).is_none(), "a foreign body id was accepted");
    assert!(other.shape(shape_id).is_none(), "a foreign shape id was accepted");
    assert!(!other.destroy_body(body_id), "a foreign body was destroyed");

    // and the owning world still hands them out
    assert!(owner.body(body_id).is_some());
    assert!(owner.shape(shape_id).is_some());
}

#[test]
fn destroying_a_body_invalidates_its_handles() {
    let mut world = World::new(WorldDefinition::new());

    let body = world.create_body(BodyDefinition::new());
    let shape_id = body.attach_circle(Vec2::ZERO, 1.0, ShapeDefinition::new()).id();
    let body_id = body.id();

    assert!(world.destroy_body(body_id), "the body should have been destroyed");

    // destroying a body takes its shapes with it, and neither handle is handed out again
    assert!(world.body(body_id).is_none());
    assert!(world.shape(shape_id).is_none());

    // a second destroy is a no-op rather than a double free
    assert!(!world.destroy_body(body_id));
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
