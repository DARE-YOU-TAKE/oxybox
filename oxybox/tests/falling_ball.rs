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
