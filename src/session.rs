use rand::seq::SliceRandom;

const ADJECTIVES: &[&str] = &[
    "brave", "calm", "eager", "fair", "gentle", "happy", "jolly", "kind", "lively", "merry",
    "noble", "polite", "proud", "quiet", "royal", "swift",
];
const NOUNS: &[&str] = &[
    "badger", "cat", "deer", "eagle", "fox", "goat", "hawk", "ibis", "jay", "koi", "lynx", "moth",
    "newt", "owl", "panda", "quail",
];

#[must_use]
pub fn random_name() -> String {
    let mut rng = rand::thread_rng();
    let a = ADJECTIVES.choose(&mut rng).unwrap();
    let n = NOUNS.choose(&mut rng).unwrap();
    format!("{a}-{n}")
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
            assert!(ADJECTIVES.contains(&a));
            assert!(NOUNS.contains(&b));
        }
    }

    #[test]
    fn compose_concatenates_with_dash() {
        assert_eq!(compose("twrk", "feat"), "twrk-feat");
    }
}
