// Copyright 2024, 2025 New Vector Ltd.
//
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Element-Commercial
// Please see LICENSE files in the repository root for full details.

use serde::{Deserialize, Serialize};

use crate::types::{DIDUrl, DID};

/// A DID Document as defined by W3C DID Core specification
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DIDDocument {
    /// The DID subject
    pub id: String,
    
    /// Alternative representations of the DID subject
    #[serde(skip_serializing_if = "Option::is_none")]
    pub also_known_as: Option<Vec<String>>,
    
    /// Controllers of the DID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub controller: Option<Vec<String>>,
    
    /// Verification methods
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification_method: Option<Vec<VerificationMethod>>,
    
    /// Authentication verification relationships
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authentication: Option<Vec<VerificationRelationship>>,
    
    /// Assertion method verification relationships
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assertion_method: Option<Vec<VerificationRelationship>>,
    
    /// Key agreement verification relationships
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_agreement: Option<Vec<VerificationRelationship>>,
    
    /// Capability invocation verification relationships
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capability_invocation: Option<Vec<VerificationRelationship>>,
    
    /// Capability delegation verification relationships
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capability_delegation: Option<Vec<VerificationRelationship>>,
    
    /// Service endpoints
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service: Option<Vec<Service>>,
}

impl DIDDocument {
    /// Create a new DID document with the given DID
    pub fn new(did: DID) -> Self {
        DIDDocument {
            id: did.id,
            also_known_as: None,
            controller: None,
            verification_method: None,
            authentication: None,
            assertion_method: None,
            key_agreement: None,
            capability_invocation: None,
            capability_delegation: None,
            service: None,
        }
    }
    
    /// Add a verification method to the document
    pub fn add_verification_method(mut self, method: VerificationMethod) -> Self {
        self.verification_method
            .get_or_insert_with(Vec::new)
            .push(method);
        self
    }
    
    /// Add an authentication relationship
    pub fn add_authentication(mut self, relationship: VerificationRelationship) -> Self {
        self.authentication
            .get_or_insert_with(Vec::new)
            .push(relationship);
        self
    }
    
    /// Add an assertion method relationship
    pub fn add_assertion_method(mut self, relationship: VerificationRelationship) -> Self {
        self.assertion_method
            .get_or_insert_with(Vec::new)
            .push(relationship);
        self
    }
    
    /// Add a key agreement relationship
    pub fn add_key_agreement(mut self, relationship: VerificationRelationship) -> Self {
        self.key_agreement
            .get_or_insert_with(Vec::new)
            .push(relationship);
        self
    }
    
    /// Add a service endpoint
    pub fn add_service(mut self, service: Service) -> Self {
        self.service.get_or_insert_with(Vec::new).push(service);
        self
    }
    
    /// Get a verification method by ID
    pub fn get_verification_method(&self, id: &str) -> Option<&VerificationMethod> {
        self.verification_method
            .as_ref()?
            .iter()
            .find(|m| m.id == id)
    }
}

/// A verification method in a DID document
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerificationMethod {
    /// The verification method ID (usually a DID URL with fragment)
    pub id: String,
    
    /// The type of verification method
    #[serde(rename = "type")]
    pub method_type: String,
    
    /// The controller of this verification method
    pub controller: String,
    
    /// Public key in multibase format
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_key_multibase: Option<String>,
    
    /// Public key in JWK format
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_key_jwk: Option<serde_json::Value>,
}

impl VerificationMethod {
    /// Create a new verification method
    pub fn new(
        id: String,
        method_type: String,
        controller: String,
        public_key_multibase: Option<String>,
    ) -> Self {
        VerificationMethod {
            id,
            method_type,
            controller,
            public_key_multibase,
            public_key_jwk: None,
        }
    }
}

/// A verification relationship can be either a reference or an embedded method
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum VerificationRelationship {
    /// Reference to a verification method by ID
    Reference(String),
    /// Embedded verification method
    Embedded(VerificationMethod),
}

/// A service endpoint in a DID document
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Service {
    /// The service ID
    pub id: String,
    
    /// The service type
    #[serde(rename = "type")]
    pub service_type: String,
    
    /// The service endpoint URL(s)
    pub service_endpoint: ServiceEndpoint,
}

impl Service {
    /// Create a new service with a single endpoint
    pub fn new(id: String, service_type: String, endpoint: String) -> Self {
        Service {
            id,
            service_type,
            service_endpoint: ServiceEndpoint::Single(endpoint),
        }
    }
}

/// Service endpoint can be a single URL or multiple URLs
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ServiceEndpoint {
    /// Single endpoint
    Single(String),
    /// Multiple endpoints
    Multiple(Vec<String>),
    /// Complex endpoint with properties
    Complex(serde_json::Value),
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_create_did_document() {
        let did = DID::new("peer", "123abc");
        let doc = DIDDocument::new(did);
        
        assert_eq!(doc.id, "did:peer:123abc");
    }
    
    #[test]
    fn test_add_verification_method() {
        let did = DID::new("peer", "123abc");
        let method = VerificationMethod::new(
            "did:peer:123abc#key-1".to_string(),
            "Ed25519VerificationKey2020".to_string(),
            "did:peer:123abc".to_string(),
            Some("z6MkpTHR8VNsBxYAAWHut2Geadd9jSwuBV8xRoAnwWsdvktH".to_string()),
        );
        
        let doc = DIDDocument::new(did).add_verification_method(method);
        
        assert_eq!(doc.verification_method.as_ref().unwrap().len(), 1);
    }
    
    #[test]
    fn test_serialize_did_document() {
        let did = DID::new("peer", "123abc");
        let doc = DIDDocument::new(did);
        
        let json = serde_json::to_string(&doc).unwrap();
        assert!(json.contains("did:peer:123abc"));
    }
}
