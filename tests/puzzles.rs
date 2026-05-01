use puzzle15::board::{Board, DynamicBoard, PackedBoard};
use puzzle15::solver::{bidirectional, bucket_dfs, Solution};

fn validate_path<B: Board>(sol: &Solution<B>, n: usize) {
    assert!(!sol.path.is_empty());
    assert_eq!(sol.moves as usize, sol.path.len() - 1);
    // Last board must be goal
    assert!(sol.path.last().unwrap().is_goal());
    // Each consecutive pair differs by exactly one blank swap
    for w in sol.path.windows(2) {
        let a = &w[0];
        let b = &w[1];
        // Find positions of blank in both
        let ea = a.empty_index();
        let eb = b.empty_index();
        assert_ne!(ea, eb, "blank didn't move");
        let total = n * n;
        let mut diffs = 0;
        for i in 0..total {
            if a.tile_at(i) != b.tile_at(i) { diffs += 1; }
        }
        assert_eq!(diffs, 2, "exactly 2 tiles must differ per move");
    }
}

fn both_solvers_agree<B: Board + std::fmt::Debug>(initial: B, expected_moves: Option<u32>) {
    let n = initial.n();
    let r1 = bucket_dfs::solve(initial.clone());
    let r2 = bidirectional::solve(initial.clone());
    match (r1, r2) {
        (None, None) => {
            assert!(expected_moves.is_none(), "expected a solution");
        }
        (Some(s1), Some(s2)) => {
            if let Some(exp) = expected_moves {
                assert_eq!(s1.moves, exp, "bucket_dfs move count mismatch");
                assert_eq!(s2.moves, exp, "bidirectional move count mismatch");
            }
            assert_eq!(s1.moves, s2.moves, "solvers disagree on move count");
            validate_path(&s1, n);
            validate_path(&s2, n);
        }
        _ => panic!("one solver found solution, other didn't"),
    }
}

// --- 3×3 (8-puzzle) tests ---

#[test]
fn test_8puzzle_goal() {
    let tiles = vec![1,2,3,4,5,6,7,8,0];
    let board = DynamicBoard::from_tiles(3, &tiles);
    both_solvers_agree(board, Some(0));
}

#[test]
fn test_8puzzle_one_move() {
    // Blank at index 8, swap with tile at index 7 (value 8) → 1 move
    let tiles = vec![1,2,3,4,5,6,7,0,8];
    let board = DynamicBoard::from_tiles(3, &tiles);
    both_solvers_agree(board, Some(1));
}

#[test]
fn test_8puzzle_easy() {
    // 1 2 3 / 4 5 6 / 7 _ 8  → 1 move
    let tiles = vec![1,2,3,4,5,6,7,0,8];
    let board = DynamicBoard::from_tiles(3, &tiles);
    both_solvers_agree(board, Some(1));
}

#[test]
fn test_8puzzle_medium() {
    // Classic small test: known 4-move solution
    // 1 2 3 / 4 0 5 / 7 8 6
    let tiles = vec![1,2,3,4,0,5,7,8,6];
    let board = DynamicBoard::from_tiles(3, &tiles);
    both_solvers_agree(board, None); // just check both agree and path valid
}

#[test]
fn test_8puzzle_unsolvable() {
    // Swap tiles 1 and 2 to create unsolvable instance
    let tiles = vec![2,1,3,4,5,6,7,8,0];
    let board = DynamicBoard::from_tiles(3, &tiles);
    assert!(!board.is_solvable());
    assert!(bucket_dfs::solve(board.clone()).is_none());
    assert!(bidirectional::solve(board).is_none());
}

// --- 4×4 (15-puzzle) tests with PackedBoard ---

#[test]
fn test_15puzzle_goal_packed() {
    let tiles: Vec<u8> = (1..=15).chain(std::iter::once(0)).collect();
    let board = PackedBoard::from_tiles(4, &tiles);
    both_solvers_agree(board, Some(0));
}

#[test]
fn test_15puzzle_one_move_packed() {
    // Blank at 15, swap with tile at 14 (value 15) → 1 move
    let tiles: Vec<u8> = (1..=14).chain([0u8, 15u8]).collect();
    let board = PackedBoard::from_tiles(4, &tiles);
    both_solvers_agree(board, Some(1));
}

#[test]
fn test_15puzzle_easy_packed() {
    // Blank at index 13, needs 2 moves to reach index 15
    let tiles: Vec<u8> = (1..=13).chain([0u8, 14u8, 15u8]).collect();
    let board = PackedBoard::from_tiles(4, &tiles);
    both_solvers_agree(board, Some(2));
}

#[test]
fn test_15puzzle_medium_packed() {
    // 5 1 3 4 / 2 6 7 8 / 9 10 11 12 / 13 14 15 0  (a few moves)
    let tiles = vec![5,1,3,4,2,6,7,8,9,10,11,12,13,14,15,0];
    let board = PackedBoard::from_tiles(4, &tiles);
    both_solvers_agree(board, None);
}

#[test]
fn test_15puzzle_unsolvable_packed() {
    // Swap 14 and 15 in goal to create unsolvable
    let tiles: Vec<u8> = (1..=13).chain([15u8, 14u8, 0u8]).collect();
    let board = PackedBoard::from_tiles(4, &tiles);
    assert!(!board.is_solvable());
    assert!(bucket_dfs::solve(board.clone()).is_none());
    assert!(bidirectional::solve(board).is_none());
}

// --- Cross-board consistency: DynamicBoard at N=4 vs PackedBoard ---

#[test]
fn test_dynamic_vs_packed_consistency() {
    let tiles = vec![1u8,2,3,4,5,6,7,8,9,10,11,12,13,0,14,15];
    let packed = PackedBoard::from_tiles(4, &tiles);
    let dynamic = DynamicBoard::from_tiles(4, &tiles);
    let s1 = bucket_dfs::solve(packed.clone()).unwrap();
    let s2 = bucket_dfs::solve(dynamic.clone()).unwrap();
    assert_eq!(s1.moves, s2.moves);
    let s3 = bidirectional::solve(packed).unwrap();
    let s4 = bidirectional::solve(dynamic).unwrap();
    assert_eq!(s3.moves, s4.moves);
    assert_eq!(s1.moves, s3.moves);
}
