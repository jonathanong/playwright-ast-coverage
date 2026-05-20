pub fn value() -> usize {
    1
}

#[cfg(test)]
mod tests {
    use super::value;

    #[test]
    fn value_returns_one() {
        assert_eq!(value(), 1);
    }
}
