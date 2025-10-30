// Copyright 2024, 2025 New Vector Ltd.
//
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Element-Commercial
// Please see LICENSE files in the repository root for full details.

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

use crate::did_document::{DIDDocument, Service, VerificationMethod, VerificationRelationship};
use crate::error::{DIDError, Result};
use crate::types::DID;

/// Trait for DID resolution
#[async_trait]
pub trait DIDResolver: Send + Sync {
    /// Resolve a DID to its DID Document
    async fn resolve(&self, did: &DID) -> Result<DIDDocument>;
    
    /// Check if this resolver supports the given DID method
    fn supports_method(&self, method: &str) -> bool;
}

/// Registry of DID resolvers for different methods
pub struct ResolverRegistry {
    resolvers: HashMap<String, Arc<dyn DIDResolver>>,
}

impl ResolverRegistry {
    /// Create a new resolver registry
    pub fn new() -> Self {
        ResolverRegistry {
            resolvers: HashMap::new(),
        }
    }
    
    /// Register a resolver for a specific DID method
    pub fn register(mut self, method: impl Into<String>, resolver: Arc<dyn DIDResolver>) -> Self {
        self.resolvers.insert(method.into(), resolver);
        self
    }
    
    /// Resolve a DID using the appropriate resolver
    pub async fn resolve(&self, did: &DID) -> Result<DIDDocument> {
        let resolver = self
            .resolvers
            .get(&did.method)
            .ok_or_else(|| DIDError::UnsupportedMethod(did.method.clone()))?;
        
        resolver.resolve(did).await
    }
}

impl Default for ResolverRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Resolver for did:peer method (numalgo 2)
pub struct PeerDIDResolver;

impl PeerDIDResolver {
    /// Create a new did:peer resolver
    pub fn new() -> Self {
        PeerDIDResolver
    }
}

impl Default for PeerDIDResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DIDResolver for PeerDIDResolver {
    async fn resolve(&self, did: &DID) -> Result<DIDDocument> {
        if did.method != "peer" {
            return Err(DIDError::UnsupportedMethod(did.method.clone()));
        }
        
        // Parse the did:peer:2 format
        // Format: did:peer:2.<numalgo2string>
        // numalgo2string: .V<verification>.E<encryption>.S<service>
        
        let method_id = &did.method_specific_id;
        if !method_id.starts_with("2.") {
            return Err(DIDError::InvalidDIDFormat(
                "did:peer must use numalgo 2 (start with '2.')".to_string(),
            ));
        }
        
        let mut doc = DIDDocument::new(did.clone());
        let parts = method_id[2..].split('.').collect::<Vec<_>>();
        
        let mut key_index = 1;
        
        for part in parts {
            if part.is_empty() {
                continue;
            }
            
            let prefix = &part[..1];
            let encoded_key = &part[1..];
            
            match prefix {
                "V" => {
                    // Verification key (Ed25519)
                    let method_id = format!("{}#key-{}", did.id, key_index);
                    let method = VerificationMethod::new(
                        method_id.clone(),
                        "Ed25519VerificationKey2020".to_string(),
                        did.id.clone(),
                        Some(format!("z{}", encoded_key)), // multibase format
                    );
                    
                    doc = doc
                        .add_verification_method(method.clone())
                        .add_authentication(VerificationRelationship::Reference(method_id.clone()))
                        .add_assertion_method(VerificationRelationship::Reference(method_id));
                    
                    key_index += 1;
                }
                "E" => {
                    // Encryption/key agreement key (X25519)
                    let method_id = format!("{}#key-{}", did.id, key_index);
                    let method = VerificationMethod::new(
                        method_id.clone(),
                        "X25519KeyAgreementKey2020".to_string(),
                        did.id.clone(),
                        Some(format!("z{}", encoded_key)),
                    );
                    
                    doc = doc
                        .add_verification_method(method)
                        .add_key_agreement(VerificationRelationship::Reference(method_id));
                    
                    key_index += 1;
                }
                "S" => {
                    // Service endpoint
                    // For simplicity, we'll parse basic service endpoints
                    // In production, this would need more sophisticated parsing
                    let service = Service::new(
                        format!("{}#service-1", did.id),
                        "DIDCommMessaging".to_string(),
                        format!("https://example.com/didcomm"),
                    );
                    doc = doc.add_service(service);
                }
                _ => {
                    tracing::warn!("Unknown did:peer element prefix: {}", prefix);
                }
            }
        }
        
        Ok(doc)
    }
    
