/// 替换关键词
#[inline]
pub fn qualified_table_name(table: &str) -> String {
    table.replace("JOIN", "JOIN_").replace("LOOP", "LOOP_")
}

/// 还原关键词
#[inline]
pub fn restore_type_name(table: &str) -> &str {
    if table.ends_with("_") {
        &table[..table.len() - 1]
    } else {
        table
    }
}

///替换关键词
#[inline]
pub fn qualified_column_name(column: &str) -> String {
    column
        .replace("DESC", "DESC_")
        .replace("LOCK", "LOCK_")
        .replace("CHAR", "CHAR_")
}
