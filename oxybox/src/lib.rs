mod body_id;
mod render;
mod shape_id;
mod world;

pub use body_id::*;
pub use render::{CircleDraw, DrawShapeCommand, PolygonDraw};
pub use shape_id::*;
pub use world::*;

pub use sys;

/// Sets the length units per meter Box2D will expect. While you're free to work with whatever units
/// you please, setting this will help Box2D tweak internal numbers to better work with your
/// expectations. For example, if your player character is 32 pixels high, then pass `32.0`, and you
/// may then confidently use pixels for all the length values you send to Box2D.
///
/// **NOTE: This is a global value -- Box2D does not support different unit lengths per-world.**
///
/// **WARNING: This should be set before any other call into Box2D. That includes
/// [`WorldDefinition::new`] and [`BodyDefinition::new`], whose defaults are scaled by this value,
/// not just [`World::new`].**
pub fn set_length_units_per_meter(length_units: f32) {
    unsafe { sys::b2SetLengthUnitsPerMeter(length_units) }
}

/// Get the current length units per meter. Defaults to `1.0`.
pub fn length_units_per_meter() -> f32 {
    unsafe { sys::b2GetLengthUnitsPerMeter() }
}
