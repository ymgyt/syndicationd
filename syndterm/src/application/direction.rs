#[derive(Debug, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    pub fn apply(&self, index: usize, len: usize, cycle: bool) -> usize {
        let diff = match self {
            Direction::Up => -1,
            Direction::Down => 1,
            Direction::Left => -1,
            Direction::Right => 1,
        };

        let index = index as i64;
        if index + diff < 0 {
            if cycle {
                len - 1
            } else {
                0
            }
        } else if index + diff >= len as i64 {
            if cycle {
                0
            } else {
                len - 1
            }
        } else {
            (index + diff) as usize
        }
    }
}
