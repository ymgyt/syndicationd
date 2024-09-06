pub type Byte = byte_unit::Byte;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse() {
        assert_eq!(4_u64 * 1024 * 1024, Byte::parse_str("4MiB", true).unwrap());
        assert_eq!(4_u64 * 1000 * 1000, Byte::parse_str("4MB", true).unwrap());
    }
}
