// Copyright 2024, 2025 New Vector Ltd.
//
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Element-Commercial
// Please see LICENSE files in the repository root for full details.

use serde::{Deserialize, Serialize};

use crate::error::{DIDError, Result};

/// A Decentralized Identifier (DID) as defined by W3C DID Core specification
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DID {
    /// The complete DID string (e.g., "did:peer:2.Ez6LSbysY2xFMRpGMhb7tFTLMpeuPRaqaWM1yECx2AtzE3KCc")
    pub id: String,
    
    /// The DID method (e.g., "peer", "web", "key")
    pub method: String,
    
    /// The method-specific identifier
    pub method_specific_id: String,
}

impl DID {
    /// Parse a DID string into its components
    pub fn parse(did_string: &str) -> Result<Self> {
        let parts: Vec<&str> = did_string.split(':').collect();
        
        if parts.len() < 3 {
            return Err(DIDError::InvalidDIDFormat(
                "DID must have at least 3 parts separated by colons".to_string(),
            ));
        }
        
        if parts[0] != "did" {
            return Err(DIDError::InvalidDIDFormat(
                "DID must start with 'did:'".to_string(),
            ));
        }
        
        let method = parts[1].to_string();
        let method_specific_id = parts[2..].join(":");
        
        Ok(DID {
            id: did_string.to_string(),
            method,
            method_specific_id,
        })
    }
    
    /// Create a new DID from method and method-specific ID
    pub fn new(method: impl Into<String>, method_specific_id: impl Into<String>) -> Self {
        let method = method.into();
        let method_specific_id = method_specific_id.into();
        let id = format!("did:{}:{}", method, method_specific_id);
        
        DID {
            id,
            method,
            method_specific_id,
        }
    }
    
    /// Get the DID as a string
    pub fn as_str(&self) -> &str {
        &self.id
    }
}

impl std::fmt::Display for DID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.id)
    }
}

impl std::str::FromStr for DID {
    type Err = DIDError;
    
    fn from_str(s: &str) -> Result<Self> {
        DID::parse(s)
    }
}

/// A DID URL, which may include a DID plus path, query, or fragment
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DIDUrl {
    /// The base DID
    pub did: DID,
    
    /// Optional path component
    pub path: Option<String>,
    
    /// Optional query component
    pub query: Option<String>,
    
    /// Optional fragment component (often used to reference verification methods)
    pub fragment: Option<String>,
}

impl DIDUrl {
    /// Parse a DID URL string
    pub fn parse(url_string: &str) -> Result<Self> {
        // Split on fragment first
        let (did_part, fragment) = if let Some(pos) = url_string.find('#') {
            (
                &url_string[..pos],
                Some(url_string[pos + 1..].to_string()),
            )
        } else {
            (url_string, None)
        };
        
        // Split on query
        let (did_path_part, query) = if let Some(pos) = did_part.find('?') {
            (
                &did_part[..pos],
                Some(did_part[pos + 1..].to_string()),
            )
        } else {
            (did_part, None)
        };
        
        // Find where the DID ends (at the first '/' after the method-specific ID)
        let did_end = did_path_part
            .char_indices()
            .filter(|(_, c)| *c == ':')
            .nth(2) // After "did:method:"
            .map(|(i, _)| {
                did_path_part[i..]
                    .find('/')
                    .map(|j| i + j)
                    .unwrap_or(did_path_part.len())
            })
            .unwrap_or(did_path_part.len());
        
        let did = DID::parse(&did_path_part[..did_end])?;
        let path = if did_end < did_path_part.len() {
            Some(did_path_part[did_end..].to_string())
        } else {
            None
        };
        
        Ok(DIDUrl {
            did,
            path,
            query,
            fragment,
        })
    }
    
    /// Create a DID URL from a DID
    pub fn from_did(did: DID) -> Self {
        DIDUrl {
            did,
            path: None,
            query: None,
            fragment: None,
        }
    }
    
    /// Add a fragment to this DID URL
    pub fn with_fragment(mut self, fragment: impl Into<String>) -> Self {
        self.fragment = Some(fragment.into());
        self
    }
}

impl std::fmt::Display for DIDUrl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.did)?;
        if let Some(path) = &self.path {
            write!(f, "{}", path)?;
        }
        if let Some(query) = &self.query {
            write!(f, "?{}", query)?;
        }
        if let Some(fragment) = &self.fragment {
            write!(f, "#{}", fragment)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_did() {
        let did = DID::parse("did:peer:2.Ez6LSbysY2xFMRpGMhb7tFTLMpeuPRaqaWM1yECx2AtzE3KCc").unwrap();
        assert_eq!(did.method, "peer");
        assert_eq!(did.method_specific_id, "2.Ez6LSbysY2xFMRpGMhb7tFTLMpeuPRaqaWM1yECx2AtzE3KCc");
    }
    
    #[test]
    fn test_parse_did_web() {
        let did = DID::parse("did:web:example.com").unwrap();
        assert_eq!(did.method, "web");
        assert_eq!(did.method_specific_id, "example.com");
    }
    
    #[test]
    fn test_parse_did_url_with_fragment() {
        let did_url = DIDUrl::parse("did:peer:123abc#key-1").unwrap();
        assert_eq!(did_url.did.method_specific_id, "123abc");
        assert_eq!(did_url.fragment.unwrap(), "key-1");
    }
}
