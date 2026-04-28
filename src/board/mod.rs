pub mod dynamic;
pub mod packed;

pub use dynamic::DynamicBoard;
pub use packed::PackedBoard;

use crate::inversions;

pub struct Neighbor<B> {
    pub board: B,
    /// true when the moved tile's Manhattan distance to goal increased
    pub is_next_box: bool,
    /// h_new - h_old against the supplied target
    pub manhattan_delta: i32,
}

pub trait Board: Sized + Clone + Eq {
    type Key: Eq + std::hash::Hash + Clone;

    fn from_tiles(n: usize, tiles: &[u8]) -> Self;
    fn goal(n: usize) -> Self;

    fn n(&self) -> usize;
    fn key(&self) -> Self::Key;
    fn empty_index(&self) -> usize;
    fn tile_at(&self, idx: usize) -> u8;

    fn is_goal(&self) -> bool;

    fn is_solvable(&self) -> bool {
        let n = self.n();
        let total = n * n;
        let mut tiles: Vec<u8> = (0..total)
            .map(|i| self.tile_at(i))
            .filter(|&t| t != 0)
            .collect();
        let inv = inversions::count(&mut tiles);
        let empty_idx = self.empty_index();
        let empty_row_from_bottom = n - (empty_idx / n);
        if (total & 1) == 1 {
            inv % 2 == 0
        } else {
            (inv + empty_row_from_bottom as u32) % 2 == 1
        }
    }

    fn manhattan_to(&self, target: &Self) -> u32;

    fn neighbors<'a>(&'a self, target: &'a Self) -> impl Iterator<Item = Neighbor<Self>> + 'a;

    fn rebuild(key: &Self::Key, n: usize) -> Self;
}

/// Compute Manhattan distance of a single tile from position `from` to `to` on an n×n grid.
pub fn manhattan_dist(from: usize, to: usize, n: usize) -> u32 {
    let (r1, c1) = (from / n, from % n);
    let (r2, c2) = (to / n, to % n);
    (r1.abs_diff(r2) + c1.abs_diff(c2)) as u32
}
