use glam::Vec2;
use oxybox::*;

/// A fixed time step, as every simulation should use.
const DT: f32 = 1.0 / 60.0;

/// Box2D keeps every world in one global array and claims slots without synchronization:
/// `b2CreateWorld` scans for the first entry with `inUse == false` and sets it, and
/// `b2DestroyWorld` clears it. Two threads doing that at once can claim the same slot, and one
/// world is then silently reinitialized underneath the other. Cargo runs tests on several threads,
/// so every test here has to hold this lock for as long as it owns a world.
///
/// Declare the guard *first* in each test, so that it is dropped last -- after the `World` it is
/// protecting.
fn world_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    // ignore poison:
    LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// The ground sits with its top face at `y == 0`, so a ball of radius 5 comes to rest at `y == 5`.
fn ground_and_ball(world: &mut World, shape_def: &ShapeDefinition) -> (BodyId, BodyId) {
    let ground = world.create_body(&BodyDefinition::new().position(Vec2::new(0.0, -10.0)));
    ground.attach_rectangle(Vec2::new(50.0, 10.0), Vec2::ZERO, 0.0, shape_def);

    let ball = world.create_body(
        &BodyDefinition::new()
            .position(Vec2::new(0.0, 20.0))
            .kind(BodyKind::Dynamic),
    );
    ball.attach_circle(Vec2::ZERO, 5.0, &shape_def.restitution(0.0));

    (ground, ball)
}

#[test]
fn falling_ball() {
    let _guard = world_lock();

    let mut world = World::new(&WorldDefinition::new().gravity(Vec2::new(0.0, -10.0)));

    let (_ground, ball) = ground_and_ball(&mut world, &ShapeDefinition::default());

    for _ in 0..120 {
        world.step(DT);
    }

    let position = ball.position();
    assert!((5.0 - position.y).abs() < 1e-3, "ball at wrong position: {position:?}");
}

#[test]
fn shape_dimensions() {
    let _guard = world_lock();

    let mut world = World::new(&WorldDefinition::new());

    let body = world.create_body(&BodyDefinition::new());
    let rect = body.attach_rectangle(Vec2::new(3.0, 7.0), Vec2::ZERO, 0.0, &ShapeDefinition::default());
    let circle = body.attach_circle(Vec2::new(100.0, 100.0), 2.0, &ShapeDefinition::default());

    // half dimensions in, full dimensions out
    assert_eq!(rect.shape_kind(), ShapeKind::Polygon);
    assert!((rect.width() - 6.0).abs() < 1e-3, "{}", rect.width());
    assert!((rect.height() - 14.0).abs() < 1e-3, "{}", rect.height());

    // the circle's offset moves it, but must not change its extents
    assert_eq!(circle.shape_kind(), ShapeKind::Circle);
    assert_eq!(circle.dimensions(), Vec2::new(4.0, 4.0));
}

#[test]
fn overlap_circle_respects_the_query_filter() {
    let _guard = world_lock();
    const CATEGORY: u64 = 0b10;

    let mut world = World::new(&WorldDefinition::new());

    let body = world.create_body(&BodyDefinition::new());
    let shape = body.attach_circle(
        Vec2::ZERO,
        1.0,
        &ShapeDefinition::new().category(CATEGORY).mask(CATEGORY),
    );
    world.step(DT);

    // the default query has a category of 1, which this shape's mask excludes, so it must not
    // be reported even though it plainly overlaps
    let hit = world.overlap_circle(Vec2::ZERO, 1.0, QueryFilter::default(), |_| Some(()));
    assert_eq!(hit, None, "shape was reported despite a non-matching filter");

    // a query that the shape's mask accepts finds it
    let filter = QueryFilter::new().category(CATEGORY).mask(CATEGORY);
    let hit = world.overlap_circle(Vec2::ZERO, 1.0, filter, |s| (s == shape).then_some(()));
    assert_eq!(hit, Some(()), "shape was not reported despite a matching filter");
}

#[test]
fn contact_events_are_empty_before_stepping() {
    let _guard = world_lock();

    let world = World::new(&WorldDefinition::new());

    assert_eq!(world.contact_events().count(), 0);
}

#[test]
fn contact_events_report_touching_bodies() {
    let _guard = world_lock();

    let mut world = World::new(&WorldDefinition::new());
    world.set_gravity(Vec2::new(0.0, -10.0));

    let shape_def = ShapeDefinition::new().enable_contact_events(true);
    let (ground, ball) = ground_and_ball(&mut world, &shape_def);

    let mut contacts = Vec::new();
    for _ in 0..120 {
        world.step(DT);

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
fn world_definition_is_applied() {
    let _guard = world_lock();

    // a gravity far stronger than the box2d default of -10, so that a world built from a default
    // definition could not produce this result
    let mut world = World::new(&WorldDefinition::new().gravity(Vec2::new(0.0, -100.0)));

    let body = world.create_body(&BodyDefinition::new().kind(BodyKind::Dynamic));
    body.attach_circle(Vec2::ZERO, 1.0, &ShapeDefinition::new());
    world.step(DT);

    let velocity = body.linear_velocity();
    assert!(
        (velocity.y - (-100.0 * DT)).abs() < 1e-3,
        "definition gravity was not applied: {velocity:?}"
    );
}
