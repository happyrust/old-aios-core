use aios_core::init_surreal;
use aios_core::rs_surreal::operation::zone_update::update_missing_zone_refno;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize the SurrealDB connection
    init_surreal().await?;

    // Update the missing zone_refno fields
    let count = update_missing_zone_refno().await?;

    println!("Updated zone_refno for {} inst_relate records", count);

    Ok(())
}
