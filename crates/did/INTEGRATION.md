# DID Integration Guide for Matrix Authentication Service

This guide explains how to integrate DID-based authentication and Verifiable Credentials into your Matrix deployment.

## Integration Architecture

The DID implementation provides three main integration points:

1. **DID Manager** - Create and manage decentralized identifiers
2. **Authentication Service** - DID-based authentication with challenge-response
3. **Device Manager** - Link Matrix device keys with DID verification methods

## Quick Start

### 1. Initialize DID Manager

```rust
use mas_did::DIDManager;

// Create DID manager with your service name
let did_manager = DIDManager::new("matrix-auth-service");
```

### 2. User Registration with DID

When a user registers, create a DID for them:

```rust
// Generate a new did:peer:2 for the user
let (user_did, did_doc) = did_manager.create_peer_did().await?;

// Store the DID association
// The DID should be linked to the user's Matrix ID in your database
store_user_did(user_id, &user_did).await?;

tracing::info!(
    "Created DID {} for user {}",
    user_did.id,
    user_id
);
```

### 3. DID-Based Authentication

Implement challenge-response authentication:

```rust
use mas_matrix::DIDAuthService;

let auth_service = DIDAuthService::new();

// Step 1: Create challenge
let challenge = auth_service.create_challenge(user_did.id.clone()).await?;

// Send challenge to client
send_to_client(&challenge.challenge_message());

// Step 2: Verify response
let signature = receive_signature_from_client().await?;
let valid = auth_service.verify_challenge(&user_did.id, &signature).await?;

if valid {
    // Step 3: Complete authentication
    auth_service
        .complete_authentication(user_did.id, localpart)
        .await?;
    
    // Create session
    create_user_session(localpart).await?;
}
```

### 4. Device Registration with DIDs

Link device keys to DID verification methods:

```rust
use mas_matrix::{DeviceKeys, DeviceManager};

let mut device_manager = DeviceManager::new();

// Generate device keys
let signing_key = generate_ed25519_key();
let encryption_key = generate_curve25519_key();

// Create device with DID association
let device = DeviceKeys::new(
    matrix_user_id.to_string(),
    device_id.to_string(),
    signing_key,
    encryption_key,
)
.with_did(user_did.id.clone())
.with_display_name(device_name);

device_manager.register_device(device);
```

## Advanced Integration Patterns

### Credential-Based Authorization

Issue verifiable credentials for roles and permissions:

```rust
use mas_did::{Credential, Issuer, CredentialSubject};

// Create a credential for admin role
let claims = serde_json::json!({
    "role": "admin",
    "server": "example.com",
    "permissions": ["manage_users", "manage_rooms"]
});

let subject = CredentialSubject::new(user_did.id.clone(), claims);

let credential = VerifiableCredential::new(
    Issuer::DID(server_did.id.clone()),
    subject,
    vec!["AdminCredential".to_string()],
)
.with_expiration(chrono::Utc::now() + chrono::Duration::days(30));

// Sign with server's DID
let signed_credential = credential.sign(&did_manager, &server_did).await?;

// Store credential
store_user_credential(user_id, &signed_credential).await?;
```

### Verifying Credentials

Check credentials during authorization:

```rust
// Retrieve user's credential
let credential = get_user_credential(user_id).await?;

// Verify signature
let valid = credential.verify(&did_manager).await?;

if !valid || credential.is_expired() {
    return Err(AuthError::InvalidCredential);
}

// Check claims
let role = credential.credential_subject.claims["role"]
    .as_str()
    .unwrap_or("");

if role == "admin" {
    // Grant admin access
}
```

### Multi-Device Management

Handle multiple devices per user:

```rust
// Get all devices for a user
let devices = device_manager.get_user_devices(matrix_user_id);

// Verify all devices are signed by user's DID
for device in devices.values() {
    if let Some(did) = &device.did {
        // Verify device signature
        verify_device_signature(device, did).await?;
    }
}

// Get devices by DID
let did_devices = device_manager.get_devices_by_did(&user_did.id);
println!("User has {} devices", did_devices.len());
```

## Database Integration

### Storing DID Associations

Recommended database schema:

