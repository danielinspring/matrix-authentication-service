// Copyright 2024, 2025 New Vector Ltd.
//
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Element-Commercial
// Please see LICENSE files in the repository root for full details.

//! Device key management with DID verification methods
//!
//! This module provides integration between Matrix device keys and DID
//! verification methods, enabling device authentication using DIDs.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Device keys with DID integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceKeys {
    /// The user's Matrix ID
    pub user_id: String,
    
    /// The device ID
    pub device_id: String,
    
    /// Device display name
    pub display_name: Option<String>,
    
    /// The DID associated with this device
    pub did: Option<String>,
    
    /// Ed25519 signing key
    pub ed25519_key: String,
    
    /// Curve25519 identity key
    pub curve25519_key: String,
    
    /// Additional device keys
    pub keys: HashMap<String, String>,
    
    /// Signatures on the device keys
    pub signatures: HashMap<String, HashMap<String, String>>,
}

impl DeviceKeys {
    /// Create new device keys
    pub fn new(
        user_id: String,
        device_id: String,
        ed25519_key: String,
        curve25519_key: String,
    ) -> Self {
        let mut keys = HashMap::new();
        keys.insert(
            format!("ed25519:{}", device_id),
            ed25519_key.clone(),
        );
        keys.insert(
            format!("curve25519:{}", device_id),
            curve25519_key.clone(),
        );
        
        DeviceKeys {
            user_id,
            device_id,
            display_name: None,
            did: None,
            ed25519_key,
            curve25519_key,
            keys,
            signatures: HashMap::new(),
        }
    }
    
    /// Associate a DID with this device
    pub fn with_did(mut self, did: String) -> Self {
        self.did = Some(did);
        self
    }
    
    /// Set the display name
    pub fn with_display_name(mut self, name: String) -> Self {
        self.display_name = Some(name);
        self
    }
    
    /// Add a signature
    pub fn add_signature(&mut self, signer_id: String, key_id: String, signature: String) {
        self.signatures
            .entry(signer_id)
            .or_insert_with(HashMap::new)
            .insert(key_id, signature);
    }
    
    /// Get the canonical JSON for signing
    pub fn canonical_json(&self) -> String {
        // In a real implementation, this would produce properly canonicalized JSON
        serde_json::to_string(self).unwrap_or_default()
    }
}

/// Cross-signing keys with DID integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossSigningKeys {
    /// Master key
    pub master_key: DeviceKeys,
    
    /// Self-signing key
    pub self_signing_key: DeviceKeys,
    
    /// User-signing key
    pub user_signing_key: DeviceKeys,
    
    /// Associated DID
    pub did: Option<String>,
}

/// Device verification status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerificationStatus {
    /// Not verified
    Unverified,
    /// Verified via cross-signing
    CrossSigned,
    /// Verified via DID
    DIDVerified,
    /// Verified via both cross-signing and DID
    FullyVerified,
}

/// Device manager with DID support
pub struct DeviceManager {
    /// Devices indexed by user_id and device_id
    devices: HashMap<String, HashMap<String, DeviceKeys>>,
}

impl DeviceManager {
    /// Create a new device manager
    pub fn new() -> Self {
        DeviceManager {
            devices: HashMap::new(),
        }
    }
    
    /// Register a device
    pub fn register_device(&mut self, device: DeviceKeys) {
        self.devices
            .entry(device.user_id.clone())
            .or_insert_with(HashMap::new)
            .insert(device.device_id.clone(), device);
    }
    
    /// Get a device
    pub fn get_device(&self, user_id: &str, device_id: &str) -> Option<&DeviceKeys> {
        self.devices.get(user_id)?.get(device_id)
    }
    
    /// Get all devices for a user
    pub fn get_user_devices(&self, user_id: &str) -> Option<&HashMap<String, DeviceKeys>> {
        self.devices.get(user_id)
    }
    
    /// Remove a device
    pub fn remove_device(&mut self, user_id: &str, device_id: &str) -> Option<DeviceKeys> {
        self.devices.get_mut(user_id)?.remove(device_id)
    }
    
    /// Get devices by DID
    pub fn get_devices_by_did(&self, did: &str) -> Vec<&DeviceKeys> {
        self.devices
            .values()
            .flat_map(|user_devices| user_devices.values())
            .filter(|device| device.did.as_deref() == Some(did))
            .collect()
    }
}

impl Default for DeviceManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_device_keys() {
        let device = DeviceKeys::new(
            "@alice:example.com".to_string(),
            "DEVICE1".to_string(),
            "ed25519_key".to_string(),
            "curve25519_key".to_string(),
        )
        .with_did("did:peer:123".to_string())
        .with_display_name("Alice's Phone".to_string());
        
        assert_eq!(device.did, Some("did:peer:123".to_string()));
        assert_eq!(device.display_name, Some("Alice's Phone".to_string()));
        assert_eq!(device.keys.len(), 2);
    }
    
    #[test]
    fn test_device_manager() {
        let mut manager = DeviceManager::new();
        
        let device = DeviceKeys::new(
            "@alice:example.com".to_string(),
            "DEVICE1".to_string(),
            "ed25519_key".to_string(),
            "curve25519_key".to_string(),
        )
        .with_did("did:peer:123".to_string());
        
        manager.register_device(device);
        
        let retrieved = manager.get_device("@alice:example.com", "DEVICE1");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().did, Some("did:peer:123".to_string()));
        
        let by_did = manager.get_devices_by_did("did:peer:123");
        assert_eq!(by_did.len(), 1);
    }
}
