// Copyright 2025 New Vector Ltd.
//
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Element-Commercial
// Please see LICENSE files in the repository root for full details.

use std::time::Duration;

use axum::{Json, extract::State, http::StatusCode};
use chrono::{DateTime, Utc};
use mas_axum_utils::{InternalError, SessionInfoExt, cookies::CookieJar};
use mas_data_model::{BoxClock, BoxRng};
use mas_did::DidManager;
use mas_router::UrlBuilder;
use mas_storage::{BoxRepository, Pagination};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Deserialize)]
pub struct ChallengeRequest {
    pub did: String,
}

#[derive(Debug, Serialize)]
pub struct ChallengeResponse {
    pub did: String,
    pub message: String,
    pub expires_at: DateTime<Utc>,
}

#[tracing::instrument(name = "http.did.challenge", skip_all, fields(did = %body.did), err)]
pub async fn challenge(
    State(_url_builder): State<UrlBuilder>,
    mut repo: BoxRepository,
    mut rng: BoxRng,
    clock: BoxClock,
    cookie_jar: CookieJar,
    Json(body): Json<ChallengeRequest>,
) -> Result<(CookieJar, Json<ChallengeResponse>), InternalError> {
    // Require an active session
    let (session_info, cookie_jar) = cookie_jar.session_info();
    let Some(session_id) = session_info.current_session_id() else {
        return Err(InternalError::from_status(StatusCode::UNAUTHORIZED));
    };

    // Validate session exists
    let Some(_session) = repo.browser_session().lookup(session_id).await? else {
        return Err(InternalError::from_status(StatusCode::UNAUTHORIZED));
    };

    // Build a message to be signed (stateless, includes timestamp and random value)
    let issued_at = clock.now();
    let expires_at = issued_at + chrono::Duration::seconds(600);
    let mut random_bytes = [0u8; 32];
    rng.fill_bytes(&mut random_bytes);
    let nonce = base64ct::Base64UrlUnpadded::encode_string(&random_bytes);

    let message = format!(
        "Matrix Authentication Service DID Link\nDID: {}\nNonce: {}\nIssued-At: {}\nExpires-At: {}",
        body.did,
        nonce,
        issued_at.to_rfc3339(),
        expires_at.to_rfc3339()
    );

    Ok((cookie_jar, Json(ChallengeResponse { did: body.did, message, expires_at })))
}

#[derive(Debug, Deserialize)]
pub struct VerifyRequest {
    pub did: String,
    pub message: String,
    pub signature: String, // base64url
}

#[derive(Debug, Serialize)]
pub struct VerifyResponse {
    pub linked: bool,
}

#[tracing::instrument(name = "http.did.verify", skip_all, fields(did = %body.did), err)]
pub async fn verify(
    mut repo: BoxRepository,
    clock: BoxClock,
    cookie_jar: CookieJar,
    Json(body): Json<VerifyRequest>,
) -> Result<(CookieJar, Json<VerifyResponse>), InternalError> {
    let now = clock.now();

    // Require session
    let (session_info, cookie_jar) = cookie_jar.session_info();
    let Some(session_id) = session_info.current_session_id() else {
        return Err(InternalError::from_status(StatusCode::UNAUTHORIZED));
    };
    let Some(session) = repo.browser_session().lookup(session_id).await? else {
        return Err(InternalError::from_status(StatusCode::UNAUTHORIZED));
    };

    // Basic freshness check: ensure Expires-At is in the future
    if let Some(line) = body
        .message
        .lines()
        .find(|l| l.starts_with("Expires-At:"))
    {
        let ts = line.trim_start_matches("Expires-At:").trim();
        let Ok(exp) = DateTime::parse_from_rfc3339(ts) else {
            return Err(InternalError::from_status(StatusCode::BAD_REQUEST));
        };
        if now > exp.with_timezone(&Utc) {
            return Err(InternalError::from_status(StatusCode::UNAUTHORIZED));
        }
    }

    // Verify signature using DID document
    let signature = base64ct::Base64UrlUnpadded::decode_vec(&body.signature)
        .map_err(|_| InternalError::from_status(StatusCode::BAD_REQUEST))?;

    let ok = DidManager::verify_signature(&body.did, body.message.as_bytes(), &signature)
        .await
        .map_err(|e| InternalError::from_anyhow(e))?;

    if !ok {
        return Err(InternalError::from_status(StatusCode::UNAUTHORIZED));
    }

    // Ensure a link record exists; create if missing
    let mut did_repo = repo.user_did_link();
    if let Some(existing) = did_repo.find_by_did(&body.did).await? {
        if existing.user_id.is_none() {
            did_repo.associate_to_user(&existing, &session.user).await?;
        }
    } else {
        let link = did_repo
            .add(&mut rand::thread_rng(), &*clock, body.did.clone(), None)
            .await?;
        did_repo.associate_to_user(&link, &session.user).await?;
    }

    Ok((cookie_jar, Json(VerifyResponse { linked: true })))
}
