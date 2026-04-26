# Port my8puzzle to Rust + Bidirectional A*

## Context

`ermiralliu/my8puzzle` is a small C++20 sliding-puzzle solver (~1k LoC). It is
templated on `N` (board side) but in practice runs at `N=4` (15-puzzle). Its
"solver" is not classic A*: it keeps two stacks — a current bucket and a "next
box" — and pushes a neighbor to the next box only if the moved tile's Manhattan
distance went up, otherwise it stays in the current bucket. When the current
bucket empties, the next box is swapped in. A `unordered_set<Tiles>` is the
visited set. `makeHistory()` is stubbed; the original never reconstructs the
move sequence.

The task is two-fold:

1. **Port** the puzzle code (skipping `Repositories/` and `Entities/`, both of
   which are commented-out / DB-only) to idiomatic Rust, focused on the
   15-puzzle (4×4) but also runnable on arbitrary N.
2. **Add a bidirectional A* solver** that searches forward from the start and
   backward from the goal simultaneously, meeting in the middle. The puzzle is
   reversible, so the same neighbor function works for both directions.

The Rust port should also fix a few rough edges from the original: (a) actually
reconstruct and print the move sequence, (b) provide both a packed 4×4 board
(8-byte nibble layout = `u64`) and a dynamic-N board behind a single trait so
the same solver functions work on either, (c) update Manhattan incrementally on
each move so A* doesn't recompute the full sum every node.

## Crate layout

```
15puzzle-rs/
├── Cargo.toml                # bin + lib, edition 2021, dep: rustc_hash
├── README.md                 # brief usage
└── src/
    ├── lib.rs                # re-exports Board trait, solvers
    ├── main.rs               # CLI: parse files, dispatch by N, time + print
    ├── board/
    │   ├── mod.rs            # `Board` trait, `Neighbor` struct, common helpers
    │   ├── packed.rs         # `PackedBoard` for 4×4, key = u64 nibbles
    │   └── dynamic.rs        # `DynamicBoard` for arbitrary N, key = Box<[u8]>
    ├── solver/
    │   ├── mod.rs            # public re-exports + `Solution` struct
    │   ├── bucket_dfs.rs     # faithful port of two-stack bucket DFS
    │   └── bidirectional.rs  # bidirectional bucketed Manhattan-DFS (= bidir A*)
    └── inversions.rs         # merge-sort inversion count for solvability
```

**Hashing.** All `HashMap`s in the solvers are
`rustc_hash::FxHashMap<B::Key, ...>`. `FxHashMap` uses the FxHash function —
non-cryptographic, very fast on integer-shaped keys, which is exactly what we
have (`u64` for `PackedBoard`, short `Box<[u8]>` for `DynamicBoard`).
`Cargo.toml` dep: `rustc-hash = "2"` (one tiny dep, no transitive deps).

Single binary, dispatched in `main.rs`:

- `n == 4`  →  `PackedBoard`
- otherwise →  `DynamicBoard`

A `--solver {bucket,bidir}` CLI flag (default `bidir`) chooses which solver to
run on each input file.

## The `Board` trait — one solver, two boards

The whole point of the trait is that each solver function is generic over
`B: Board`, so we write `bucket_dfs::solve` and `bidirectional::solve` exactly
once and they work for both the packed 4×4 board and the dynamic-N board.

