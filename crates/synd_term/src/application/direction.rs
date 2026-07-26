#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    #[allow(
        clippy::cast_sign_loss,
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap
    )]
    pub(crate) fn apply(self, index: usize, len: usize) -> usize {
        if len == 0 {
            return 0;
        }
        let diff = match self {
            Direction::Up | Direction::Left => -1,
            Direction::Down | Direction::Right => 1,
        };

        let index = index as i64;
        if index + diff < 0 {
            len - 1
        } else if index + diff >= len as i64 {
            0
        } else {
            (index + diff) as usize
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::{Just, ProptestConfig, Strategy, prop_oneof, proptest};

    proptest! {
        #![proptest_config(ProptestConfig::default())]
        #[test]
        #[allow(clippy::cast_possible_wrap)]
        fn apply(
            dir in direction_strategy(),
            index in 0..10_usize,
            len in 0..10_usize)
        {
            let apply = dir.apply(index, len) as i64;
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
}
