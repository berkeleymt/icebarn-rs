use crate::bpz::Dir;

pub fn rotate_from_north(dir: Dir) -> &'static str {
    match dir {
        Dir::North => "",
        Dir::South => "rotate-180",
        Dir::East => "rotate-90",
        Dir::West => "rotate-270",
    }
}
