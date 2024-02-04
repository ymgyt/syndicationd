#[derive(Debug, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum IndexOutOfRange {
    Wrapping,
    Saturating,
}

impl Direction {
    #[allow(
        clippy::cast_sign_loss,
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap
    )]
    pub fn apply(&self, index: usize, len: usize, out: IndexOutOfRange) -> usize {
        if len == 0 {
            return index;
        }
        let diff = match self {
            Direction::Up | Direction::Left => -1,
            Direction::Down | Direction::Right => 1,
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
