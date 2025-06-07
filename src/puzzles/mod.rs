use std::sync::LazyLock;

use crate::bpz::Puzzle;

macro_rules! bpz {
    ($key:literal, $path:literal) => {
        ($key, include_str!($path))
    };
}

pub static PUZZLES: LazyLock<Vec<(&'static str, Puzzle)>> = LazyLock::new(|| {
    [
        bpz!("Basic 1", "./basic-1.bpz"),
        bpz!("Basic 2", "./basic-2.bpz"),
        bpz!("Basic 3", "./basic-3.bpz"),
        bpz!("World Tour 1", "./world-tour-1.bpz"),
        bpz!("World Tour 2", "./world-tour-2.bpz"),
        bpz!("World Tour 3", "./world-tour-3.bpz"),
        bpz!("Drive-Thru 1", "./drive-thru-1.bpz"),
        bpz!("Drive-Thru 2", "./drive-thru-2.bpz"),
        bpz!("Drive-Thru 3", "./drive-thru-3.bpz"),
        bpz!("Black Ice 1", "./black-ice-1.bpz"),
        bpz!("Black Ice 2", "./black-ice-2.bpz"),
        bpz!("Black Ice 3", "./black-ice-3.bpz"),
        bpz!("Challenge 1 (Basic)", "./challenge-1.bpz"),
        bpz!("Challenge 2 (World Tour)", "./challenge-2.bpz"),
        bpz!("Challenge 3 (Drive-Thru)", "./challenge-3.bpz"),
        bpz!("Challenge 4 (Black Ice)", "./challenge-4.bpz"),
    ]
    .into_iter()
    .map(|(name, src)| {
        (
            name,
            src.parse().expect(&format!("Failed to parse {}", name)),
        )
    })
    .collect()
});