```sql
-- DID to user mapping
CREATE TABLE user_dids (
    user_id TEXT PRIMARY KEY,
    did TEXT UNIQUE NOT NULL,
    did_document JSONB NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);

-- Device DIDs
CREATE TABLE device_dids (
    device_id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    did TEXT NOT NULL,
    device_keys JSONB NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    FOREIGN KEY (user_id) REFERENCES user_dids(user_id)
);

-- Verifiable credentials
CREATE TABLE user_credentials (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    credential_type TEXT NOT NULL,
    credential JSONB NOT NULL,
    issued_at TIMESTAMP NOT NULL,
    expires_at TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES user_dids(user_id)
);

-- Indexes
CREATE INDEX idx_user_dids_did ON user_dids(did);
CREATE INDEX idx_device_dids_user ON device_dids(user_id);
CREATE INDEX idx_device_dids_did ON device_dids(did);
CREATE INDEX idx_credentials_user ON user_credentials(user_id);
CREATE INDEX idx_credentials_type ON user_credentials(credential_type);
```

### Example Query Functions

```rust
// Store user DID
async fn store_user_did(
    pool: &PgPool,
    user_id: &str,
    did: &DID,
    doc: &DIDDocument,
) -> Result<()> {
    let doc_json = serde_json::to_value(doc)?;
    
    sqlx::query!(
        r#"
        INSERT INTO user_dids (user_id, did, did_document)
        VALUES ($1, $2, $3)
        ON CONFLICT (user_id) DO UPDATE
        SET did = $2, did_document = $3, updated_at = NOW()
        "#,
        user_id,
        did.as_str(),
        doc_json
    )
    .execute(pool)
    .await?;
    
    Ok(())
}

// Retrieve user DID
async fn get_user_did(
    pool: &PgPool,
    user_id: &str,
) -> Result<Option<(DID, DIDDocument)>> {
    let row = sqlx::query!(
        r#"
        SELECT did, did_document
        FROM user_dids
        WHERE user_id = $1
        "#,
        user_id
    )
    .fetch_optional(pool)
    .await?;
    
    if let Some(row) = row {
        let did = DID::parse(&row.did)?;
        let doc = serde_json::from_value(row.did_document)?;
        Ok(Some((did, doc)))
    } else {
        Ok(None)
    }
}
```

## Configuration

### Environment Variables

```bash
# DID service configuration
DID_SERVICE_NAME="matrix-auth-service"
DID_KEY_STORAGE="keyring"  # or "file", "hsm"

# Challenge validity
DID_CHALLENGE_VALIDITY_SECONDS=300

# Key rotation
DID_KEY_ROTATION_DAYS=90
```

### Synapse Integration

For Synapse homeservers, create a password provider:

```python
# synapse_did_provider.py
import requests
from synapse.module_api import ModuleApi

class DIDPasswordProvider:
    def __init__(self, config: dict, api: ModuleApi):
        self.api = api
        self.mas_url = config["mas_url"]
    
    async def check_auth(self, username: str, login_type: str, login_dict: dict):
        if login_type != "m.login.did":
            return None
        
        # Forward to MAS for DID authentication
        response = await self.api.http_client.post_json_get_json(
            f"{self.mas_url}/auth/did/verify",
            {
                "username": username,
                "did": login_dict["did"],
                "signature": login_dict["signature"],
            }
        )
        
        if response.get("valid"):
            return (username, None)
        return None
```

### Matrix Authentication Service Configuration

```yaml
# config.yaml
did:
  enabled: true
  service_name: "matrix-auth-service"
  
  # Supported DID methods
  methods:
    - peer
    - web
    - key
  
  # Key storage backend
  key_storage:
    type: keyring  # keyring, file, or hsm
    
  # Authentication settings
  authentication:
    challenge_validity_seconds: 300
    require_did: false  # Make DID optional initially
  
  # Verifiable credentials
  credentials:
    enabled: true
    issuer_did: "did:web:example.com"
    
  # Key rotation
  key_rotation:
    enabled: true
    rotation_days: 90
    grace_period_days: 7
```

## Testing Integration

### Unit Tests