    fn supports_method(&self, method: &str) -> bool {
        method == "peer"
    }
}

/// Resolver for did:web method
pub struct WebDIDResolver {
    client: reqwest::Client,
}

impl WebDIDResolver {
    /// Create a new did:web resolver
    pub fn new() -> Self {
        WebDIDResolver {
            client: reqwest::Client::new(),
        }
    }
}

impl Default for WebDIDResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DIDResolver for WebDIDResolver {
    async fn resolve(&self, did: &DID) -> Result<DIDDocument> {
        if did.method != "web" {
            return Err(DIDError::UnsupportedMethod(did.method.clone()));
        }
        
        // Convert did:web:example.com to https://example.com/.well-known/did.json
        // Convert did:web:example.com:path to https://example.com/path/did.json
        
        let domain_and_path = did.method_specific_id.replace(':', "/");
        let url = if domain_and_path.contains('/') {
            format!("https://{}/did.json", domain_and_path)
        } else {
            format!("https://{}/.well-known/did.json", domain_and_path)
        };
        
        tracing::debug!("Resolving did:web from URL: {}", url);
        
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| DIDError::ResolutionFailed(format!("Failed to fetch DID document: {}", e)))?;
        
        if !response.status().is_success() {
            return Err(DIDError::ResolutionFailed(format!(
                "HTTP error: {}",
                response.status()
            )));
        }
        
        let doc: DIDDocument = response
            .json()
            .await
            .map_err(|e| DIDError::ResolutionFailed(format!("Failed to parse DID document: {}", e)))?;
        
        // Verify the DID in the document matches the requested DID
        if doc.id != did.id {
            return Err(DIDError::InvalidDocument(format!(
                "DID mismatch: expected {}, got {}",
                did.id, doc.id
            )));
        }
        
        Ok(doc)
    }
    
    fn supports_method(&self, method: &str) -> bool {
        method == "web"
    }
}

/// Resolver for did:key method
pub struct KeyDIDResolver;

impl KeyDIDResolver {
    /// Create a new did:key resolver
    pub fn new() -> Self {
        KeyDIDResolver
    }
}

impl Default for KeyDIDResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl DIDResolver for KeyDIDResolver {
    async fn resolve(&self, did: &DID) -> Result<DIDDocument> {
        if did.method != "key" {
            return Err(DIDError::UnsupportedMethod(did.method.clone()));
        }
        
        // did:key is deterministically generated from the key itself
        // The method-specific-id is a multibase-encoded public key
        
        let mut doc = DIDDocument::new(did.clone());
        
        // Create a single verification method from the key
        let method_id = format!("{}#{}", did.id, did.method_specific_id);
        let method = VerificationMethod::new(
            method_id.clone(),
            "Ed25519VerificationKey2020".to_string(),
            did.id.clone(),
            Some(did.method_specific_id.clone()),
        );
        
        doc = doc
            .add_verification_method(method)
            .add_authentication(VerificationRelationship::Reference(method_id.clone()))
            .add_assertion_method(VerificationRelationship::Reference(method_id.clone()))
            .add_capability_invocation(VerificationRelationship::Reference(method_id.clone()))
            .add_capability_delegation(VerificationRelationship::Reference(method_id));
        
        Ok(doc)
    }
    
    fn supports_method(&self, method: &str) -> bool {
        method == "key"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_peer_did_resolver() {
        let resolver = PeerDIDResolver::new();
        
        // Simple did:peer:2 with one verification key
        let did = DID::parse("did:peer:2.VzXwpBnMdCm1cLmKuzgESn29nqnonp1ioqrQMRHNsmjMyppzx8xB2pv7cw8q1PdDacSrdWE3dtB9f7Nxk886mdzNFoPtY").unwrap();
        
        let doc = resolver.resolve(&did).await.unwrap();
        assert_eq!(doc.id, did.id);
        assert!(doc.verification_method.is_some());
    }
    
    #[test]
    fn test_resolver_registry() {
        let mut registry = ResolverRegistry::new();
        registry = registry.register("peer", Arc::new(PeerDIDResolver::new()));
        registry = registry.register("key", Arc::new(KeyDIDResolver::new()));
        
        assert!(registry.resolvers.contains_key("peer"));
        assert!(registry.resolvers.contains_key("key"));
    }
}
