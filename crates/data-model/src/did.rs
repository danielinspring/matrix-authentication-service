// Copyright 2025 New Vector Ltd.
//
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Element-Commercial
// Please see LICENSE files in the repository root for full details.

use chrono::{DateTime, Utc};
use serde::Serialize;
use ulid::Ulid;

/// A link between a user and a Decentralized Identifier (DID)
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UserDidLink {
    pub id: Ulid,
    pub user_id: Option<Ulid>,
    pub did: String,
    pub method: Option<String>,
    pub created_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
}
