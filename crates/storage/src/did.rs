// Copyright 2025 New Vector Ltd.
//
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Element-Commercial
// Please see LICENSE files in the repository root for full details.

//! Repository interfaces for Decentralized Identifier (DID) links

use async_trait::async_trait;
use mas_data_model::{Clock, User, UserDidLink};
use rand_core::RngCore;
use ulid::Ulid;

use crate::{Page, Pagination, repository_impl};

/// Filter parameters for listing user DID links
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct UserDidLinkFilter<'a> {
    user: Option<&'a User>,
    did: Option<&'a str>,
    method: Option<&'a str>,
}

impl<'a> UserDidLinkFilter<'a> {
    /// Create a new filter with default values
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Filter for a specific user
    #[must_use]
    pub fn for_user(mut self, user: &'a User) -> Self {
        self.user = Some(user);
        self
    }

    /// Filter for a specific DID string
    #[must_use]
    pub fn for_did(mut self, did: &'a str) -> Self {
        self.did = Some(did);
        self
    }

    /// Filter for a specific DID method
    #[must_use]
    pub fn for_method(mut self, method: &'a str) -> Self {
        self.method = Some(method);
        self
    }

    #[must_use]
    pub fn user(&self) -> Option<&User> { self.user }
    #[must_use]
    pub fn did(&self) -> Option<&str> { self.did }
    #[must_use]
    pub fn method(&self) -> Option<&str> { self.method }
}

/// Repository for managing links between users and DIDs
#[async_trait]
pub trait UserDidLinkRepository: Send + Sync {
    /// The error type returned by the repository
    type Error;

    /// Lookup a link by its ID
    async fn lookup(&mut self, id: Ulid) -> Result<Option<UserDidLink>, Self::Error>;

    /// Find a link by DID string
    async fn find_by_did(&mut self, did: &str) -> Result<Option<UserDidLink>, Self::Error>;

    /// Create a new DID link (unassociated to a user initially)
    async fn add(
        &mut self,
        rng: &mut (dyn RngCore + Send),
        clock: &dyn Clock,
        did: String,
        method: Option<String>,
    ) -> Result<UserDidLink, Self::Error>;

    /// Associate an existing DID link to a user
    async fn associate_to_user(
        &mut self,
        link: &UserDidLink,
        user: &User,
    ) -> Result<(), Self::Error>;

    /// List DID links
    async fn list(
        &mut self,
        filter: UserDidLinkFilter<'_>,
        pagination: Pagination,
    ) -> Result<Page<UserDidLink>, Self::Error>;

    /// Count DID links
    async fn count(&mut self, filter: UserDidLinkFilter<'_>) -> Result<usize, Self::Error>;

    /// Remove (revoke) a DID link
    async fn remove(&mut self, link: UserDidLink) -> Result<(), Self::Error>;
}

repository_impl!(UserDidLinkRepository:
    async fn lookup(&mut self, id: Ulid) -> Result<Option<UserDidLink>, Self::Error>;
    async fn find_by_did(&mut self, did: &str) -> Result<Option<UserDidLink>, Self::Error>;
    async fn add(
        &mut self,
        rng: &mut (dyn RngCore + Send),
        clock: &dyn Clock,
        did: String,
        method: Option<String>,
    ) -> Result<UserDidLink, Self::Error>;
    async fn associate_to_user(&mut self, link: &UserDidLink, user: &User) -> Result<(), Self::Error>;
    async fn list(&mut self, filter: UserDidLinkFilter<'_>, pagination: Pagination) -> Result<Page<UserDidLink>, Self::Error>;
    async fn count(&mut self, filter: UserDidLinkFilter<'_>) -> Result<usize, Self::Error>;
    async fn remove(&mut self, link: UserDidLink) -> Result<(), Self::Error>;
);
