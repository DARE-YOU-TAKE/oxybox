//! Kept in its own test binary: it holds every world slot Box2D has, which would make any test
//! running alongside it fail to create a world. Cargo runs test binaries one at a time.

use oxybox::*;

#[test]
fn creating_too_many_worlds_fails() {
    let wd = WorldDefinition::new();

    let _worlds: [World; 128] = std::array::from_fn(|_| World::new(wd));
    assert!(World::try_new(wd).is_err());
}
