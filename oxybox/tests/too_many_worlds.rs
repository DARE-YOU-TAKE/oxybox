//! Kept in its own test binary to test the global state.

use oxybox::*;

#[test]
fn creating_too_many_worlds_fails() {
    let wd = WorldDefinition::new();

    let _worlds: [World; 128] = std::array::from_fn(|_| World::new(wd));
    assert!(World::try_new(wd).is_err());
}
