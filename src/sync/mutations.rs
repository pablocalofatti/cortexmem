use anyhow::Result;

use crate::db::Database;

/// Records a mutation for sync replication.
/// Called after every write operation. Callers handle errors via fire-and-forget logging.
pub(crate) fn capture_mutation(
    db: &Database,
    entity: &str,
    entity_key: &str,
    op: &str,
    payload: &str,
    project: &str,
) -> Result<i64> {
    db.insert_sync_mutation(entity, entity_key, op, payload, project)
}
