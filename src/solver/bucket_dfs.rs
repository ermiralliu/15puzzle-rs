use rustc_hash::FxHashMap;

use crate::board::Board;
use super::Solution;

pub fn solve<B: Board>(initial: B) -> Option<Solution<B>> {
    let n = initial.n();
    if initial.is_goal() {
        return Some(Solution { moves: 0, path: vec![initial] });
    }
    if !initial.is_solvable() {
        return None;
    }

    let goal = B::goal(n);
    let init_key = initial.key();

    // child_key → (parent_key, ParentExtra)
    let mut parents: FxHashMap<B::Key, (B::Key, B::ParentExtra)> = FxHashMap::default();
    parents.insert(init_key.clone(), (init_key.clone(), initial.parent_extra()));

    // Queue entries: (board, parent_key)
    let mut current: Vec<(B, B::Key)> = vec![(initial, init_key)];
    let mut next_box: Vec<(B, B::Key)> = Vec::new();

    loop {
        while let Some((board, entry_parent)) = current.pop() {
            let board_key = board.key();

            if parents.contains_key(&board_key) {
                // Already inserted on a same-bucket push; parent is correct.
            } else {
                // Node came from next_box — insert with carried parent key.
                parents.insert(board_key.clone(), (entry_parent, board.parent_extra()));
            }

            if board.is_goal() {
                return Some(reconstruct(&parents, &board_key, n));
            }

            for neighbor in board.neighbors(&goal) {
                let nkey = neighbor.board.key();
                if parents.contains_key(&nkey) {
                    continue;
                }
                if neighbor.is_next_box {
                    next_box.push((neighbor.board, board_key.clone()));
                } else {
                    parents.insert(nkey.clone(), (board_key.clone(), neighbor.board.parent_extra()));
                    current.push((neighbor.board, board_key.clone()));
                }
            }
        }

        if next_box.is_empty() {
            return None;
        }
        std::mem::swap(&mut current, &mut next_box);
    }
}

fn reconstruct<B: Board>(
    parents: &FxHashMap<B::Key, (B::Key, B::ParentExtra)>,
    goal_key: &B::Key,
    n: usize,
) -> Solution<B> {
    let mut path = Vec::new();
    let mut key = goal_key.clone();
    loop {
        let (parent_key, extra) = &parents[&key];
        let board = B::rebuild(&key, extra, n);
        path.push(board);
        if parent_key == &key {
            break;
        }
        key = parent_key.clone();
    }
    path.reverse();
    let moves = (path.len() - 1) as u32;
    Solution { moves, path }
}