```rust
// src/board/mod.rs
pub trait Board: Sized + Clone + Eq {
    /// Canonical, hashable encoding used as the HashMap key.
    /// PackedBoard → u64; DynamicBoard → Box<[u8]>.
    type Key: Eq + std::hash::Hash + Clone;

    fn from_tiles(n: usize, tiles: &[u8]) -> Self;
    fn goal(n: usize) -> Self;

    fn n(&self) -> usize;
    fn key(&self) -> Self::Key;
    fn empty_index(&self) -> usize;
    fn tile_at(&self, idx: usize) -> u8;

    fn is_goal(&self) -> bool;
    fn is_solvable(&self) -> bool;          // uses inversions::count

    /// Manhattan distance from this board to the given target board.
    /// Used to seed h() for A* (forward h = manhattan(self, goal),
    /// backward h = manhattan(self, start)).
    fn manhattan_to(&self, target: &Self) -> u32;

    /// Yields up to 4 neighbors. Each neighbor reports:
    ///   - the new board,
    ///   - whether Manhattan-to-goal *increased* on this move (`is_next_box`,
    ///     used by bucket_dfs),
    ///   - the signed Manhattan delta against an arbitrary target
    ///     (used by bidir A* to update h() incrementally).
    fn neighbors<'a>(&'a self, target: &'a Self) -> impl Iterator<Item = Neighbor<Self>> + 'a;
}

pub struct Neighbor<B> {
    pub board: B,
    pub is_next_box: bool,    // matches the C++ `BoardDtos::isNextBox`
    pub manhattan_delta: i32, // h_new - h_old, against the supplied target
}
```

`Neighbor` is the Rust analogue of `BoardDtos<N>` from
`/tmp/my8puzzle/src/Models/Board.hpp:32`, just enriched with the signed delta
so A* can do incremental h updates.

### `PackedBoard` (4×4, `src/board/packed.rs`)

Mirrors the `Tiles<4>` specialization at
`/tmp/my8puzzle/src/Models/Tiles.hpp:58-118`. Internally a single `u64`: tile
at index `i` lives in nibble `i` (bits `4i .. 4i+4`). Get/set are masked
shifts. Empty index is also stored explicitly so we don't have to scan to find
the 0.

`Key = u64` — the packed representation is itself the canonical hash key. No
collisions possible because every distinct board has a distinct `u64`.

### `DynamicBoard` (arbitrary N, `src/board/dynamic.rs`)

Mirrors the generic `Tiles<N>` template. Backing storage `Box<[u8]>` of length
`n*n`. `Key = Box<[u8]>` (cheap to clone, cheap to hash).

## Bucket-DFS solver — faithful port

`src/solver/bucket_dfs.rs`. One function:

```rust
pub fn solve<B: Board>(initial: B) -> Option<Solution<B>>;
```

Algorithm follows `/tmp/my8puzzle/src/Services/SolverService.cpp:20-71`:

