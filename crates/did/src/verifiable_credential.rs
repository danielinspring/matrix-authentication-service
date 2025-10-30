// Copyright 2024, 2025 New Vector Ltd.
//
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Element-Commercial
// Please see LICENSE files in the repository root for full details.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::did_manager::DIDManager;
use crate::error::{DIDError, Result};
use crate::types::DID;

/// A Verifiable Credential as defined by W3C VC Data Model
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifiableCredential {
    /// JSON-LD context
    #[serde(rename = "@context")]
    pub context: Vec<String>,
    
    /// Credential ID
    pub id: Option<String>,
    
    /// Credential type(s)
    #[serde(rename = "type")]
    pub credential_type: Vec<String>,
    
    /// The entity that issued the credential
    pub issuer: Issuer,
    
    /// When the credential was issued
    pub issuance_date: DateTime<Utc>,
    
    /// When the credential expires (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiration_date: Option<DateTime<Utc>>,
    
    /// The subject of the credential
    pub credential_subject: CredentialSubject,
    
    /// Credential status (for revocation checking)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credential_status: Option<CredentialStatus>,
    
    /// Cryptographic proof
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proof: Option<Proof>,
}

impl VerifiableCredential {
    /// Create a new verifiable credential
    pub fn new(
        issuer: Issuer,
        credential_subject: CredentialSubject,
        credential_types: Vec<String>,
    ) -> Self {
        let mut types = vec!["VerifiableCredential".to_string()];
        types.extend(credential_types);
        
        VerifiableCredential {
            context: vec![
                "https://www.w3.org/2018/credentials/v1".to_string(),
                "https://www.w3.org/2018/credentials/examples/v1".to_string(),
            ],
            id: None,
            credential_type: types,
            issuer,
            issuance_date: Utc::now(),
            expiration_date: None,
            credential_subject,
            credential_status: None,
            proof: None,
        }
    }
    
    /// Set the credential ID
    pub fn with_id(mut self, id: String) -> Self {
        self.id = Some(id);
        self
    }
    
    /// Set the expiration date
    pub fn with_expiration(mut self, expiration: DateTime<Utc>) -> Self {
        self.expiration_date = Some(expiration);
        self
    }
    
    /// Check if the credential has expired
    pub fn is_expired(&self) -> bool {
        if let Some(expiration) = self.expiration_date {
            Utc::now() > expiration
        } else {
            false
        }
    }
    
    /// Sign the credential with a DID
    pub async fn sign(mut self, did_manager: &DIDManager, issuer_did: &DID) -> Result<Self> {
        // Create canonical representation for signing
        let mut vc_for_signing = self.clone();
        vc_for_signing.proof = None;
        
        let vc_json = serde_json::to_string(&vc_for_signing)
            .map_err(|e| DIDError::CryptoError(format!("Failed to serialize credential: {}", e)))?;
        
        // Sign the credential
        let signature = did_manager.sign(issuer_did, vc_json.as_bytes())?;
        
        // Create proof
        let proof = Proof {
            proof_type: "Ed25519Signature2020".to_string(),
            created: Utc::now(),
            verification_method: format!("{}#key-1", issuer_did.id),
            proof_purpose: "assertionMethod".to_string(),
            proof_value: base64ct::Base64::encode_string(&signature),
        };
        
        self.proof = Some(proof);
        Ok(self)
    }
    
    /// Verify the credential's proof
    pub async fn verify(&self, did_manager: &DIDManager) -> Result<bool> {
        let proof = self
            .proof
            .as_ref()
            .ok_or_else(|| DIDError::VerificationFailed("No proof present".to_string()))?;
        
        // Extract issuer DID
        let issuer_did_str = match &self.issuer {
            Issuer::DID(did) => did,
            Issuer::Object { id, .. } => id,
        };
        let issuer_did = DID::parse(issuer_did_str)?;
        
        // Recreate the signed data
        let mut vc_for_verification = self.clone();
        vc_for_verification.proof = None;
        
        let vc_json = serde_json::to_string(&vc_for_verification)
            .map_err(|e| DIDError::CryptoError(format!("Failed to serialize credential: {}", e)))?;
        
        // Decode signature
        let signature = base64ct::Base64::decode_vec(&proof.proof_value)
            .map_err(|e| DIDError::VerificationFailed(format!("Invalid signature encoding: {}", e)))?;
        
        // Verify signature
        did_manager.verify(&issuer_did, vc_json.as_bytes(), &signature)
    }
}

