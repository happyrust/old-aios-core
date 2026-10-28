// `test_duckdb` 已停用（2026-10-28）。
//
// 该文件 468 行**全部是注释**，没有任何可编译代码；它对应的 `duckdb` 正式依赖
// 也已在 `Cargo.toml` 中移除（原因见那里的注释：会拖进 libduckdb-sys 并在
// 构建时下载编译一份 DuckDB C 源码，无外网机器上直接失败）。
//
// 恢复时请一并把 `duckdb` 加进 `[dev-dependencies]`，再放开下面这行。
// pub mod test_duckdb;
pub mod test_sqlite;
// pub mod test_graph_query;
