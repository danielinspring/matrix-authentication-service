// Copyright 2024, 2025 New Vector Ltd.
//
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Element-Commercial
// Please see LICENSE files in the repository root for full details.

use ed25519_dalek::{SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use x25519_dalek::{PublicKey, StaticSecret};
use zeroize::Zeroizing;

use crate::error::{DIDError, Result};

/// Supported key types for DID cryptographic operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyType {
    /// Ed25519 signing key
    Ed25519,
    /// X25519 key agreement (encryption)
    X25519,
}

impl std::fmt::Display for KeyType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KeyType::Ed25519 => write!(f, "Ed25519VerificationKey2020"),
            KeyType::X25519 => write!(f, "X25519KeyAgreementKey2020"),
        }
    }
}

/// A cryptographic key pair
#[derive(Clone)]
pub struct KeyPair {
    /// The type of key
    pub key_type: KeyType,
    
    /// The public key bytes
    pub public_key: Vec<u8>,
    
    /// The private key bytes (zeroized on drop)
    private_key: Zeroizing<Vec<u8>>,
}

impl KeyPair {
    /// Generate a new Ed25519 signing key pair
    pub fn generate_ed25519() -> Result<Self> {
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key = signing_key.verifying_key();
        
        Ok(KeyPair {
            key_type: KeyType::Ed25519,
            public_key: verifying_key.to_bytes().to_vec(),
            private_key: Zeroizing::new(signing_key.to_bytes().to_vec()),
        })
    }
    
    /// Generate a new X25519 key agreement key pair
    pub fn generate_x25519() -> Result<Self> {
        let secret = StaticSecret::random_from_rng(OsRng);
        let public = PublicKey::from(&secret);
        
        Ok(KeyPair {
            key_type: KeyType::X25519,
            public_key: public.as_bytes().to_vec(),
            private_key: Zeroizing::new(secret.to_bytes().to_vec()),
        })
    }
    
    /// Get the public key as base64
    pub fn public_key_base64(&self) -> String {
        base64ct::Base64::encode_string(&self.public_key)
    }
    
    /// Get the public key as multibase (base58-btc)
    pub fn public_key_multibase(&self) -> String {
        // For now, use base64url multibase
        format!("u{}", base64ct::Base64UrlUnpadded::encode_string(&self.public_key))
    }
    
    /// Sign data with this key pair (Ed25519 only)
    pub fn sign(&self, data: &[u8]) -> Result<Vec<u8>> {
        if self.key_type != KeyType::Ed25519 {
            return Err(DIDError::InvalidKeyType(
                "Only Ed25519 keys can sign".to_string(),
            ));
        }
        
        let signing_key = SigningKey::from_bytes(
            self.private_key
                .as_slice()
                .try_into()
                .map_err(|_| DIDError::CryptoError("Invalid key length".to_string()))?,
        );
        
        use ed25519_dalek::Signer;
        let signature = signing_key.sign(data);
        Ok(signature.to_bytes().to_vec())
    }
    
    /// Verify a signature with this key pair's public key (Ed25519 only)
    pub fn verify(&self, data: &[u8], signature: &[u8]) -> Result<bool> {
        if self.key_type != KeyType::Ed25519 {
            return Err(DIDError::InvalidKeyType(
                "Only Ed25519 keys can verify".to_string(),
            ));
        }
        
        let verifying_key = VerifyingKey::from_bytes(
            self.public_key
                .as_slice()
                .try_into()
                .map_err(|_| DIDError::CryptoError("Invalid key length".to_string()))?,
        )
        .map_err(|e| DIDError::CryptoError(format!("Invalid public key: {}", e)))?;
        
        let sig = ed25519_dalek::Signature::from_bytes(
            signature
                .try_into()
                .map_err(|_| DIDError::CryptoError("Invalid signature length".to_string()))?,
        );
        
        use ed25519_dalek::Verifier;
        Ok(verifying_key.verify(data, &sig).is_ok())
    }
    
    /// Get the private key bytes (use with caution!)
    pub fn private_key_bytes(&self) -> &[u8] {
        &self.private_key
    }
}

impl std::fmt::Debug for KeyPair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KeyPair")
            .field("key_type", &self.key_type)
            .field("public_key", &hex::encode(&self.public_key))
            .field("private_key", &"<redacted>")
            .finish()
    }
}

/// Key manager for secure key storage and retrieval
pub struct KeyManager {
    /// The service name for keyring storage
    service: String,
}

