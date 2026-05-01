use rustc_hash::FxHashMap;

use crate::board::Board;
use super::Solution;

struct Entry<B: Board>(B, B::Key);

struct Side<B: Board> {
    current: Vec<Entry<B>>,
    next_box: Vec<Entry<B>>,
    /// child_key → parent_key (root points to itself)
    visited: FxHashMap<B::Key, B::Key>,
    bucket_index: u32,
    h_initial: u32,
    target: B,
}

impl<B: Board> Side<B> {
    fn f(&self) -> u32 {
        if self.current.is_empty() && self.next_box.is_empty() {
            u32::MAX
        } else if self.current.is_empty() {
            self.h_initial + 2 * (self.bucket_index + 1)
        } else {
            self.h_initial + 2 * self.bucket_index
        }
    }

    fn promote(&mut self) {
        std::mem::swap(&mut self.current, &mut self.next_box);
        self.bucket_index += 1;
    }
}

pub fn solve<B: Board>(initial: B) -> Option<Solution<B>> {
    let n = initial.n();
    let goal = B::goal(n);

    if initial.is_goal() {
        return Some(Solution { moves: 0, path: vec![initial] });
    }
    if !initial.is_solvable() {
        return None;
    }

    let init_h = initial.manhattan_to(&goal);
    let goal_h = goal.manhattan_to(&initial);

    let init_key = initial.key();
    let goal_key = goal.key();

    let mut fwd: Side<B> = {
        let mut visited = FxHashMap::default();
        visited.insert(init_key.clone(), init_key.clone());
        Side {
            current: vec![Entry(initial.clone(), init_key.clone())],
            next_box: Vec::new(),
            visited,
            bucket_index: 0,
            h_initial: init_h,
            target: goal.clone(),
        }
    };

    let mut bwd: Side<B> = {
        let mut visited = FxHashMap::default();
        visited.insert(goal_key.clone(), goal_key.clone());
        Side {
            current: vec![Entry(goal.clone(), goal_key.clone())],
            next_box: Vec::new(),
            visited,
            bucket_index: 0,
            h_initial: goal_h,
            target: initial.clone(),
        }
    };

    let mut mu = u32::MAX;
    let mut meet: Option<B::Key> = None;

    loop {
        let f_fwd = fwd.f();
        let f_bwd = bwd.f();

        let min_f = f_fwd.min(f_bwd);
        if min_f == u32::MAX || min_f >= mu {
            break;
        }

        // Pick side with smaller f; ties go to larger frontier
        let expand_fwd = if f_fwd != f_bwd {
            f_fwd < f_bwd
        } else {
            fwd.current.len() >= bwd.current.len()
        };

        if expand_fwd {
            expand_side(&mut fwd, &mut bwd, &mut mu, &mut meet);
        } else {
            expand_side(&mut bwd, &mut fwd, &mut mu, &mut meet);
        }
    }

    meet.map(|meet_key| {
        let fwd_path = walk_path::<B>(&fwd.visited, &meet_key, n);
        let bwd_path = walk_path::<B>(&bwd.visited, &meet_key, n);

        // fwd_path: [initial, ..., meet_board]
        // bwd_path: [goal, ..., meet_board]
        // Reverse bwd, skip meet duplicate at start
        let mut path = fwd_path;
        let mut bwd_reversed = bwd_path;
        bwd_reversed.reverse();
        path.extend(bwd_reversed.into_iter().skip(1));

        let moves = (path.len() - 1) as u32;
        Solution { moves, path }
    })
}

fn chain_len<K: Eq + std::hash::Hash + Clone>(visited: &FxHashMap<K, K>, key: &K) -> u32 {
    let mut len = 0;
    let mut k = key.clone();
    loop {
        let parent = &visited[&k];
        if parent == &k { break; }
        len += 1;
        k = parent.clone();
    }
    len
}

fn expand_side<B: Board>(
    side: &mut Side<B>,
    other: &mut Side<B>,
    mu: &mut u32,
    meet: &mut Option<B::Key>,
) {
    if side.current.is_empty() {
        if side.next_box.is_empty() {
            return;
        }
        side.promote();
    }

    let Entry(board, parent_key_of_this) = match side.current.pop() {
        Some(e) => e,
        None => return,
    };
    let board_key = board.key();

    // Insert on pop if not already in visited (handles nodes promoted from next_box)
    if !side.visited.contains_key(&board_key) {
        side.visited.insert(board_key.clone(), parent_key_of_this);
    }

    let g = chain_len(&side.visited, &board_key);

    // Meet check on popped node
    if other.visited.contains_key(&board_key) {
        let candidate = g + chain_len(&other.visited, &board_key);
        if candidate < *mu {
            *mu = candidate;
            *meet = Some(board_key.clone());
        }
    }

    for neighbor in board.neighbors(&side.target) {
        let nkey = neighbor.board.key();
        if side.visited.contains_key(&nkey) {
            continue;
        }

        // Meet detection on neighbor
        if other.visited.contains_key(&nkey) {
            let candidate = (g + 1) + chain_len(&other.visited, &nkey);
            if candidate < *mu {
                *mu = candidate;
                *meet = Some(nkey.clone());
            }
        }

        if neighbor.is_next_box {
            side.next_box.push(Entry(neighbor.board, board_key.clone()));
        } else {
            side.visited.insert(nkey.clone(), board_key.clone());
            side.current.push(Entry(neighbor.board, board_key.clone()));
        }
    }
}

fn walk_path<B: Board>(
    visited: &FxHashMap<B::Key, B::Key>,
    from: &B::Key,
    n: usize,
) -> Vec<B> {
    let mut path = Vec::new();
    let mut key = from.clone();
    loop {
        let parent_key = &visited[&key];
        let board = B::rebuild(&key, n);
        path.push(board);
        if parent_key == &key {
            break;
        }
        key = parent_key.clone();
    }
    path.reverse();
    path
}

// Threaded variant sketch (never compiled — reference only).
#[cfg(any())]
mod threaded {
    // Each side becomes a thread. Both sides own their own Side<B>.
    // Cross-side meet detection wraps each visited in Arc<RwLock<FxHashMap<...>>>.
    // mu and meet go into Arc<Mutex<(u32, Option<B::Key>)>>.
    // Each side exposes its current f via a shared AtomicU32.
    // A shared AtomicBool stop signals termination.
    // Reconstruction happens after both threads join.
    //
    // Trade-off: lock contention on visited maps can erase the parallelism
    // gain unless we shard or batch reads. Uncomment to opt in.
}
