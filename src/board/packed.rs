use super::{Board, Neighbor, manhattan_dist};

/// 4×4 board stored as a u64: tile at index i lives in nibble i (bits 4i..4i+4).
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct PackedBoard {
    tiles: u64,
    empty: u8, // index of the blank (0..16)
}

impl PackedBoard {
    #[inline]
    fn get(&self, idx: usize) -> u8 {
        ((self.tiles >> (idx * 4)) & 0xF) as u8
    }

    #[inline]
    fn set(&mut self, idx: usize, val: u8) {
        let shift = idx * 4;
        self.tiles = (self.tiles & !(0xF_u64 << shift)) | ((val as u64) << shift);
    }

    fn swap(&mut self, a: usize, b: usize) {
        let va = self.get(a);
        let vb = self.get(b);
        self.set(a, vb);
        self.set(b, va);
    }
}

impl Board for PackedBoard {
    type Key = u64;
    type ParentExtra = ();

    fn from_tiles(n: usize, tiles: &[u8]) -> Self {
        assert_eq!(n, 4);
        assert_eq!(tiles.len(), 16);
        let mut packed = 0u64;
        let mut empty = 0u8;
        for (i, &t) in tiles.iter().enumerate() {
            packed |= (t as u64) << (i * 4);
            if t == 0 {
                empty = i as u8;
            }
        }
        PackedBoard { tiles: packed, empty }
    }

    fn goal(n: usize) -> Self {
        assert_eq!(n, 4);
        // Goal: 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 0
        let tiles: Vec<u8> = (1..=15).chain(std::iter::once(0)).collect();
        Self::from_tiles(4, &tiles)
    }

    fn n(&self) -> usize { 4 }

    fn key(&self) -> u64 { self.tiles }

    fn empty_index(&self) -> usize { self.empty as usize }

    fn tile_at(&self, idx: usize) -> u8 { self.get(idx) }

    fn is_goal(&self) -> bool {
        // Check tiles 0..14 are 1..15 and tile 15 is 0
        for i in 0..15usize {
            if self.get(i) != (i + 1) as u8 {
                return false;
            }
        }
        self.get(15) == 0
    }

    fn manhattan_to(&self, target: &Self) -> u32 {
        let n = 4;
        // Build position lookup for target: target_pos[tile] = index
        let mut target_pos = [0usize; 16];
        for i in 0..16 {
            let t = target.get(i) as usize;
            target_pos[t] = i;
        }
        let mut dist = 0u32;
        for i in 0..16 {
            let t = self.get(i) as usize;
            if t != 0 {
                dist += manhattan_dist(i, target_pos[t], n);
            }
        }
        dist
    }

    fn neighbors<'a>(&'a self, target: &'a Self) -> impl Iterator<Item = Neighbor<Self>> + 'a {
        PackedNeighborIter::new(self, target)
    }

    fn parent_extra(&self) -> () { () }

    fn rebuild(key: &u64, _extra: &(), _n: usize) -> Self {
        let mut empty = 0u8;
        for i in 0..16 {
            if ((key >> (i * 4)) & 0xF) == 0 {
                empty = i as u8;
                break;
            }
        }
        PackedBoard { tiles: *key, empty }
    }
}

struct PackedNeighborIter<'a> {
    board: &'a PackedBoard,
    #[allow(dead_code)]
    target: &'a PackedBoard,
    /// goal-position lookup: goal_pos[tile] = index in goal
    target_pos: [usize; 16],
    /// current h of board against target
    h_current: u32,
    dirs: [(i32, i32); 4],
    idx: usize,
}

impl<'a> PackedNeighborIter<'a> {
    fn new(board: &'a PackedBoard, target: &'a PackedBoard) -> Self {
        let mut target_pos = [0usize; 16];
        for i in 0..16 {
            let t = target.get(i) as usize;
            target_pos[t] = i;
        }
        let h_current = board.manhattan_to(target);
        PackedNeighborIter {
            board,
            target,
            target_pos,
            h_current,
            dirs: [(-1, 0), (1, 0), (0, -1), (0, 1)],
            idx: 0,
        }
    }
}

impl<'a> Iterator for PackedNeighborIter<'a> {
    type Item = Neighbor<PackedBoard>;

    fn next(&mut self) -> Option<Self::Item> {
        let n = 4i32;
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
            let tile = self.board.get(neighbor_idx);
            let mut new_board = self.board.clone();
            new_board.swap(self.board.empty as usize, neighbor_idx);
            new_board.empty = neighbor_idx as u8;

            // Compute Manhattan delta for the moved tile
            // tile moved from neighbor_idx to old empty position
            let tile_goal = self.target_pos[tile as usize];
            let old_dist = manhattan_dist(neighbor_idx, tile_goal, 4) as i32;
            let new_dist = manhattan_dist(self.board.empty as usize, tile_goal, 4) as i32;
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
