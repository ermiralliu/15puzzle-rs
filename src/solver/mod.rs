pub mod bidirectional;
pub mod bucket_dfs;

use crate::board::Board;

pub struct Solution<B: Board> {
    pub moves: u32,
    pub path: Vec<B>,
}
