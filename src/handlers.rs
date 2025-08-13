use crate::db::DbPool;
use crate::models::{NewUser, UpdateUser, User};
use crate::schema::users;
use diesel::prelude::*;
use tracing::instrument;
use uuid::Uuid;

pub struct UserRepository;

impl UserRepository {
    #[instrument(skip(pool))]
    pub async fn create_user(pool: &DbPool, new_user: NewUser) -> Result<User, anyhow::Error> {
        let mut conn = crate::db::get_connection(pool)?;

        let user = tokio::task::spawn_blocking(move || {
            diesel::insert_into(users::table)
                .values(&new_user)
                .returning(User::as_returning())
                .get_result(&mut conn)
        })
        .await??;

        Ok(user)
    }

    #[instrument(skip(pool))]
    pub async fn get_user_by_id(pool: &DbPool, user_id: Uuid) -> Result<User, anyhow::Error> {
        let mut conn = crate::db::get_connection(pool)?;

        let user = tokio::task::spawn_blocking(move || {
            users::table
                .filter(users::id.eq(user_id))
                .select(User::as_select())
                .first(&mut conn)
        })
        .await??;

        Ok(user)
    }

    #[instrument(skip(pool))]
    pub async fn get_all_users(pool: &DbPool) -> Result<Vec<User>, anyhow::Error> {
        let mut conn = crate::db::get_connection(pool)?;

        let users_list = tokio::task::spawn_blocking(move || {
            users::table.select(User::as_select()).load(&mut conn)
        })
        .await??;

        Ok(users_list)
    }

    #[instrument(skip(pool))]
    pub async fn update_user(
        pool: &DbPool,
        user_id: Uuid,
        update_user: UpdateUser,
    ) -> Result<User, anyhow::Error> {
        let mut conn = crate::db::get_connection(pool)?;

        let user = tokio::task::spawn_blocking(move || {
            diesel::update(users::table.filter(users::id.eq(user_id)))
                .set(&update_user)
                .returning(User::as_returning())
                .get_result(&mut conn)
        })
        .await??;

        Ok(user)
    }

    #[instrument(skip(pool))]
    pub async fn delete_user(pool: &DbPool, user_id: Uuid) -> Result<bool, anyhow::Error> {
        let mut conn = crate::db::get_connection(pool)?;

        let deleted_count = tokio::task::spawn_blocking(move || {
            diesel::delete(users::table.filter(users::id.eq(user_id))).execute(&mut conn)
        })
        .await??;

        Ok(deleted_count > 0)
    }
}
