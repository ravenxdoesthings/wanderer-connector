use diesel::prelude::*;
use diesel::r2d2::{self, ConnectionManager};
use std::env;

pub type DbPool = r2d2::Pool<ConnectionManager<PgConnection>>;
pub type DbConnection = r2d2::PooledConnection<ConnectionManager<PgConnection>>;

pub fn establish_connection_pool() -> Result<DbPool, anyhow::Error> {
    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");

    let manager = ConnectionManager::<PgConnection>::new(database_url);
    let pool = r2d2::Pool::builder()
        .max_size(10)
        .build(manager)?;

    Ok(pool)
}

pub fn get_connection(pool: &DbPool) -> Result<DbConnection, anyhow::Error> {
    let conn = pool.get()?;
    Ok(conn)
}
