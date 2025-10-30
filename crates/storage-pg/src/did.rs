// Copyright 2025 New Vector Ltd.
//
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Element-Commercial
// Please see LICENSE files in the repository root for full details.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use mas_data_model::{Clock, User, UserDidLink};
use mas_storage::{Page, Pagination, pagination::Node, did::{UserDidLinkFilter, UserDidLinkRepository}};
use rand::RngCore;
use sea_query::{Expr, PostgresQueryBuilder, Query, enum_def};
use sea_query_binder::SqlxBinder;
use sqlx::PgConnection;
use ulid::Ulid;
use uuid::Uuid;

use crate::{DatabaseError, filter::{Filter, StatementExt}, iden::UserDids, pagination::QueryBuilderExt, tracing::ExecuteExt};

/// An implementation of [`UserDidLinkRepository`] for a PostgreSQL connection
pub struct PgUserDidLinkRepository<'c> {
    conn: &'c mut PgConnection,
}

impl<'c> PgUserDidLinkRepository<'c> {
    /// Create a new [`PgUserDidLinkRepository`] from an active PostgreSQL connection
    pub fn new(conn: &'c mut PgConnection) -> Self {
        Self { conn }
    }
}

#[derive(sqlx::FromRow)]
#[enum_def]
struct DidLinkLookup {
    user_did_id: Uuid,
    user_id: Option<Uuid>,
    did: String,
    method: Option<String>,
    created_at: DateTime<Utc>,
    revoked_at: Option<DateTime<Utc>>,
}

impl Node<Ulid> for DidLinkLookup {
    fn cursor(&self) -> Ulid {
        self.user_did_id.into()
    }
}

impl From<DidLinkLookup> for UserDidLink {
    fn from(value: DidLinkLookup) -> Self {
        UserDidLink {
            id: value.user_did_id.into(),
            user_id: value.user_id.map(Into::into),
            did: value.did,
            method: value.method,
            created_at: value.created_at,
            revoked_at: value.revoked_at,
        }
    }
}

impl Filter for UserDidLinkFilter<'_> {
    fn generate_condition(&self, _has_joins: bool) -> impl sea_query::IntoCondition {
        sea_query::Condition::all()
            .add_option(self.user().map(|user| {
                Expr::col((UserDids::Table, UserDids::UserId)).eq(Uuid::from(user.id))
            }))
            .add_option(self.did().map(|did| {
                Expr::col((UserDids::Table, UserDids::Did)).eq(did)
            }))
            .add_option(self.method().map(|method| {
                Expr::col((UserDids::Table, UserDids::Method)).eq(method)
            }))
    }
}

