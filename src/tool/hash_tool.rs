pub fn hash_str(input: &str) -> u64 {
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    std::hash::Hash::hash(input, &mut hash);
    std::hash::Hasher::finish(&hash)
}

pub fn hash_two_str(from: &str, to: &str) -> u64 {
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    std::hash::Hash::hash(from, &mut hash);
    std::hash::Hash::hash(to, &mut hash);
    std::hash::Hasher::finish(&hash)
}
