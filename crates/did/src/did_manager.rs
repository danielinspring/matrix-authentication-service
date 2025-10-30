// Copyright 2024, 2025 New Vector Ltd.
//
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Element-Commercial
// Please see LICENSE files in the repository root for full details.

use std::sync::Arc;

use crate::did_document::{DIDDocument, Service, VerificationMethod, VerificationRelationship};
use crate::error::{DIDError, Result};
use crate::key_manager::{KeyManager, KeyPair};
use crate::resolver::{KeyDIDResolver, PeerDIDResolver, ResolverRegistry, WebDIDResolver};
use crate::types::DID;

/// Manager for creating and managing DIDs
pub struct DIDManager {
    key_manager: KeyManager,
    resolver_registry: ResolverRegistry,
}

impl DIDManager {
    /// Create a new DID manager
    pub fn new(service_name: impl Into<String>) -> Self {
        let key_manager = KeyManager::new(service_name);
        
        // Register default resolvers
        let resolver_registry = ResolverRegistry::new()
            .register("peer", Arc::new(PeerDIDResolver::new()))
            .register("web", Arc::new(WebDIDResolver::new()))
            .register("key", Arc::new(KeyDIDResolver::new()));
        
        DIDManager {
            key_manager,
            resolver_registry,
        }
    }
    
    /// Create a new did:peer:2 DID with signing and encryption keys
    pub async fn create_peer_did(&self) -> Result<(DID, DIDDocument)> {
        // Generate keys
        let signing_key = KeyPair::generate_ed25519()?;
        let encryption_key = KeyPair::generate_x25519()?;
        
        // Build did:peer:2 identifier
        // Format: did:peer:2.V<signing_key>.E<encryption_key>
        
        // For multibase encoding, we use base58-btc (z prefix)
        let signing_key_mb = signing_key.public_key_multibase();
        let encryption_key_mb = encryption_key.public_key_multibase();
        
        // Remove the 'u' prefix from base64url and convert to base58-like format
        // For simplicity, we'll use a basic encoding here
        // In production, proper multibase/multicodec encoding should be used
        let signing_encoded = base58_encode(&signing_key.public_key);
        let encryption_encoded = base58_encode(&encryption_key.public_key);
        
        let method_specific_id = format!("2.V{}.E{}", signing_encoded, encryption_encoded);
        let did = DID::new("peer", method_specific_id);
        
        // Store keys
        self.key_manager
            .store_key(&format!("{}-signing", did.id), &signing_key)?;
        self.key_manager
            .store_key(&format!("{}-encryption", did.id), &encryption_key)?;
        
        // Build DID document
        let signing_method_id = format!("{}#key-1", did.id);
        let encryption_method_id = format!("{}#key-2", did.id);
        
        let signing_method = VerificationMethod::new(
            signing_method_id.clone(),
            "Ed25519VerificationKey2020".to_string(),
            did.id.clone(),
            Some(signing_key.public_key_multibase()),
        );
        
        let encryption_method = VerificationMethod::new(
            encryption_method_id.clone(),
            "X25519KeyAgreementKey2020".to_string(),
            did.id.clone(),
            Some(encryption_key.public_key_multibase()),
        );
        
        let doc = DIDDocument::new(did.clone())
            .add_verification_method(signing_method)
            .add_verification_method(encryption_method)
            .add_authentication(VerificationRelationship::Reference(signing_method_id.clone()))
            .add_assertion_method(VerificationRelationship::Reference(signing_method_id))
            .add_key_agreement(VerificationRelationship::Reference(encryption_method_id));
        
        Ok((did, doc))
    }
    
    /// Create a new did:key DID from an Ed25519 key
    pub async fn create_key_did(&self) -> Result<(DID, DIDDocument)> {
        let keypair = KeyPair::generate_ed25519()?;
        
        // did:key uses multibase encoding of the public key
        let multibase_key = keypair.public_key_multibase();
        let did = DID::new("key", multibase_key);
        
        // Store the key
        self.key_manager
            .store_key(&format!("{}-signing", did.id), &keypair)?;
        
        // Resolve to get the document
        let doc = self.resolver_registry.resolve(&did).await?;
        
        Ok((did, doc))
    }
    
    /// Add a service endpoint to a DID document
    pub fn add_service_endpoint(
        &self,
        mut doc: DIDDocument,
        service_type: impl Into<String>,
        endpoint: impl Into<String>,
    ) -> DIDDocument {
        let service_id = format!("{}#service-{}", doc.id, doc.service.as_ref().map_or(1, |s| s.len() + 1));
        
        let service = Service::new(service_id, service_type.into(), endpoint.into());
        doc.add_service(service)
    }
    
    /// Resolve a DID to its document
    pub async fn resolve(&self, did: &DID) -> Result<DIDDocument> {
        self.resolver_registry.resolve(did).await
    }
    
    /// Get the signing key for a DID
    pub fn get_signing_key(&self, did: &DID) -> Result<KeyPair> {
        self.key_manager
            .retrieve_key(&format!("{}-signing", did.id))
    }
    
    /// Get the encryption key for a DID
    pub fn get_encryption_key(&self, did: &DID) -> Result<KeyPair> {
        self.key_manager
            .retrieve_key(&format!("{}-encryption", did.id))
    }
    
    /// Sign data with a DID's signing key
    pub fn sign(&self, did: &DID, data: &[u8]) -> Result<Vec<u8>> {
        let keypair = self.get_signing_key(did)?;
        keypair.sign(data)
    }
    
    /// Verify a signature using a DID's signing key
    pub fn verify(&self, did: &DID, data: &[u8], signature: &[u8]) -> Result<bool> {
        let keypair = self.get_signing_key(did)?;
        keypair.verify(data, signature)
    }
    
    /// Delete a DID and its associated keys
    pub fn delete_did(&self, did: &DID) -> Result<()> {
        // Try to delete both signing and encryption keys
        // Ignore errors if keys don't exist
        let _ = self.key_manager.delete_key(&format!("{}-signing", did.id));
        let _ = self.key_manager.delete_key(&format!("{}-encryption", did.id));
        Ok(())
    }
}

/// Simple base58 encoding for key representation
fn base58_encode(data: &[u8]) -> String {
    // This is a simplified implementation
    // In production, use a proper base58 library like bs58
    base64ct::Base64::encode_string(data)
        .chars()
        .filter(|c| c.is_alphanumeric())
        .take(43)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_create_peer_did() {
        let manager = DIDManager::new("test-service");
        let (did, doc) = manager.create_peer_did().await.unwrap();
        
        assert_eq!(did.method, "peer");
        assert!(did.method_specific_id.starts_with("2."));
        assert!(doc.verification_method.is_some());
        
        // Clean up
        manager.delete_did(&did).unwrap();
    }
    
    #[tokio::test]
    async fn test_sign_and_verify() {
        let manager = DIDManager::new("test-service");
        let (did, _) = manager.create_peer_did().await.unwrap();
        
        let data = b"test message";
        let signature = manager.sign(&did, data).unwrap();
        let valid = manager.verify(&did, data, &signature).unwrap();
        
        assert!(valid);
        
        // Clean up
        manager.delete_did(&did).unwrap();
    }
}
