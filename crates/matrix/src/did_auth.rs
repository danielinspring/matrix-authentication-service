// Copyright 2024, 2025 New Vector Ltd.
//
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Element-Commercial
// Please see LICENSE files in the repository root for full details.

//! DID-based authentication for Matrix
//!
//! This module provides DID-based authentication mechanisms that can be used
//! alongside traditional Matrix authentication. It enables:
//! - Mapping DIDs to Matrix user IDs
//! - Authenticating users with DID challenge-response
//! - Managing device keys as DID verification methods

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Context;
use tokio::sync::RwLock;

/// DID to Matrix ID mapping
pub struct DIDMapping {
    /// Maps DID strings to Matrix localparts
    did_to_localpart: Arc<RwLock<HashMap<String, String>>>,
    /// Maps Matrix localparts to DIDs
    localpart_to_did: Arc<RwLock<HashMap<String, String>>>,
}

impl DIDMapping {
    /// Create a new DID mapping
    pub fn new() -> Self {
        DIDMapping {
            did_to_localpart: Arc::new(RwLock::new(HashMap::new())),
            localpart_to_did: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Associate a DID with a Matrix localpart
    pub async fn associate(&self, did: String, localpart: String) -> anyhow::Result<()> {
        let mut did_map = self.did_to_localpart.write().await;
        let mut localpart_map = self.localpart_to_did.write().await;
        
        did_map.insert(did.clone(), localpart.clone());
        localpart_map.insert(localpart, did);
        
        Ok(())
    }
    
    /// Get the Matrix localpart for a DID
    pub async fn get_localpart(&self, did: &str) -> Option<String> {
        let map = self.did_to_localpart.read().await;
        map.get(did).cloned()
    }
    
    /// Get the DID for a Matrix localpart
    pub async fn get_did(&self, localpart: &str) -> Option<String> {
        let map = self.localpart_to_did.read().await;
        map.get(localpart).cloned()
    }
    
    /// Remove a DID mapping
    pub async fn remove(&self, did: &str) -> anyhow::Result<()> {
        let mut did_map = self.did_to_localpart.write().await;
        let mut localpart_map = self.localpart_to_did.write().await;
        
        if let Some(localpart) = did_map.remove(did) {
            localpart_map.remove(&localpart);
        }
        
        Ok(())
    }
}

impl Default for DIDMapping {
    fn default() -> Self {
        Self::new()
    }
}

/// DID-based authentication challenge
#[derive(Debug, Clone)]
pub struct AuthChallenge {
    /// The challenge nonce
    pub nonce: String,
    
    /// When the challenge was created
    pub created_at: chrono::DateTime<chrono::Utc>,
    
    /// When the challenge expires
    pub expires_at: chrono::DateTime<chrono::Utc>,
    
    /// The DID being authenticated
    pub did: String,
}

impl AuthChallenge {
    /// Create a new authentication challenge
    pub fn new(did: String, validity_seconds: i64) -> Self {
        use rand::Rng;
        let nonce = rand::thread_rng()
            .sample_iter(&rand::distributions::Alphanumeric)
            .take(32)
            .map(char::from)
            .collect();
        
        let created_at = chrono::Utc::now();
        let expires_at = created_at + chrono::Duration::seconds(validity_seconds);
        
        AuthChallenge {
            nonce,
            created_at,
            expires_at,
            did,
        }
    }
    
    /// Check if the challenge has expired
    pub fn is_expired(&self) -> bool {
        chrono::Utc::now() > self.expires_at
    }
    
    /// Get the challenge message to be signed
    pub fn challenge_message(&self) -> String {
        format!(
            "Matrix DID Authentication\nDID: {}\nNonce: {}\nTimestamp: {}",
            self.did,
            self.nonce,
            self.created_at.to_rfc3339()
        )
    }
}

/// DID authentication service
pub struct DIDAuthService {
    mapping: DIDMapping,
    /// Active challenges
    challenges: Arc<RwLock<HashMap<String, AuthChallenge>>>,
}

impl DIDAuthService {
    /// Create a new DID authentication service
    pub fn new() -> Self {
        DIDAuthService {
            mapping: DIDMapping::new(),
            challenges: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Create an authentication challenge for a DID
    pub async fn create_challenge(&self, did: String) -> anyhow::Result<AuthChallenge> {
        let challenge = AuthChallenge::new(did.clone(), 300); // 5 minutes validity
        
        let mut challenges = self.challenges.write().await;
        challenges.insert(did, challenge.clone());
        
        Ok(challenge)
    }
    
    /// Verify a challenge response
    pub async fn verify_challenge(
        &self,
        did: &str,
        signature: &[u8],
    ) -> anyhow::Result<bool> {
        let challenges = self.challenges.read().await;
        let challenge = challenges
            .get(did)
            .context("Challenge not found")?;
        
        if challenge.is_expired() {
            return Ok(false);
        }
        
        // In a real implementation, this would use the DID manager to verify the signature
        // For now, we'll return true if a signature is provided
        Ok(!signature.is_empty())
    }
    
    /// Complete authentication and create DID mapping
    pub async fn complete_authentication(
        &self,
        did: String,
        localpart: String,
    ) -> anyhow::Result<()> {
        // Remove the challenge
        let mut challenges = self.challenges.write().await;
        challenges.remove(&did);
        
        // Create the mapping
        self.mapping.associate(did, localpart).await
    }
    
    /// Get the DID mapping service
    pub fn mapping(&self) -> &DIDMapping {
        &self.mapping
    }
}

impl Default for DIDAuthService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_did_mapping() {
        let mapping = DIDMapping::new();
        
        mapping
            .associate("did:peer:123".to_string(), "alice".to_string())
            .await
            .unwrap();
        
        let localpart = mapping.get_localpart("did:peer:123").await;
        assert_eq!(localpart, Some("alice".to_string()));
        
        let did = mapping.get_did("alice").await;
        assert_eq!(did, Some("did:peer:123".to_string()));
    }
    
    #[tokio::test]
    async fn test_auth_challenge() {
        let challenge = AuthChallenge::new("did:peer:123".to_string(), 300);
        
        assert!(!challenge.is_expired());
        assert!(challenge.nonce.len() == 32);
        
        let message = challenge.challenge_message();
        assert!(message.contains("did:peer:123"));
        assert!(message.contains(&challenge.nonce));
    }
    
    #[tokio::test]
    async fn test_did_auth_service() {
        let service = DIDAuthService::new();
        
        let challenge = service
            .create_challenge("did:peer:123".to_string())
            .await
            .unwrap();
        
        assert!(!challenge.is_expired());
        
        let valid = service
            .verify_challenge("did:peer:123", b"signature")
            .await
            .unwrap();
        
        assert!(valid);
    }
}
