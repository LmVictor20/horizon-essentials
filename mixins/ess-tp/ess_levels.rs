use std::sync::Arc;
use horizon_client::chunk::Chunk;
use super::{Dim, Levels};

pub fn parked_chunk(levels: &Levels, dim: Dim, key: (i32, i32)) -> Option<Arc<Chunk>> {
    levels.parked.get(&dim).and_then(|p| p.world.chunks.get(&key).cloned())
}

pub fn parked_stats(levels: &Levels, dim: Dim) -> (usize, usize, usize) {
    levels.parked.get(&dim).map_or((0, 0, 0), |p| {
        (
            p.mobs.len(),
            p.world.chunks.len(),
            p.world.chunks.values().map(|c| c.block_entities.len()).sum(),
        )
    })
}