impl KeyManager {
    /// Create a new key manager
    pub fn new(service: impl Into<String>) -> Self {
        KeyManager {
            service: service.into(),
        }
    }
    
    /// Store a key pair securely in the platform keyring
    pub fn store_key(&self, key_id: &str, key_pair: &KeyPair) -> Result<()> {
        let entry = keyring::Entry::new(&self.service, key_id)
            .map_err(|e| DIDError::KeyStorageFailed(format!("Failed to create keyring entry: {}", e)))?;
        
        // Serialize the key data
        let key_data = serde_json::json!({
            "key_type": key_pair.key_type,
            "public_key": hex::encode(&key_pair.public_key),
            "private_key": hex::encode(key_pair.private_key_bytes()),
        });
        
        let key_json = serde_json::to_string(&key_data)
            .map_err(|e| DIDError::KeyStorageFailed(format!("Failed to serialize key: {}", e)))?;
        
        entry
            .set_password(&key_json)
            .map_err(|e| DIDError::KeyStorageFailed(format!("Failed to store key: {}", e)))?;
        
        Ok(())
    }
    
    /// Retrieve a key pair from the platform keyring
    pub fn retrieve_key(&self, key_id: &str) -> Result<KeyPair> {
        let entry = keyring::Entry::new(&self.service, key_id)
            .map_err(|e| DIDError::KeyRetrievalFailed(format!("Failed to create keyring entry: {}", e)))?;
        
        let key_json = entry
            .get_password()
            .map_err(|e| DIDError::KeyRetrievalFailed(format!("Failed to retrieve key: {}", e)))?;
        
        let key_data: serde_json::Value = serde_json::from_str(&key_json)
            .map_err(|e| DIDError::KeyRetrievalFailed(format!("Failed to parse key data: {}", e)))?;
        
        let key_type: KeyType = serde_json::from_value(key_data["key_type"].clone())
            .map_err(|e| DIDError::KeyRetrievalFailed(format!("Invalid key type: {}", e)))?;
        
        let public_key = hex::decode(
            key_data["public_key"]
                .as_str()
                .ok_or_else(|| DIDError::KeyRetrievalFailed("Missing public key".to_string()))?,
        )
        .map_err(|e| DIDError::KeyRetrievalFailed(format!("Invalid public key hex: {}", e)))?;
        
        let private_key = Zeroizing::new(
            hex::decode(
                key_data["private_key"]
                    .as_str()
                    .ok_or_else(|| DIDError::KeyRetrievalFailed("Missing private key".to_string()))?,
            )
            .map_err(|e| DIDError::KeyRetrievalFailed(format!("Invalid private key hex: {}", e)))?,
        );
        
        Ok(KeyPair {
            key_type,
            public_key,
            private_key,
        })
    }
    
    /// Delete a key from the platform keyring
    pub fn delete_key(&self, key_id: &str) -> Result<()> {
        let entry = keyring::Entry::new(&self.service, key_id)
            .map_err(|e| DIDError::KeyStorageFailed(format!("Failed to create keyring entry: {}", e)))?;
        
        entry
            .delete_password()
            .map_err(|e| DIDError::KeyStorageFailed(format!("Failed to delete key: {}", e)))?;
        
        Ok(())
    }
    
    /// Check if a key exists in storage
    pub fn key_exists(&self, key_id: &str) -> bool {
        let entry = match keyring::Entry::new(&self.service, key_id) {
            Ok(e) => e,
            Err(_) => return false,
        };
        
        entry.get_password().is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_generate_ed25519() {
        let keypair = KeyPair::generate_ed25519().unwrap();
        assert_eq!(keypair.key_type, KeyType::Ed25519);
        assert_eq!(keypair.public_key.len(), 32);
    }
    
    #[test]
    fn test_generate_x25519() {
        let keypair = KeyPair::generate_x25519().unwrap();
        assert_eq!(keypair.key_type, KeyType::X25519);
        assert_eq!(keypair.public_key.len(), 32);
    }
    
    #[test]
    fn test_sign_and_verify() {
        let keypair = KeyPair::generate_ed25519().unwrap();
        let data = b"test message";
        
        let signature = keypair.sign(data).unwrap();
        assert_eq!(signature.len(), 64);
        
        let valid = keypair.verify(data, &signature).unwrap();
        assert!(valid);
        
        let invalid = keypair.verify(b"wrong message", &signature).unwrap();
        assert!(!invalid);
    }
}
