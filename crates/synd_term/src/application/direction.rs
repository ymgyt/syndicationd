#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub(crate) enum IndexOutOfRange {
    Wrapping,
    #[allow(dead_code)]
    Saturating,
}

impl Direction {
    #[allow(
        clippy::cast_sign_loss,
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap
    )]
    pub(crate) fn apply(self, index: usize, len: usize, out: IndexOutOfRange) -> usize {
        if len == 0 {
            return 0;
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

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::{prop_oneof, proptest, Just, ProptestConfig, Strategy};

    proptest! {
        #![proptest_config(ProptestConfig::default())]
        #[test]
        #[allow(clippy::cast_possible_wrap)]
        fn apply(
            dir in direction_strategy(),
            index in 0..10_usize,
            len in 0..10_usize,
            out in index_out_of_range_strategy())
        {
            let apply = dir.apply(index, len,out) as i64;
            let index = index as i64;
            let len = len as i64;
            assert!(
                (apply - index).abs() == 1 ||
                apply == 0 ||
                apply == len-1
            );
        }


    }
    fn direction_strategy() -> impl Strategy<Value = Direction> {
        prop_oneof![
            Just(Direction::Up),
            Just(Direction::Down),
            Just(Direction::Left),
            Just(Direction::Right),
        ]
    }

    fn index_out_of_range_strategy() -> impl Strategy<Value = IndexOutOfRange> {
        prop_oneof![
            Just(IndexOutOfRange::Wrapping),
            Just(IndexOutOfRange::Saturating)
        ]
    }
}