/// The issuer of a credential
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Issuer {
    /// Issuer as a DID string
    DID(String),
    /// Issuer as an object with additional properties
    Object {
        id: String,
        #[serde(flatten)]
        properties: serde_json::Value,
    },
}

/// The subject of a credential
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialSubject {
    /// The DID of the subject
    pub id: String,
    
    /// Additional claims about the subject
    #[serde(flatten)]
    pub claims: serde_json::Value,
}

impl CredentialSubject {
    /// Create a new credential subject
    pub fn new(id: String, claims: serde_json::Value) -> Self {
        CredentialSubject { id, claims }
    }
}

/// Credential status for revocation checking
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CredentialStatus {
    /// Status ID
    pub id: String,
    
    /// Status type
    #[serde(rename = "type")]
    pub status_type: String,
}

/// Cryptographic proof for a credential
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Proof {
    /// The type of proof
    #[serde(rename = "type")]
    pub proof_type: String,
    
    /// When the proof was created
    pub created: DateTime<Utc>,
    
    /// The verification method used
    pub verification_method: String,
    
    /// The purpose of the proof
    pub proof_purpose: String,
    
    /// The signature value
    pub proof_value: String,
}

/// Helper struct for creating credentials
pub struct Credential;

impl Credential {
    /// Create a Matrix user credential
    pub fn matrix_user(
        issuer_did: String,
        subject_did: String,
        matrix_id: String,
        displayname: Option<String>,
    ) -> VerifiableCredential {
        let claims = serde_json::json!({
            "matrixId": matrix_id,
            "displayName": displayname,
        });
        
        let subject = CredentialSubject::new(subject_did, claims);
        
        VerifiableCredential::new(
            Issuer::DID(issuer_did),
            subject,
            vec!["MatrixUserCredential".to_string()],
        )
    }
    
    /// Create a device credential
    pub fn device(
        issuer_did: String,
        subject_did: String,
        device_id: String,
        device_keys: serde_json::Value,
    ) -> VerifiableCredential {
        let claims = serde_json::json!({
            "deviceId": device_id,
            "deviceKeys": device_keys,
        });
        
        let subject = CredentialSubject::new(subject_did, claims);
        
        VerifiableCredential::new(
            Issuer::DID(issuer_did),
            subject,
            vec!["DeviceCredential".to_string()],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_create_credential() {
        let issuer = Issuer::DID("did:peer:123".to_string());
        let subject = CredentialSubject::new(
            "did:peer:456".to_string(),
            serde_json::json!({ "name": "Alice" }),
        );
        
        let vc = VerifiableCredential::new(issuer, subject, vec!["TestCredential".to_string()]);
        
        assert_eq!(vc.credential_type.len(), 2);
        assert!(vc.credential_type.contains(&"VerifiableCredential".to_string()));
        assert!(!vc.is_expired());
    }
    
    #[test]
    fn test_matrix_user_credential() {
        let vc = Credential::matrix_user(
            "did:peer:123".to_string(),
            "did:peer:456".to_string(),
            "@alice:example.com".to_string(),
            Some("Alice".to_string()),
        );
        
        assert!(vc.credential_type.contains(&"MatrixUserCredential".to_string()));
        assert_eq!(vc.credential_subject.id, "did:peer:456");
    }
    
    #[tokio::test]
    async fn test_sign_and_verify_credential() {
        let did_manager = DIDManager::new("test-vc-service");
        let (issuer_did, _) = did_manager.create_peer_did().await.unwrap();
        
        let subject = CredentialSubject::new(
            "did:peer:subject".to_string(),
            serde_json::json!({ "name": "Test" }),
        );
        
        let vc = VerifiableCredential::new(
            Issuer::DID(issuer_did.id.clone()),
            subject,
            vec!["TestCredential".to_string()],
        );
        
        let signed_vc = vc.sign(&did_manager, &issuer_did).await.unwrap();
        assert!(signed_vc.proof.is_some());
        
        let valid = signed_vc.verify(&did_manager).await.unwrap();
        assert!(valid);
        
        // Clean up
        did_manager.delete_did(&issuer_did).unwrap();
    }
}
