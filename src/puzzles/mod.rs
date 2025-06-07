use std::sync::LazyLock;

use crate::bpz::Puzzle;

macro_rules! bpz {
    ($key:literal, $path:literal) => {
        ($key, include_str!($path))
    };
}

pub static PUZZLES: LazyLock<Vec<(&'static str, Puzzle)>> = LazyLock::new(|| {
    [
        bpz!("Example (Basic)", "./example-1.bpz"),
        bpz!("Example (World Tour)", "./example-2.bpz"),
        bpz!("Example (Drive-Thru)", "./example-3.bpz"),
        bpz!("Example (Black Ice)", "./example-4.bpz"),
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
