// Copyright 2024, 2025 New Vector Ltd.
//
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Element-Commercial
// Please see LICENSE files in the repository root for full details.

//! DID (Decentralized Identifier) and Verifiable Credentials support for Matrix
//!
//! This crate provides comprehensive DID functionality including:
//! - DID creation and resolution (did:peer, did:web, did:key)
//! - Key management with platform-native secure storage
//! - Verifiable Credentials support
//! - Integration with Matrix's E2EE infrastructure

mod did_document;
mod did_manager;
mod error;
mod key_manager;
mod resolver;
mod types;
mod verifiable_credential;

pub use did_document::{DIDDocument, Service, VerificationMethod, VerificationRelationship};
pub use did_manager::DIDManager;
pub use error::{DIDError, Result};
pub use key_manager::{KeyManager, KeyPair, KeyType};
pub use resolver::{DIDResolver, ResolverRegistry};
pub use types::{DIDUrl, DID};
pub use verifiable_credential::{
    Credential, CredentialStatus, CredentialSubject, Issuer, Proof, VerifiableCredential,
};
