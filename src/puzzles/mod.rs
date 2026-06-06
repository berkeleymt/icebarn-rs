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
        bpz!("Basic 4", "./basic-4.bpz"),
        bpz!("Basic 5", "./basic-5.bpz"),
        bpz!("Paint 1", "./paint-1.bpz"),
        bpz!("Paint 2", "./paint-2.bpz"),
        bpz!("Paint 3", "./paint-3.bpz"),
        bpz!("Spiral 1", "./spiral-1.bpz"),
        bpz!("Spiral 2", "./spiral-2.bpz"),
        bpz!("Spiral 3", "./spiral-3.bpz"),
        bpz!("Binario 1", "./binario-1.bpz"),
        bpz!("Binario 2", "./binario-2.bpz"),
        bpz!("Binario 3", "./binario-3.bpz"),
        bpz!("Challenge 1 (Basic)", "./challenge-1.bpz"),
        bpz!("Challenge 2 (Paint)", "./challenge-2.bpz"),
        bpz!("Challenge 3 (Spiral)", "./challenge-3.bpz"),
        bpz!("Challenge 4 (Binario)", "./challenge-4.bpz"),
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

