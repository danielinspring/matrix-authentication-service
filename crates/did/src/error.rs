// Copyright 2024, 2025 New Vector Ltd.
//
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Element-Commercial
// Please see LICENSE files in the repository root for full details.

use thiserror::Error;

/// Result type alias for DID operations
pub type Result<T> = std::result::Result<T, DIDError>;

/// Errors that can occur during DID operations
#[derive(Debug, Error)]
pub enum DIDError {
    /// Invalid DID format
    #[error("Invalid DID format: {0}")]
    InvalidDIDFormat(String),

    /// DID method not supported
    #[error("DID method not supported: {0}")]
    UnsupportedMethod(String),

    /// DID resolution failed
    #[error("DID resolution failed: {0}")]
    ResolutionFailed(String),

    /// Key generation failed
    #[error("Key generation failed: {0}")]
    KeyGenerationFailed(String),

    /// Key storage failed
    #[error("Key storage failed: {0}")]
    KeyStorageFailed(String),

    /// Key retrieval failed
    #[error("Key retrieval failed: {0}")]
    KeyRetrievalFailed(String),

    /// Invalid key type
    #[error("Invalid key type: {0}")]
    InvalidKeyType(String),

    /// Cryptographic operation failed
    #[error("Cryptographic operation failed: {0}")]
    CryptoError(String),

    /// DID document invalid
    #[error("DID document invalid: {0}")]
    InvalidDocument(String),

    /// Verification failed
    #[error("Verification failed: {0}")]
    VerificationFailed(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    /// Network error
    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    /// URL parse error
    #[error("URL parse error: {0}")]
    UrlParseError(#[from] url::ParseError),

    /// Generic error
    #[error("{0}")]
    Other(String),
}

impl From<String> for DIDError {
    fn from(s: String) -> Self {
        DIDError::Other(s)
    }
}

impl From<&str> for DIDError {
    fn from(s: &str) -> Self {
        DIDError::Other(s.to_string())
    }
}
