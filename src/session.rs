#[must_use]
pub fn random_name() -> String {
    petname::petname(2, "-").expect("petname wordlist is non-empty")
}

#[must_use]
pub fn compose(folder: &str, name: &str) -> String {
    format!("{folder}-{name}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_name_has_expected_shape() {
        for _ in 0..16 {
            let n = random_name();
            let (a, b) = n.split_once('-').expect("contains dash");
            assert!(!a.is_empty());
            assert!(!b.is_empty());
            assert!(!b.contains('-'));
        }
    }

    #[test]
    fn compose_concatenates_with_dash() {
        assert_eq!(compose("twrk", "feat"), "twrk-feat");
    }
}
