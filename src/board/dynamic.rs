use super::{Board, Neighbor, manhattan_dist};

/// N×N board for arbitrary N, backed by Box<[u8]>.
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct DynamicBoard {
    n: usize,
    tiles: Box<[u8]>,
    empty: usize,
}

impl DynamicBoard {
    #[inline]
    fn swap(&mut self, a: usize, b: usize) {
        self.tiles.swap(a, b);
    }
}

impl Board for DynamicBoard {
    type Key = Box<[u8]>;

    fn from_tiles(n: usize, tiles: &[u8]) -> Self {
        assert_eq!(tiles.len(), n * n);
        let empty = tiles.iter().position(|&t| t == 0).unwrap();
        DynamicBoard { n, tiles: tiles.into(), empty }
    }

    fn goal(n: usize) -> Self {
        let total = n * n;
        let mut tiles = Vec::with_capacity(total);
        for i in 1..total {
            tiles.push(i as u8);
        }
        tiles.push(0);
        DynamicBoard { n, tiles: tiles.into(), empty: total - 1 }
    }

    fn n(&self) -> usize { self.n }

    fn key(&self) -> Box<[u8]> { self.tiles.clone() }

    fn empty_index(&self) -> usize { self.empty }

    fn tile_at(&self, idx: usize) -> u8 { self.tiles[idx] }

    fn is_goal(&self) -> bool {
        let total = self.n * self.n;
        for i in 0..total - 1 {
            if self.tiles[i] != (i + 1) as u8 {
                return false;
            }
        }
        self.tiles[total - 1] == 0
    }

    fn manhattan_to(&self, target: &Self) -> u32 {
        let n = self.n;
        let total = n * n;
        let mut target_pos = vec![0usize; total + 1];
        for i in 0..total {
            let t = target.tiles[i] as usize;
            target_pos[t] = i;
        }
        let mut dist = 0u32;
        for i in 0..total {
            let t = self.tiles[i] as usize;
            if t != 0 {
                dist += manhattan_dist(i, target_pos[t], n);
            }
        }
        dist
    }

    fn neighbors<'a>(&'a self, target: &'a Self) -> impl Iterator<Item = Neighbor<Self>> + 'a {
        DynamicNeighborIter::new(self, target)
    }

    fn rebuild(key: &Box<[u8]>, n: usize) -> Self {
        let empty = key.iter().position(|&t| t == 0).unwrap();
        DynamicBoard { n, tiles: key.clone(), empty }
    }
}

struct DynamicNeighborIter<'a> {
    board: &'a DynamicBoard,
    target_pos: Vec<usize>,
    h_current: u32,
    dirs: [(i32, i32); 4],
    idx: usize,
}

impl<'a> DynamicNeighborIter<'a> {
    fn new(board: &'a DynamicBoard, target: &'a DynamicBoard) -> Self {
        let n = board.n;
        let total = n * n;
        let mut target_pos = vec![0usize; total + 1];
        for i in 0..total {
            let t = target.tiles[i] as usize;
            target_pos[t] = i;
        }
        let h_current = board.manhattan_to(target);
        DynamicNeighborIter {
            board,
            target_pos,
            h_current,
            dirs: [(-1, 0), (1, 0), (0, -1), (0, 1)],
            idx: 0,
        }
    }
}

impl<'a> Iterator for DynamicNeighborIter<'a> {
    type Item = Neighbor<DynamicBoard>;

    fn next(&mut self) -> Option<Self::Item> {
        let n = self.board.n as i32;
        let empty = self.board.empty as i32;
        let er = empty / n;
        let ec = empty % n;

        while self.idx < 4 {
            let (dr, dc) = self.dirs[self.idx];
            self.idx += 1;
            let nr = er + dr;
            let nc = ec + dc;
            if nr < 0 || nr >= n || nc < 0 || nc >= n {
                continue;
            }
            let neighbor_idx = (nr * n + nc) as usize;
            let tile = self.board.tiles[neighbor_idx] as usize;
            let mut new_board = self.board.clone();
            new_board.swap(self.board.empty, neighbor_idx);
            new_board.empty = neighbor_idx;

            let tile_goal = self.target_pos[tile];
            let old_dist = manhattan_dist(neighbor_idx, tile_goal, self.board.n) as i32;
            let new_dist = manhattan_dist(self.board.empty, tile_goal, self.board.n) as i32;
            let delta = new_dist - old_dist;
            let new_h = (self.h_current as i32 + delta) as u32;

            return Some(Neighbor {
                board: new_board,
                is_next_box: new_h > self.h_current,
                manhattan_delta: delta,
            });
        }
        None
    }
}
