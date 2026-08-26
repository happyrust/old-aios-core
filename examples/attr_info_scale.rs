//! Reports the size of the table `get_default_pdms_db_info()` actually loaded.
//!
//! Built with `--features load_file` the table comes from the working directory,
//! so running this binary from two directories holding different
//! `all_attr_info.json` files is what distinguishes a live read from the copy
//! `include_str!` would have baked in.

fn main() {
    let info = aios_core::get_default_pdms_db_info();
    let nouns = info.noun_attr_info_map.len();
    let pairs: usize = info
        .noun_attr_info_map
        .iter()
        .map(|noun| noun.value().len())
        .sum();
    println!(
        "cwd={:?} nouns={nouns} pairs={pairs}",
        std::env::current_dir().unwrap_or_default()
    );
}
