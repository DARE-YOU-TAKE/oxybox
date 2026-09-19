/// These are performance results returned by dynamic tree queries."]
#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct OverlapStats {
    /// Number of internal nodes visited during the query"]
    pub node_visits: i32,

    /// Number of leaf nodes visited during the query"]
    pub leaf_visits: i32,
}
