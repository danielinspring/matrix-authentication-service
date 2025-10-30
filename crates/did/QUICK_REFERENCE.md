# DID Quick Reference Guide

Quick reference for common DID operations in the Matrix Authentication Service.

## Table of Contents

- [Setup](#setup)
- [DID Operations](#did-operations)
- [Key Management](#key-management)
- [Authentication](#authentication)
- [Credentials](#credentials)
- [Device Management](#device-management)
- [Error Handling](#error-handling)

## Setup

```rust
use mas_did::{DIDManager, DID, DIDDocument};
use mas_matrix::{DIDAuthService, DeviceManager};

// Initialize DID manager
let did_manager = DIDManager::new("matrix-auth-service");

// Initialize auth service
let auth_service = DIDAuthService::new();

// Initialize device manager
let device_manager = DeviceManager::new();
```

## DID Operations

### Create a did:peer DID

```rust
let (did, doc) = did_manager.create_peer_did().await?;
// did: DID { method: "peer", ... }
// doc: DIDDocument with verification methods
```

### Create a did:key DID

```rust
let (did, doc) = did_manager.create_key_did().await?;
// Ephemeral DID for temporary use
```

### Parse a DID string

```rust
let did = DID::parse("did:peer:2.Ez6LS...")?;
println!("Method: {}", did.method);
println!("ID: {}", did.method_specific_id);
```

### Resolve a DID

```rust
let doc = did_manager.resolve(&did).await?;

// Access verification methods
if let Some(methods) = &doc.verification_method {
    for method in methods {
        println!("Key: {}", method.id);
    }
}
```

### Add service endpoint

```rust
let doc = did_manager.add_service_endpoint(
    doc,
    "DIDCommMessaging",
    "https://example.com/didcomm"
);
```

### Delete a DID

```rust
did_manager.delete_did(&did)?;
```

## Key Management

### Generate keys

```rust
use mas_did::{KeyPair, KeyType};

// Ed25519 signing key
let signing_key = KeyPair::generate_ed25519()?;

// X25519 encryption key
let encryption_key = KeyPair::generate_x25519()?;
```

### Sign data

```rust
let data = b"Message to sign";
let signature = did_manager.sign(&did, data)?;
```

### Verify signature

```rust
let valid = did_manager.verify(&did, data, &signature)?;
if valid {
    println!("Signature valid!");
}
```

### Store key

```rust
use mas_did::KeyManager;

let key_manager = KeyManager::new("my-service");
let keypair = KeyPair::generate_ed25519()?;

key_manager.store_key("my-key-id", &keypair)?;
```

### Retrieve key

```rust
let keypair = key_manager.retrieve_key("my-key-id")?;
```

### Check if key exists

```rust
if key_manager.key_exists("my-key-id") {
    println!("Key found!");
}
```

## Authentication

### Create authentication challenge

```rust
let challenge = auth_service
    .create_challenge(did.id.clone())
    .await?;

// Send to client
let message = challenge.challenge_message();
// "Matrix DID Authentication\nDID: ...\nNonce: ...\nTimestamp: ..."
```

### Verify challenge response

```rust
let signature = receive_from_client().await?;

let valid = auth_service
    .verify_challenge(&did.id, &signature)
    .await?;

if valid {
    println!("Authentication successful!");
}
```

### Complete authentication

```rust
auth_service
    .complete_authentication(did.id, "alice".to_string())
    .await?;
```

### Check DID mapping

```rust
// Get Matrix localpart from DID
let localpart = auth_service
    .mapping()
    .get_localpart(&did.id)
    .await;

// Get DID from localpart
let did_str = auth_service
    .mapping()
    .get_did("alice")
    .await;
```

## Credentials

### Create a credential

```rust
use mas_did::{Credential, Issuer, CredentialSubject, VerifiableCredential};

// Matrix user credential
let vc = Credential::matrix_user(
    issuer_did.id.clone(),
    user_did.id.clone(),
    "@alice:example.com".to_string(),
    Some("Alice".to_string()),
);

// Device credential
let vc = Credential::device(
    issuer_did.id.clone(),
    device_did.id.clone(),
    "DEVICE123".to_string(),
    serde_json::json!({
        "ed25519": "key...",
        "curve25519": "key..."
    }),
);

// Custom credential
let subject = CredentialSubject::new(
    user_did.id.clone(),
    serde_json::json!({
        "role": "admin",
        "permissions": ["read", "write"]
    }),
);

let vc = VerifiableCredential::new(
    Issuer::DID(issuer_did.id.clone()),
    subject,
    vec!["CustomCredential".to_string()],
);
```

### Add credential properties

```rust
let vc = vc
    .with_id("https://example.com/credentials/123".to_string())
    .with_expiration(chrono::Utc::now() + chrono::Duration::days(30));
```

### Sign credential

```rust
let signed_vc = vc.sign(&did_manager, &issuer_did).await?;
```

### Verify credential

```rust
let valid = signed_vc.verify(&did_manager).await?;

if !valid || signed_vc.is_expired() {
    return Err("Invalid or expired credential");
}
```

### Access credential claims

```rust
let claims = &signed_vc.credential_subject.claims;
let role = claims["role"].as_str().unwrap_or("user");
```

## Device Management

### Register device

```rust
use mas_matrix::{DeviceKeys, VerificationStatus};

let device = DeviceKeys::new(
    "@alice:example.com".to_string(),
    "DEVICE1".to_string(),
    "ed25519_key_base64".to_string(),
    "curve25519_key_base64".to_string(),
)
.with_did(user_did.id.clone())
.with_display_name("Alice's Phone".to_string());

device_manager.register_device(device);
```

### Get device

```rust
let device = device_manager
    .get_device("@alice:example.com", "DEVICE1");

if let Some(device) = device {
    println!("Device DID: {:?}", device.did);
    println!("Display name: {:?}", device.display_name);
}
```

### Get all user devices

```rust
if let Some(devices) = device_manager.get_user_devices("@alice:example.com") {
    for (device_id, device) in devices {
        println!("{}: {}", device_id, device.display_name.as_deref().unwrap_or(""));
    }
}
```

### Get devices by DID

```rust
let devices = device_manager.get_devices_by_did(&user_did.id);
println!("User has {} devices", devices.len());
```

### Remove device

```rust
let removed = device_manager
    .remove_device("@alice:example.com", "DEVICE1");

if removed.is_some() {
    println!("Device removed");
}
```

### Add device signature

```rust
let mut device = device.clone();
device.add_signature(
    "@alice:example.com".to_string(),
    "ed25519:DEVICE1".to_string(),
    "signature_base64".to_string(),
);
```

## Error Handling

### Result types

```rust
use mas_did::{Result, DIDError};

fn my_function() -> Result<DID> {
    let did = DID::parse("did:peer:123")?;
    Ok(did)
}
```

### Error types

```rust
match result {
    Ok(value) => println!("Success: {:?}", value),
    Err(DIDError::InvalidDIDFormat(msg)) => {
        eprintln!("Invalid DID: {}", msg);
    }
    Err(DIDError::KeyStorageFailed(msg)) => {
        eprintln!("Key storage error: {}", msg);
    }
    Err(DIDError::ResolutionFailed(msg)) => {
        eprintln!("Resolution failed: {}", msg);
    }
    Err(DIDError::VerificationFailed(msg)) => {
        eprintln!("Verification failed: {}", msg);
    }
    Err(e) => eprintln!("Other error: {}", e),
}
```

### Common error patterns

```rust
// Handling missing keys
let keypair = match key_manager.retrieve_key("key-id") {
    Ok(key) => key,
    Err(DIDError::KeyRetrievalFailed(_)) => {
        // Generate new key if not found
        let new_key = KeyPair::generate_ed25519()?;
        key_manager.store_key("key-id", &new_key)?;
        new_key
    }
    Err(e) => return Err(e),
};

// Handling expired challenges
let valid = match auth_service.verify_challenge(&did.id, &signature).await {
    Ok(true) => true,
    Ok(false) => {
        // Challenge expired or invalid
        return Err("Authentication failed");
    }
    Err(e) => return Err(e.into()),
};
```

## Common Patterns

### Complete user registration flow

```rust
async fn register_user(username: &str) -> Result<(String, DID)> {
    // Create DID
    let (did, doc) = did_manager.create_peer_did().await?;
    
    // Store in database
    store_user_did(username, &did, &doc).await?;
    
    // Create mapping
    auth_service.mapping()
        .associate(did.id.clone(), username.to_string())
        .await?;
    
    Ok((username.to_string(), did))
}
```

### Complete authentication flow

```rust
async fn authenticate_user(did: &DID) -> Result<String> {
    // Create challenge
    let challenge = auth_service
        .create_challenge(did.id.clone())
        .await?;
    
    // Send to client and get signature
    let signature = get_user_signature(&challenge).await?;
    
    // Verify
    let valid = auth_service
        .verify_challenge(&did.id, &signature)
        .await?;
    
    if !valid {
        return Err(DIDError::VerificationFailed("Invalid signature".into()));
    }
    
    // Get Matrix ID
    let localpart = auth_service
        .mapping()
        .get_localpart(&did.id)
        .await
        .ok_or("No mapping found")?;
    
    Ok(localpart)
}
```

### Issue and verify credential

```rust
async fn issue_user_credential(
    issuer_did: &DID,
    user_did: &DID,
    matrix_id: &str,
) -> Result<VerifiableCredential> {
    // Create credential
    let vc = Credential::matrix_user(
        issuer_did.id.clone(),
        user_did.id.clone(),
        matrix_id.to_string(),
        None,
    )
    .with_expiration(chrono::Utc::now() + chrono::Duration::days(365));
    
    // Sign
    let signed_vc = vc.sign(&did_manager, issuer_did).await?;
    
    // Store
    store_credential(&signed_vc).await?;
    
    Ok(signed_vc)
}

async fn verify_user_credential(vc: &VerifiableCredential) -> Result<bool> {
    // Check expiration
    if vc.is_expired() {
        return Ok(false);
    }
    
    // Verify signature
    let valid = vc.verify(&did_manager).await?;
    
    Ok(valid)
}
```

## Testing Helpers

### Test setup

```rust
#[tokio::test]
async fn test_my_feature() {
    let did_manager = DIDManager::new("test-service");
    let (did, _) = did_manager.create_peer_did().await.unwrap();
    
    // Your test code
    
    // Cleanup
    did_manager.delete_did(&did).unwrap();
}
```

### Mock authentication

```rust
async fn mock_auth_flow() -> (DID, String) {
    let did_manager = DIDManager::new("test");
    let auth_service = DIDAuthService::new();
    
    let (did, _) = did_manager.create_peer_did().await.unwrap();
    auth_service
        .complete_authentication(did.id.clone(), "testuser".to_string())
        .await
        .unwrap();
    
    (did, "testuser".to_string())
}
```

## Performance Tips

1. **Batch operations** when creating multiple DIDs
2. **Cache resolved DID documents** to avoid repeated resolution
3. **Use did:peer for users** (no network calls)
4. **Use did:web for servers** (cacheable)
5. **Reuse KeyManager instances** to avoid repeated keyring access

## Security Checklist

- [ ] Store keys in platform keyring (never in plaintext)
- [ ] Use time-bound challenges with expiration
- [ ] Validate all DID formats before use
- [ ] Check credential expiration before trusting
- [ ] Verify signatures on all credentials
- [ ] Use HTTPS for did:web resolution
- [ ] Implement rate limiting on authentication attempts
- [ ] Log all DID operations for audit
- [ ] Rotate keys periodically
- [ ] Clear sensitive data from memory after use

## Next Steps

- Read the full [README.md](./README.md) for detailed documentation
- Check [INTEGRATION.md](./INTEGRATION.md) for production deployment
- Review [DID_IMPLEMENTATION_SUMMARY.md](../../DID_IMPLEMENTATION_SUMMARY.md) for architecture
