use crate::mirrors_layout;

/// Surface materials allow chain shapes to have per segment surface properties
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SurfaceMaterial {
    /// The Coulomb (dry) friction coefficient, usually in the range `0.0..=1.0`.
    pub friction: f32,

    /// The coefficient of restitution (bounce) usually in the range `0.0..=1.0`.
    ///
    /// See [wikipedia](https://en.wikipedia.org/wiki/Coefficient_of_restitution).
    pub restitution: f32,

    /// The rolling resistance, usually in the range `0.0..=1.0`.
    pub rolling_resistance: f32,

    /// The tangent speed for conveyor belts.
    pub tangent_speed: f32,

    /// User material identifier.
    ///
    /// This is passed with query results and to the friction and restitution combining functions.
    /// Box2D does not use it internally.
    pub user_material_id: i32,

    /// Custom debug draw color.
    pub custom_color: u32,
}

mirrors_layout! {
    SurfaceMaterial => sys::b2SurfaceMaterial {
        friction => friction,
        restitution => restitution,
        rolling_resistance => rollingResistance,
        tangent_speed => tangentSpeed,
        user_material_id => userMaterialId,
        custom_color => customColor,
    }
}

impl SurfaceMaterial {
    /// Creates a new SurfaceMaterial filled in with the Box2D defaults
    pub fn new() -> Self {
        // safety: `SurfaceMaterial` and `b2SurfaceMaterial` have the same layout, which the
        // assertions above check field by field at compile time.
        unsafe { std::mem::transmute(sys::b2DefaultSurfaceMaterial()) }
    }
}

impl Default for SurfaceMaterial {
    fn default() -> Self {
        Self::new()
    }
}