1. `current: Vec<B> = vec![initial]` (stack).
2. `next_box: Vec<B> = vec![]`.
3. `parents: FxHashMap<B::Key, ParentEntry<B>> = FxHashMap::default()` — **dual-
   purpose**: it is both the visited set and the parent-pointer map for
   reconstruction. The map direction is `child_key → parent_entry`, so:
   - **"Have we seen this board?"** is a single `parents.contains_key(&v.key())`
     — O(1). No traversal. The map is keyed by the child, never searched by
     the parent.
   - **Reconstruction** walks `parents` from the goal's child key back to the
     start by repeatedly looking up the parent key — also O(1) per hop.

   Per the user's rule:
   - Insert when a board is *popped* from `current` and about to be expanded.
   - Insert when a *new* neighbor is pushed onto `current` (same bucket,
     Manhattan didn't go up).
   - Do **not** insert when a neighbor is pushed onto `next_box`.

   `ParentEntry<B>` shape:
   - For `PackedBoard` (`Key = u64`), the user's exact `HashMap<u64, u64>` is
     enough — the key *is* the board, so reconstruction can rebuild boards
     from keys without storing them.
   - For `DynamicBoard` (`Key = Box<[u8]>`), we store `(parent_key, B)` so
     reconstruction doesn't need to re-derive the board.

   To keep one solver implementation, expose this as an associated type on
   the `Board` trait:

   ```rust
   trait Board {
       type Key: Eq + std::hash::Hash + Clone;
       /// Stored alongside the parent key in the visited map.
       /// `()` for PackedBoard (key fully encodes board); `Self` for DynamicBoard.
       type ParentExtra: Clone;
       fn parent_extra(&self) -> Self::ParentExtra;
       fn rebuild(key: &Self::Key, extra: &Self::ParentExtra, n: usize) -> Self;
   }
   ```

   Then `parents: FxHashMap<B::Key, (B::Key, B::ParentExtra)>` works
   uniformly: `()` collapses to `HashMap<u64, u64>` for the packed case
   (Rust's `(u64, ())` has the same layout as `u64`).
4. The initial board is inserted with itself as parent (sentinel meaning "no
   parent").
5. Outer loop: while `current` non-empty, pop, check goal, insert into
   `parents` if not already, expand neighbors, route to `current` or `next_box`
   per `Neighbor::is_next_box`. When `current` empties, swap with `next_box`.
6. On goal hit, walk `parents` backward by `B::Key` to reconstruct the path
   (O(L) hops, each O(1) lookup).

`Solution<B> { moves: u32, path: Vec<B> }`.

This is a faithful port of the original behavior plus two intentional
improvements that the user asked for:

- Visited insertion happens both on pop *and* on push-to-current-bucket (the
  C++ only inserts on pop).
- The path is actually reconstructed (the C++ `makeHistory` was stubbed at
  `/tmp/my8puzzle/src/Services/SolverService.cpp:73-82`).

## Bidirectional A* — `src/solver/bidirectional.rs`

```rust
pub fn solve<B: Board>(initial: B) -> Option<Solution<B>>;
```

**Key insight (no priority queue needed).** With the Manhattan heuristic on
this puzzle, every move changes `g` by +1 and `h` by ±1, so `f = g + h` changes
by exactly **0 or +2**. That means the open list of an A* search splits
cleanly into a sequence of buckets where bucket `k` contains every node with
`f = f_initial + 2k`. Expanding within a bucket only ever generates nodes in
the same bucket (good move) or the next one (bad move). This is *exactly* the
two-stack `current` / `next_box` structure of `bucket_dfs` — bucketed
Manhattan-DFS **is** A* with `h = Manhattan`, but with O(1) push/pop instead of
heap log-N. So the bidirectional solver reuses the same bucket machinery on
both sides; no `BinaryHeap` anywhere.

### State

Per side (`fwd` and `bwd`, structurally identical):

```rust
struct Side<B: Board> {
    current: Vec<B>,                                       // current bucket stack
    next_box: Vec<B>,                                      // next bucket stack
    // child_key → (parent_key, board-extra, depth/g)
    visited: FxHashMap<B::Key, (B::Key, B::ParentExtra, u32)>,
    bucket_index: u32,                                     // promotions so far
    h_initial: u32,                                        // h-value of side's start
}
```

Lookup direction is `child → parent` (same as `bucket_dfs`), so the
"have-we-seen-this-board" check is a single `visited.contains_key(&v.key())`
— O(1). We store `g` alongside the parent so meet checks (g_fwd + g_bwd) are
also O(1), no chain walking. `f` of any node currently in `current` equals
`h_initial + 2 * bucket_index`, so per-node `f` is implicit.

### Setup

- `fwd.current = vec![initial]`, `fwd.h_initial = initial.manhattan_to(&goal)`.
- `bwd.current = vec![goal]`,    `bwd.h_initial = goal.manhattan_to(&initial)`.
- Insert each side's start into its `visited` with `parent = key(self)`
  (sentinel for "root").
- `mu: u32 = u32::MAX`, `meet: Option<B::Key> = None`.

### Loop

1. Compute `f_fwd = fwd.h_initial + 2*fwd.bucket_index` if `fwd.current`
   non-empty, else `u32::MAX`. Same for `f_bwd`. (Note: nodes still sitting in
   `next_box` belong to bucket `bucket_index + 1`, so their f is at least
   `h_initial + 2*(bucket_index+1)` — accounted for after promotion.)
2. **Termination.** If `min(f_fwd, f_bwd) >= mu`, stop. The cheapest still-
   reachable node on either frontier already costs at least `mu`, so any
   future-found meet would be `>= mu` (this uses consistency of Manhattan).
3. Pick the side `S` with the smaller `f_S`; ties go to the smaller frontier.
4. If `S.current` is empty: promote — swap `current` with `next_box`,
   `bucket_index += 1`, then loop. If both `current` and `next_box` are
   empty on this side, declare exhaustion (only happens on unsolvable
   inputs; we checked `is_solvable()` first, so this is a safety net).
5. Pop board `u` from `S.current`. Its `g` is read straight from
   `S.visited[&u.key()].2`, no traversal.
6. **Meet check on the popped node.** If `u.key()` is in the *other* side's
   `visited`, candidate `mu' = u.g + other.visited[&u.key()].2` — O(1) using
   the depth we stored alongside the parent. Update `(mu, meet)` if smaller.
7. Expand `u`'s neighbors using `u.neighbors(target)` where `target` is the
   *opposite* side's start (so `manhattan_delta` is signed against this
   side's heuristic target). For each neighbor `v`:
   - If `v.key()` is already in `S.visited`, skip.
   - **Meet detection on the frontier.** If `v.key()` is in the other
     side's `visited`, compute candidate `mu'` and update `(mu, meet)` if
     lower. Whether or not it's a meet, still route `v` per the next step.
   - Insert `v` into `S.visited` only if it goes onto `S.current`
     (`is_next_box == false`). Per the user's rule from `bucket_dfs`: do
     **not** insert into visited when pushing to `next_box`.
   - Push `v` onto `S.current` or `S.next_box` per `is_next_box`.

### Reconstruction

When the loop terminates with `meet = Some(k)`:

- Walk forward parent pointers from `k` to `initial.key()`, collecting boards
  → reverse → forward half.
- Walk backward parent pointers from `k` to `goal.key()`, collecting boards
  → that's the backward half (already in goal-direction order, so don't
  reverse — these are already the steps from `meet` toward `goal`).
- Concatenate forward half + backward half (excluding the duplicated `meet`).
- `Solution { moves: mu, path }`.

### Why this is correct

- Bucket `k` on side `S` contains exactly the nodes with `f_S = h_S_initial +
  2k`. All such nodes have admissible-and-consistent estimates of distance to
  `S`'s target.
- The Pohl termination `min(f_fwd, f_bwd) >= mu` is sound for consistent `h`:
  any not-yet-expanded node on either side has cost-to-target >= 0, so any
  meet found through it has total cost >= `f` of that node >= `mu`.
- Meet detection on neighbor generation (not just on pop) catches the case
  where both sides queue the same board on the same iteration — without it,
  one side could keep expanding past the optimum.

### Cost vs heap-based A*

Each push/pop is `O(1)` (a `Vec` push/pop) plus one `HashMap` op, vs `O(log
N) + HashMap` for a heap. For the 15-puzzle, frontier sizes can hit millions,
so this is a meaningful constant-factor win.

### Threaded variant (commented out)

At the bottom of `bidirectional.rs`, included but wrapped in `#[cfg(any())]`
(never built; pure source-level reference) so the user can opt in by
uncommenting later. Sketch:

- Each side becomes a thread. Both sides own their own `Side<B>` (no
  contention on the per-side state).
- Cross-side meet detection needs the other side's `visited`. Wrap each
  side's `visited` in `Arc<RwLock<FxHashMap<...>>>` so the *other* thread
  can do contains-key reads while this thread writes. Reads vastly
  outnumber writes during expansion.
- `mu` and `meet` go into `Arc<Mutex<(u32, Option<B::Key>)>>` (or an
  `AtomicU64` for `mu` plus a separate mutex for `meet`).
- Each thread checks termination `min(f_self, f_other) >= mu` by reading
  `f_other` from a shared `AtomicU32` per side updated whenever bucket
  promotion happens.
- Once a thread detects termination it sets a shared `AtomicBool stop` flag;
  the other thread sees it on its next outer-loop check and returns.
- Reconstruction happens after both threads join: whichever thread captured
  the meet key reads from both `visited` maps.

Trade-off note in the comment block: lock contention on the `visited` maps
can erase the parallelism gain unless we shard or batch reads. The user
explicitly asked for this to start commented out.

## `main.rs`

Mirrors `/tmp/my8puzzle/src/Program.cpp` behavior plus a solver flag:

```
15puzzle-rs [--solver bucket|bidir] FILE [FILE ...]
```

Input file format (unchanged from C++): first integer `n`, then `n*n`
whitespace-separated tile values (0 = empty). For each file:

1. Parse → `Vec<u8>` of length `n*n`.
2. Print the board (matches `print_array` from `Models/Board.hpp:21`).
3. If `n == 4`: build `PackedBoard`, call solver. Else: build `DynamicBoard`,
   call solver. Done via a small generic helper `run<B: Board>(...)` so both
   arms share code.
4. Check `is_solvable()`; if not, print `"No solution possible"` and continue.
5. Time the solver call (`std::time::Instant`).
6. Print min moves, elapsed ms, and the move sequence (one board per line in
   the same `toString` format as `Models/Board.tpp:40-54`).

## Inversions / solvability — `src/inversions.rs`

Port of `Helpers/MergeSort.cpp`. A small `pub fn count(arr: &mut [u8]) -> u32`
returning the inversion count. SIZE-1 ≤ 24 in practice; merge sort is overkill
but matches the source. Used only by `Board::is_solvable`.

Solvability rules from `Board.tpp:99-115`:

- If `N*N` is odd → solvable iff inversions are even.
- If `N*N` is even → solvable iff `(inversions + empty_row) % 2 == 1`.

## What we drop from the C++ tree

- `Repositories/` and `Entities/` — DB persistence; user said skip.
- `Models/BoardSave.hpp` — only used by Repositories.
- `Services/TaskService.cpp` — entirely commented out.
- `structures/CustomList.cpp`, `structures/SmallerHashSet.cpp` — both
  commented-out C# scratch files.
- `structures/PreAllocatedStack` — its only role is a 4-element stack of
  neighbors, which becomes `arrayvec`-style stack-allocated state in Rust;
  for ≤4 elements we just return a `[Option<Neighbor<B>>; 4]` or use
  `smallvec::SmallVec<[_; 4]>`. To stay zero-deps, use
  `[Option<Neighbor<B>>; 4]` and a small custom iterator.

## Critical files (in source repo, all read-only references)

- `/tmp/my8puzzle/src/Program.cpp` — CLI behavior to mirror
- `/tmp/my8puzzle/src/Models/Board.hpp:51-120`, `Board.tpp` — Board logic
- `/tmp/my8puzzle/src/Models/Tiles.hpp:58-118` — nibble packing for N=4
- `/tmp/my8puzzle/src/Services/SolverService.cpp:20-71` — bucket-DFS to port
- `/tmp/my8puzzle/src/Helpers/MergeSort.cpp` — inversion count

## Files to create

All under `/home/user/15puzzle-rs/`:

- `Cargo.toml`
- `src/main.rs`
- `src/lib.rs`
- `src/board/mod.rs`
- `src/board/packed.rs`
- `src/board/dynamic.rs`
- `src/solver/mod.rs`
- `src/solver/bucket_dfs.rs`
- `src/solver/bidirectional.rs`
- `src/inversions.rs`
- `tests/puzzles.rs` — integration tests with a few canonical 8-puzzle and
  15-puzzle inputs (e.g. the classic 80-move 15-puzzle hard instance is
  out of reach for these solvers, but small instances of known optimal
  length validate correctness on both solvers).
- `samples/puzzle04_easy.txt`, `samples/puzzle04_medium.txt`,
  `samples/puzzle03.txt` — small input files for manual testing.

Branch: `claude/bidirectional-astar-puzzle-JCYp5` (already checked out).

## Verification

1. `cargo build --release` — clean build, no warnings.
2. `cargo test --release` — integration tests must pass. For each sample
   puzzle, both `bucket_dfs::solve` and `bidirectional::solve` must return
   the same `moves` count and a valid path (each step differs from the
   previous by one swap of `0` with an orthogonal neighbor).
3. Manual: `cargo run --release -- --solver bidir samples/puzzle03.txt`
   should match `cargo run --release -- --solver bucket samples/puzzle03.txt`
   on `moves`.
4. Cross-check at least one 15-puzzle input against the original C++ binary
   built from `/tmp/my8puzzle` to confirm the bucket-DFS port returns the
   same move count.
5. Sanity property: for the goal board as input, both solvers return
   `moves = 0` and `path = [goal]`.
