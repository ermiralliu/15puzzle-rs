use rustc_hash::FxHashMap;

use crate::board::Board;
use super::Solution;

/// Entry in the search queue: (board, parent_key, g_value)
struct Entry<B: Board>(B, B::Key, u32);

struct Side<B: Board> {
    current: Vec<Entry<B>>,
    next_box: Vec<Entry<B>>,
    /// child_key → (parent_key, extra, g)
    visited: FxHashMap<B::Key, (B::Key, B::ParentExtra, u32)>,
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
        visited.insert(init_key.clone(), (init_key.clone(), initial.parent_extra(), 0));
        Side {
            current: vec![Entry(initial.clone(), init_key.clone(), 0)],
            next_box: Vec::new(),
            visited,
            bucket_index: 0,
            h_initial: init_h,
            target: goal.clone(),
        }
    };

    let mut bwd: Side<B> = {
        let mut visited = FxHashMap::default();
        visited.insert(goal_key.clone(), (goal_key.clone(), goal.parent_extra(), 0));
        Side {
            current: vec![Entry(goal.clone(), goal_key.clone(), 0)],
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

    let Entry(board, _parent_key_of_this, _g_of_this) = match side.current.pop() {
        Some(e) => e,
        None => return,
    };
    let board_key = board.key();

    // Insert on pop if not already in visited (handles nodes promoted from next_box)
    let g = if let Some(entry) = side.visited.get(&board_key) {
        entry.2
    } else {
        // This node came from next_box; _parent_key_of_this and _g_of_this are correct
        side.visited.insert(
            board_key.clone(),
            (_parent_key_of_this.clone(), board.parent_extra(), _g_of_this),
        );
        _g_of_this
    };

    // Meet check on popped node
    if let Some(other_entry) = other.visited.get(&board_key) {
        let candidate = g + other_entry.2;
        if candidate < *mu {
            *mu = candidate;
            *meet = Some(board_key.clone());
        }
    }

    let ng = g + 1;
    for neighbor in board.neighbors(&side.target) {
        let nkey = neighbor.board.key();
        if side.visited.contains_key(&nkey) {
            continue;
        }

        // Meet detection on neighbor
        if let Some(other_entry) = other.visited.get(&nkey) {
            let candidate = ng + other_entry.2;
            if candidate < *mu {
                *mu = candidate;
                *meet = Some(nkey.clone());
            }
        }

        if neighbor.is_next_box {
            // Don't insert into visited yet; store parent_key for later
            side.next_box.push(Entry(neighbor.board, board_key.clone(), ng));
        } else {
            // Insert immediately to prevent same-bucket duplicates
            side.visited.insert(
                nkey.clone(),
                (board_key.clone(), neighbor.board.parent_extra(), ng),
            );
            side.current.push(Entry(neighbor.board, board_key.clone(), ng));
        }
    }
}

fn walk_path<B: Board>(
    visited: &FxHashMap<B::Key, (B::Key, B::ParentExtra, u32)>,
    from: &B::Key,
    n: usize,
) -> Vec<B> {
    let mut path = Vec::new();
    let mut key = from.clone();
    loop {
        let (parent_key, extra, _g) = &visited[&key];
        let board = B::rebuild(&key, extra, n);
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
