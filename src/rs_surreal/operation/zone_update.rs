use crate::{RefnoEnum, SUL_DB};
use anyhow::Result;
use serde::Deserialize;

#[derive(Deserialize)]
struct CountResult {
    count: usize,
}

/// Update all records in the inst_relate table that don't have a zone_refno field.
/// For each record, call fn::find_ancestor_type to find the zone_refno,
/// and then assign that value to the record.
pub async fn update_missing_zone_refno() -> Result<usize> {
    // First, get count of records that need updating
    let count_sql = r#"
        SELECT count() FROM inst_relate 
        WHERE zone_refno = NONE AND in != NONE;
    "#;

    let mut count_response = SUL_DB.query(count_sql).await?;
    let count_result: Vec<CountResult> = count_response.take(0)?;
    let record_count = count_result.first().map(|r| r.count).unwrap_or(0);

    if record_count == 0 {
        return Ok(0);
    }

    // Then, update the records
    let update_sql = r#"
        LET $missing_zone_records = SELECT id, in FROM inst_relate 
                                    WHERE zone_refno = NONE 
                                    AND in != NONE;
        
        FOR $record IN $missing_zone_records {
            LET $zone = fn::find_ancestor_type($record.in, 'ZONE');
            IF $zone != NONE && $zone.refno != NONE {
                UPDATE $record.id SET zone_refno = $zone.refno;
            }
        }
    "#;

    SUL_DB.query(update_sql).await?;

    Ok(record_count)
}