```rust
#[tokio::test]
async fn test_user_registration_with_did() {
    let did_manager = DIDManager::new("test-service");
    let (did, doc) = did_manager.create_peer_did().await.unwrap();
    
    assert_eq!(did.method, "peer");
    assert!(doc.verification_method.is_some());
    
    // Cleanup
    did_manager.delete_did(&did).unwrap();
}

#[tokio::test]
async fn test_authentication_flow() {
    let auth_service = DIDAuthService::new();
    let did = "did:peer:test123";
    
    let challenge = auth_service
        .create_challenge(did.to_string())
        .await
        .unwrap();
    
    assert!(!challenge.is_expired());
    
    // Simulate signature
    let signature = b"test_signature";
    let valid = auth_service
        .verify_challenge(did, signature)
        .await
        .unwrap();
    
    assert!(valid);
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_full_authentication_flow() {
    // Setup
    let did_manager = DIDManager::new("test-integration");
    let auth_service = DIDAuthService::new();
    
    // Create user DID
    let (user_did, _) = did_manager.create_peer_did().await.unwrap();
    
    // Authentication challenge
    let challenge = auth_service
        .create_challenge(user_did.id.clone())
        .await
        .unwrap();
    
    // Sign challenge
    let message = challenge.challenge_message();
    let signature = did_manager.sign(&user_did, message.as_bytes()).unwrap();
    
    // Verify
    let valid = did_manager
        .verify(&user_did, message.as_bytes(), &signature)
        .unwrap();
    assert!(valid);
    
    // Complete authentication
    auth_service
        .complete_authentication(user_did.id.clone(), "testuser".to_string())
        .await
        .unwrap();
    
    // Verify mapping
    let localpart = auth_service
        .mapping()
        .get_localpart(&user_did.id)
        .await;
    assert_eq!(localpart, Some("testuser".to_string()));
    
    // Cleanup
    did_manager.delete_did(&user_did).unwrap();
}
```

## Monitoring and Observability

### Metrics to Track

```rust
use opentelemetry::metrics::{Counter, Histogram};

struct DIDMetrics {
    dids_created: Counter<u64>,
    authentication_attempts: Counter<u64>,
    authentication_successes: Counter<u64>,
    authentication_failures: Counter<u64>,
    credential_verifications: Counter<u64>,
    authentication_duration: Histogram<f64>,
}
```

### Logging Best Practices

```rust
use tracing::{info, warn, error};

// DID creation
info!(
    did = %user_did.id,
    user_id = %user_id,
    "Created DID for user"
);

// Authentication
info!(
    did = %did,
    "Authentication challenge created"
);

// Failures
warn!(
    did = %did,
    reason = "expired",
    "Authentication challenge failed"
);
```

## Security Considerations

### Production Checklist

- [ ] Enable key rotation with appropriate intervals
- [ ] Use platform-native secure storage
- [ ] Implement rate limiting on authentication attempts
- [ ] Monitor for suspicious DID creation patterns
- [ ] Regularly audit credential usage
- [ ] Set appropriate credential expiration times
- [ ] Implement credential revocation mechanism
- [ ] Use HTTPS for all did:web resolution
- [ ] Validate all DID documents before use
- [ ] Implement proper access controls on DID operations

### Threat Model

1. **Key Compromise**: Platform keyring provides hardware-backed security
2. **Replay Attacks**: Challenges include timestamps and nonces
3. **DID Spoofing**: Cryptographic signatures prevent impersonation
4. **Credential Forgery**: Signatures tie credentials to issuer DIDs
5. **Man-in-the-Middle**: HTTPS required for did:web resolution

## Troubleshooting

### Common Issues

**Issue**: "Key storage failed"
- Check platform keyring is available
- Verify application has necessary permissions
- Try file-based storage as fallback

**Issue**: "DID resolution failed"
- Verify network connectivity for did:web
- Check DID format is correct
- Ensure resolver is registered for method

**Issue**: "Signature verification failed"
- Confirm correct key is being used
- Check message format matches signing
- Verify key hasn't been rotated

## Next Steps

1. **Phase 1**: Deploy basic DID support alongside existing auth
2. **Phase 2**: Enable DID-based authentication for new users
3. **Phase 3**: Migrate existing users to DIDs
4. **Phase 4**: Enable advanced features (credentials, multi-device)

## Support

For questions or issues:
- Review the main README.md
- Check the examples in the repository
- File issues on GitHub
