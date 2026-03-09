use anyhow::Result;
use sqlx_core::executor::Executor;
use sqlx_core::raw_sql::raw_sql;
use sqlx_postgres::PgPool;

pub async fn run_migrations(pool: &PgPool) -> Result<()> {
    let sql = include_str!("../../migrations/001_initial.sql");
    pool.execute(raw_sql(sql)).await?;
    tracing::info!("Cloud database migrations applied");
    Ok(())
}
