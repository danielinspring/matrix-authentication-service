# mas-did: DID and Verifiable Credentials for Matrix

This crate provides comprehensive DID (Decentralized Identifier) and Verifiable Credentials support for the Matrix Authentication Service, enabling decentralized identity management and cryptographic verification.

## Features

- **DID Methods**: Support for `did:peer`, `did:web`, and `did:key` methods
- **Key Management**: Secure key storage using platform-native keyrings
- **DID Resolution**: Resolve DIDs to DID Documents
- **Verifiable Credentials**: W3C-compliant credential creation and verification
- **Matrix Integration**: DID-based authentication and device key management

## Architecture

### DID Support

The implementation follows the W3C DID Core specification with support for:

- **did:peer:2** - Peer-to-peer DIDs optimized for messaging (primary method)
- **did:web** - Web-based DIDs for organizational identities  
- **did:key** - Simple key-based DIDs for ephemeral use

### Cryptographic Stack

Based on the **RustCrypto** ecosystem for production-ready security:

- `ed25519-dalek` - Ed25519 signing keys
- `x25519-dalek` - X25519 key agreement (encryption)
- `chacha20poly1305` - AEAD encryption
- `sha2` - SHA-256 hashing
- `keyring` - Platform-native secure key storage

### Key Management

Keys are stored securely using platform-specific mechanisms:

- **macOS/iOS**: Keychain with Secure Enclave backing
- **Windows**: Credential Manager with DPAPI encryption
- **Linux**: Secret Service API or keyutils

## Usage

### Creating a DID

```rust
use mas_did::DIDManager;

let manager = DIDManager::new("my-app");

// Create a did:peer:2 with signing and encryption keys
let (did, doc) = manager.create_peer_did().await?;
println!("Created DID: {}", did);
```

### Resolving a DID

```rust
let doc = manager.resolve(&did).await?;
println!("DID Document: {:#?}", doc);
```

### Signing and Verifying

```rust
let data = b"message to sign";
let signature = manager.sign(&did, data)?;

let valid = manager.verify(&did, data, &signature)?;
assert!(valid);
```

### Creating Verifiable Credentials

```rust
use mas_did::{Credential, Issuer, CredentialSubject};

// Create a Matrix user credential
let vc = Credential::matrix_user(
    issuer_did.id.clone(),
    user_did.id.clone(),
    "@alice:example.com".to_string(),
    Some("Alice".to_string()),
);

// Sign the credential
let signed_vc = vc.sign(&did_manager, &issuer_did).await?;

// Verify the credential
let valid = signed_vc.verify(&did_manager).await?;
```

## Matrix Integration

### DID-Based Authentication

```rust
use mas_matrix::{DIDAuthService, DIDMapping};

let auth_service = DIDAuthService::new();

// Create authentication challenge
let challenge = auth_service.create_challenge(did.id.clone()).await?;

// Verify challenge response
let valid = auth_service.verify_challenge(&did.id, &signature).await?;

// Complete authentication and create mapping
auth_service.complete_authentication(did.id, "alice".to_string()).await?;
```

### Device Key Management

```rust
use mas_matrix::{DeviceKeys, DeviceManager};

let mut manager = DeviceManager::new();

let device = DeviceKeys::new(
    "@alice:example.com".to_string(),
    "DEVICE1".to_string(),
    ed25519_key,
    curve25519_key,
)
.with_did(did.id.clone())
.with_display_name("Alice's Phone".to_string());

manager.register_device(device);
```

## Security Considerations

### Key Storage

Keys are stored using platform-native secure storage mechanisms. On macOS/iOS, keys stored in the Keychain benefit from Secure Enclave hardware backing when available.

### Key Rotation

For production deployments, implement key rotation following these guidelines:

- Rotate signing keys every 90-180 days
- Use envelope encryption for scalable rotation
- Maintain N+1 keys during rotation windows

### DID Methods

- **did:peer** - Maximum privacy, zero transaction costs, offline capable
- **did:web** - Better for organizational identities, requires web infrastructure
- **did:key** - Ephemeral use only, no key rotation support

## Testing

Run the test suite:

```bash
cargo test -p mas-did
```

Run specific tests:

```bash
cargo test -p mas-did test_create_peer_did
cargo test -p mas-did test_sign_and_verify_credential
```

## Implementation Status

### Phase 1: Foundation ✅

- [x] DID types and parsing
- [x] Key generation and management
- [x] Platform-native key storage
- [x] DID document structure
- [x] DID resolvers (peer, web, key)
- [x] DID manager
- [x] Verifiable credentials

### Phase 2: Matrix Integration ✅

- [x] DID-based authentication
- [x] Device key management
- [x] DID to Matrix ID mapping
- [x] Challenge-response authentication

### Phase 3: Advanced Features (Future)

- [ ] HSM integration via PKCS#11
- [ ] Cloud KMS support (AWS KMS, Azure Key Vault)
- [ ] TEE support (Intel SGX, ARM TrustZone)
- [ ] Social recovery with Shamir Secret Sharing
- [ ] Zero-knowledge proofs for selective disclosure
- [ ] MLS integration for E2EE

## References

- [W3C DID Core Specification](https://www.w3.org/TR/did-1.0/)
- [W3C Verifiable Credentials Data Model](https://www.w3.org/TR/vc-data-model/)
- [did:peer Method Specification](https://identity.foundation/peer-did-method-spec/)
- [did:web Method Specification](https://w3c-ccg.github.io/did-method-web/)
- [Matrix Protocol](https://spec.matrix.org/)

## License

AGPL-3.0-only OR LicenseRef-Element-Commercial
