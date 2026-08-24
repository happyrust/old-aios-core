use crate::{RefU64, rs_surreal::SUL_DB};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use surrealdb::sql::Thing;

/// Represents a previous connection of a pipe element
#[derive(Default, Debug, Serialize, Deserialize)]
pub struct PrevConnection {
    pub id: RefU64,
    pub prev_full_name: String,
    pub prev: Option<RefU64>,
    pub tubi_name: Option<String>,
    pub tubi_refno: Option<RefU64>,
    pub tubi_spref: Option<String>,
}

/// Represents a next connection of a pipe element
#[derive(Default, Debug, Serialize, Deserialize)]
pub struct NextConnection {
    pub id: RefU64,
    pub next_full_name: String,
    pub next: Option<RefU64>,
    pub tubi_name: Option<String>,
    pub tubi_refno: Option<RefU64>,
    pub tubi_spref: Option<String>,
}

/// Get the previous connected pipe element
///
/// Wraps the Surreal function fn::prev_connect_pe
pub async fn get_prev_connect_pe(pe_id: RefU64) -> Result<Option<RefU64>> {
    let sql = format!("RETURN fn::prev_connect_pe({})", pe_id.to_pe_key());
    let result: Option<RefU64> = SUL_DB.query(sql).await?.take(0)?;

    Ok(result)
}

/// Get the next connected pipe element
///
/// Wraps the Surreal function fn::next_connect_pe
pub async fn get_next_connect_pe(pe_id: RefU64) -> Result<Option<RefU64>> {
    let sql = format!("RETURN fn::next_connect_pe({})", pe_id.to_pe_key());
    let result: Option<RefU64> = SUL_DB.query(sql).await?.take(0)?;

    Ok(result)
}

/// Check if a pipe element has a leaving tubi
///
/// Wraps the Surreal function fn::has_leave_tubi
pub async fn has_leave_tubi(pe_id: RefU64) -> Result<bool> {
    let sql = format!("RETURN fn::has_leave_tubi({})", pe_id.to_pe_key());
    let result: Option<bool> = SUL_DB.query(sql).await?.take(0)?;

    Ok(result.unwrap_or(false))
}

/// Check if a pipe element has an arriving tubi
///
/// Wraps the Surreal function fn::has_arrive_tubi
pub async fn has_arrive_tubi(pe_id: RefU64) -> Result<bool> {
    let sql = format!("RETURN fn::has_arrive_tubi({})", pe_id.to_pe_key());
    let result: Option<bool> = SUL_DB.query(sql).await?.take(0)?;

    Ok(result.unwrap_or(false))
}

/// Get the previous connection of a pipe element
///
/// Wraps the Surreal function fn::prev_connect_pe_data
pub async fn get_prev_connection(pe_id: RefU64) -> Result<PrevConnection> {
    let sql = format!("RETURN fn::prev_connect_pe_data({})", pe_id.to_pe_key());
    let mut result: Option<PrevConnection> = SUL_DB.query(sql).await?.take(0)?;

    Ok(result.unwrap_or(PrevConnection::default()))
}

/// Get the next connection of a pipe element
///
/// Wraps the Surreal function fn::next_connect_pe_data
pub async fn get_next_connection(pe_id: RefU64) -> Result<NextConnection> {
    let sql = format!("RETURN fn::next_connect_pe_data({})", pe_id.to_pe_key());
    let mut result: Option<NextConnection> = SUL_DB.query(sql).await?.take(0)?;

    Ok(result.unwrap_or(NextConnection::default()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{RefnoEnum, init_test_surreal};

    #[tokio::test]
    async fn test_get_connections() -> Result<()> {
        // Initialize the SurrealDB connection
        init_test_surreal().await;

        // Use a specific refno for testing
        let pe_id: RefU64 = "24383_76192".into();

        // Get pipe connections
        let prev_connection = get_prev_connection(pe_id).await?;
        let next_connection = get_next_connection(pe_id).await?;

        dbg!(&prev_connection);
        dbg!(&next_connection);

        // Print the connections for debugging
        println!("Pipe connections for {:?}:", pe_id);
        println!("  Previous: {:?}", prev_connection.prev);
        println!("  Next: {:?}", next_connection.next);

        // Assert that we got valid connection structures
        assert_eq!(prev_connection.id, pe_id);
        assert_eq!(next_connection.id, pe_id);

        // Note: The actual prev/next values depend on your database content
        // You might want to add specific assertions for your test data

        Ok(())
    }
}
