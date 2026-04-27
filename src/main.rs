use std::env;
use std::fs;
use std::time::Instant;

use puzzle15::board::{Board, DynamicBoard, PackedBoard};
use puzzle15::solver::Solution;

fn print_board<B: Board>(board: &B) {
    let n = board.n();
    for r in 0..n {
        let row: Vec<String> = (0..n)
            .map(|c| {
                let t = board.tile_at(r * n + c);
                if t == 0 { "  0".to_string() } else { format!("{:3}", t) }
            })
            .collect();
        println!("{}", row.join(" "));
    }
}

fn print_solution<B: Board>(sol: &Solution<B>) {
    println!("Min moves: {}", sol.moves);
    for board in &sol.path {
        println!("---");
        print_board(board);
    }
}

fn run<B: Board>(tiles: Vec<u8>, n: usize, solver: &str) {
    let board = B::from_tiles(n, &tiles);

    println!("Initial board:");
    print_board(&board);

    if !board.is_solvable() {
        println!("No solution possible");
        return;
    }

    let start = Instant::now();
    let result = if solver == "bucket" {
        puzzle15::solver::bucket_dfs::solve(board)
    } else {
        puzzle15::solver::bidirectional::solve(board)
    };
    let elapsed = start.elapsed();

    match result {
        Some(sol) => {
            println!("Solved in {}ms", elapsed.as_millis());
            print_solution(&sol);
        }
        None => println!("No solution found"),
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut solver = "bidir";
    let mut files: Vec<&str> = Vec::new();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--solver" => {
                i += 1;
                if i < args.len() {
                    solver = match args[i].as_str() {
                        "bucket" => "bucket",
                        _ => "bidir",
                    };
                }
            }
            arg => files.push(arg),
        }
        i += 1;
    }

    if files.is_empty() {
        eprintln!("Usage: 15puzzle-rs [--solver bucket|bidir] FILE [FILE ...]");
        std::process::exit(1);
    }

    for file in files {
        println!("=== {} ===", file);
        let content = match fs::read_to_string(file) {
            Ok(c) => c,
            Err(e) => { eprintln!("Error reading {}: {}", file, e); continue; }
        };
        let nums: Vec<u8> = content
            .split_whitespace()
            .filter_map(|s| s.parse().ok())
            .collect();
        if nums.is_empty() {
            eprintln!("Empty or invalid file: {}", file);
            continue;
        }
        let n = nums[0] as usize;
        let tiles = nums[1..].to_vec();
        if tiles.len() != n * n {
            eprintln!("Expected {}×{}={} tiles, got {}", n, n, n * n, tiles.len());
            continue;
        }

        if n == 4 {
            run::<PackedBoard>(tiles, n, solver);
        } else {
            run::<DynamicBoard>(tiles, n, solver);
        }
    }
}