#[async_trait]
impl UserDidLinkRepository for PgUserDidLinkRepository<'_> {
    type Error = DatabaseError;

    #[tracing::instrument(
        name = "db.user_did_link.lookup",
        skip_all,
        fields(
            db.query.text,
            user_did_link.id = %id,
        ),
        err,
    )]
    async fn lookup(&mut self, id: Ulid) -> Result<Option<UserDidLink>, Self::Error> {
        let res = sqlx::query_as!(
            DidLinkLookup,
            r#"
                SELECT user_did_id, user_id, did, method, created_at, revoked_at
                FROM user_dids
                WHERE user_did_id = $1
            "#,
            Uuid::from(id),
        )
        .traced()
        .fetch_optional(&mut *self.conn)
        .await?
        .map(Into::into);

        Ok(res)
    }

    #[tracing::instrument(
        name = "db.user_did_link.find_by_did",
        skip_all,
        fields(
            db.query.text,
            user_did_link.did = did,
        ),
        err,
    )]
    async fn find_by_did(&mut self, did: &str) -> Result<Option<UserDidLink>, Self::Error> {
        let res = sqlx::query_as!(
            DidLinkLookup,
            r#"
                SELECT user_did_id, user_id, did, method, created_at, revoked_at
                FROM user_dids
                WHERE did = $1
            "#,
            did,
        )
        .traced()
        .fetch_optional(&mut *self.conn)
        .await?
        .map(Into::into);

        Ok(res)
    }

    #[tracing::instrument(
        name = "db.user_did_link.add",
        skip_all,
        fields(
            db.query.text,
            user_did_link.id,
            user_did_link.did = did,
            user_did_link.method = method,
        ),
        err,
    )]
    async fn add(
        &mut self,
        rng: &mut (dyn RngCore + Send),
        clock: &dyn Clock,
        did: String,
        method: Option<String>,
    ) -> Result<UserDidLink, Self::Error> {
        let created_at = clock.now();
        let id = Ulid::from_datetime_with_source(created_at.into(), rng);
        tracing::Span::current().record("user_did_link.id", tracing::field::display(id));

        sqlx::query!(
            r#"
                INSERT INTO user_dids (user_did_id, user_id, did, method, created_at, revoked_at)
                VALUES ($1, NULL, $2, $3, $4, NULL)
            "#,
            Uuid::from(id),
            &did,
            method.as_deref(),
            created_at,
        )
        .traced()
        .execute(&mut *self.conn)
        .await?;

        Ok(UserDidLink {
            id,
            user_id: None,
            did,
            method,
            created_at,
            revoked_at: None,
        })
    }

    #[tracing::instrument(
        name = "db.user_did_link.associate_to_user",
        skip_all,
        fields(
            db.query.text,
            %link.id,
            %user.id,
            %user.username,
        ),
        err,
    )]
    async fn associate_to_user(
        &mut self,
        link: &UserDidLink,
        user: &User,
    ) -> Result<(), Self::Error> {
        sqlx::query!(
            r#"
                UPDATE user_dids
                SET user_id = $1
                WHERE user_did_id = $2
            "#,
            Uuid::from(user.id),
            Uuid::from(link.id),
        )
        .traced()
        .execute(&mut *self.conn)
        .await?;

        Ok(())
    }

    #[tracing::instrument(
        name = "db.user_did_link.list",
        skip_all,
        fields(
            db.query.text,
        ),
        err,
    )]
    async fn list(
        &mut self,
        filter: UserDidLinkFilter<'_>,
        pagination: Pagination,
    ) -> Result<Page<UserDidLink>, Self::Error> {
        let (sql, arguments) = Query::select()
            .expr_as(Expr::col((UserDids::Table, UserDids::UserDidId)), DidLinkLookupIden::UserDidId)
            .expr_as(Expr::col((UserDids::Table, UserDids::UserId)), DidLinkLookupIden::UserId)
            .expr_as(Expr::col((UserDids::Table, UserDids::Did)), DidLinkLookupIden::Did)
            .expr_as(Expr::col((UserDids::Table, UserDids::Method)), DidLinkLookupIden::Method)
            .expr_as(Expr::col((UserDids::Table, UserDids::CreatedAt)), DidLinkLookupIden::CreatedAt)
            .expr_as(Expr::col((UserDids::Table, UserDids::RevokedAt)), DidLinkLookupIden::RevokedAt)
            .from(UserDids::Table)
            .apply_filter(filter)
            .generate_pagination((UserDids::Table, UserDids::UserDidId), pagination)
            .build_sqlx(PostgresQueryBuilder);

        let edges: Vec<DidLinkLookup> = sqlx::query_as_with(&sql, arguments)
            .traced()
            .fetch_all(&mut *self.conn)
            .await?;

        let page = pagination.process(edges).map(UserDidLink::from);

        Ok(page)
    }

    #[tracing::instrument(
        name = "db.user_did_link.count",
        skip_all,
        fields(
            db.query.text,
        ),
        err,
    )]
    async fn count(&mut self, filter: UserDidLinkFilter<'_>) -> Result<usize, Self::Error> {
        let (sql, arguments) = Query::select()
            .expr(Expr::col((UserDids::Table, UserDids::UserDidId)).count())
            .from(UserDids::Table)
            .apply_filter(filter)
            .build_sqlx(PostgresQueryBuilder);

        let count: i64 = sqlx::query_scalar_with(&sql, arguments)
            .traced()
            .fetch_one(&mut *self.conn)
            .await?;

        count.try_into().map_err(DatabaseError::to_invalid_operation)
    }

    #[tracing::instrument(
        name = "db.user_did_link.remove",
        skip_all,
        fields(
            db.query.text,
            user_did_link.id,
            user_did_link.did = %link.did,
        ),
        err,
    )]
    async fn remove(&mut self, link: UserDidLink) -> Result<(), Self::Error> {
        let res = sqlx::query!(
            r#"
                DELETE FROM user_dids
                WHERE user_did_id = $1
            "#,
            Uuid::from(link.id),
        )
        .traced()
        .execute(&mut *self.conn)
        .await?;

        DatabaseError::ensure_affected_rows(&res, 1)?;
        Ok(())
    }
}
