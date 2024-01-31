#[derive(Debug, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(PartialEq, Eq)]
pub enum IndexOutOfRange {
    Wrapping,
    Saturating,
}

impl Direction {
    pub fn apply(&self, index: usize, len: usize, out: IndexOutOfRange) -> usize {
        let diff = match self {
            Direction::Up => -1,
            Direction::Down => 1,
            Direction::Left => -1,
            Direction::Right => 1,
        };

        let index = index as i64;
        if index + diff < 0 {
            match out {
                IndexOutOfRange::Wrapping => len - 1,
                IndexOutOfRange::Saturating => 0,
            }
        } else if index + diff >= len as i64 {
            match out {
                IndexOutOfRange::Wrapping => 0,
                IndexOutOfRange::Saturating => len - 1,
            }
        } else {
            (index + diff) as usize
        }
    }
}
