# DID Implementation Architecture

## High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Matrix Authentication Service             │
│                                                              │
│  ┌────────────────────────────────────────────────────┐    │
│  │              User Registration & Login              │    │
│  └─────────────────────┬──────────────────────────────┘    │
│                        │                                     │
│  ┌─────────────────────▼──────────────────────────────┐    │
│  │         DID Authentication Service                  │    │
│  │  - Challenge-response auth                          │    │
│  │  - DID ↔ Matrix ID mapping                         │    │
│  │  - Session management                               │    │
│  └─────────────────────┬──────────────────────────────┘    │
│                        │                                     │
└────────────────────────┼─────────────────────────────────────┘
                         │
        ┌────────────────┴────────────────┐
        │                                  │
┌───────▼──────────┐            ┌─────────▼──────────┐
│   mas-matrix     │            │     mas-did         │
│                  │            │                     │
│ - DIDAuthService │            │ - DIDManager        │
│ - DIDMapping     │◄───────────┤ - DID Resolution    │
│ - DeviceManager  │            │ - Key Management    │
│ - DeviceKeys     │            │ - VC Support        │
└──────────────────┘            └──────┬──────────────┘
                                       │
                    ┌──────────────────┼──────────────────┐
                    │                  │                   │
            ┌───────▼──────┐  ┌────────▼────────┐  ┌──────▼──────┐
            │  Resolvers   │  │  Key Manager    │  │ Credentials │
            │              │  │                 │  │             │
            │ - PeerDID    │  │ - KeyPair Gen   │  │ - Issue     │
            │ - WebDID     │  │ - Platform      │  │ - Verify    │
            │ - KeyDID     │  │   Keyring       │  │ - Types     │
            └──────────────┘  └─────────────────┘  └─────────────┘
```

## Component Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                        mas-did Crate                         │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌────────────────────────────────────────────────────┐    │
│  │              DIDManager (High-Level API)            │    │
│  │                                                      │    │
│  │  + create_peer_did() -> (DID, DIDDocument)         │    │
│  │  + create_key_did() -> (DID, DIDDocument)          │    │
│  │  + resolve(DID) -> DIDDocument                      │    │
│  │  + sign(DID, data) -> signature                     │    │
│  │  + verify(DID, data, sig) -> bool                   │    │
│  └────────┬────────────────────────────┬────────────────┘    │
│           │                            │                      │
│  ┌────────▼───────────┐       ┌───────▼──────────┐          │
│  │  ResolverRegistry  │       │   KeyManager     │          │
│  │                    │       │                  │          │
│  │  - PeerResolver    │       │  - generate()    │          │
│  │  - WebResolver     │       │  - store()       │          │
│  │  - KeyResolver     │       │  - retrieve()    │          │
│  └────────────────────┘       │  - delete()      │          │
│                                └──────┬───────────┘          │
│  ┌──────────────────────────┐        │                      │
│  │  DIDDocument             │        │                      │
│  │                          │        │                      │
│  │  - id: String            │   ┌────▼──────────────┐      │
│  │  - verificationMethod[]  │   │  Platform Keyring │      │
│  │  - authentication[]      │   │                   │      │
│  │  - keyAgreement[]        │   │  macOS: Keychain  │      │
│  │  - service[]             │   │  Win: CredMgr     │      │
│  └──────────────────────────┘   │  Linux: SecretSvc │      │
│                                  └───────────────────┘      │
│  ┌──────────────────────────────────────────────────┐      │
│  │  VerifiableCredential                             │      │
│  │                                                    │      │
│  │  - issuer: DID                                    │      │
│  │  - subject: CredentialSubject                     │      │
│  │  - issuanceDate                                   │      │
│  │  - expirationDate                                 │      │
│  │  - proof: Signature                               │      │
│  │                                                    │      │
│  │  + sign(DIDManager, DID) -> VC                   │      │
│  │  + verify(DIDManager) -> bool                     │      │
│  └──────────────────────────────────────────────────┘      │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

## Sequence Diagrams

### User Registration with DID

```
Client          Service         DIDManager      KeyManager      Keyring
  │                │                 │               │             │
  │─Register────►  │                 │               │             │
  │   request      │                 │               │             │
  │                │                 │               │             │
  │                │──create_peer──► │               │             │
  │                │      _did()     │               │             │
  │                │                 │               │             │
  │                │                 │──generate────►│             │
  │                │                 │  Ed25519()    │             │
  │                │                 │               │             │
  │                │                 │──generate────►│             │
  │                │                 │  X25519()     │             │
  │                │                 │               │             │
  │                │                 │──store_key──►│──store────►│
  │                │                 │               │             │
  │                │◄────(DID, Doc)──┤               │             │
  │                │                 │               │             │
  │                │──store_in_db───►│               │             │
  │                │   (user, DID)   │               │             │
  │                │                 │               │             │
  │◄───Response────│                 │               │             │
  │  (DID created) │                 │               │             │
```

### Authentication Flow

```
Client          AuthService     DIDManager      KeyManager
  │                │                 │               │
  │──Auth with───► │                 │               │
  │    DID         │                 │               │
  │                │                 │               │
  │                │──create────────►│               │
  │                │  Challenge()    │               │
  │                │                 │               │
  │◄───Challenge───┤                 │               │
  │   (nonce)      │                 │               │
  │                │                 │               │
  │──Sign with────►│                 │               │
  │   private key  │                 │               │
  │   (signature)  │                 │               │
  │                │                 │               │
  │                │──verify────────►│──retrieve───►│
  │                │  Challenge()    │   _key()     │
  │                │                 │               │
  │                │                 │──verify()────►│
  │                │                 │               │
  │                │◄────valid───────┤               │
  │                │                 │               │
  │                │──complete──────►│               │
  │                │  Authentication()│               │
  │                │                 │               │
  │◄───Success─────┤                 │               │
  │   (session)    │                 │               │
```

### Credential Issuance and Verification

```
Issuer          DIDManager      Subject         Verifier
  │                │                │               │
  │──create_VC────►│                │               │
  │                │                │               │
  │──sign(VC)─────►│                │               │
  │                │──get_signing──►│               │
  │                │     _key()     │               │
  │                │                │               │
  │◄───signed_VC───┤                │               │
  │                │                │               │
  │──────send_to_subject───────────►│               │
  │          (VC)                   │               │
  │                │                │               │
  │                │                │──present_VC──►│
  │                │                │               │
  │                │                │               │──verify────►
  │                │                │               │    (VC)
  │                │◄───────────────┼───────────────│
  │                │   resolve_issuer_DID           │
  │                │                │               │
  │                │──────────────────────────────► │
  │                │     get_verification_key       │
  │                │                │               │
  │                │                │◄──────────────│
  │                │                │   valid=true  │
```

### Device Registration with DID

```
User            DeviceManager   DIDManager      KeyManager
  │                │                 │               │
  │──New Device───►│                 │               │
  │                │                 │               │
  │                │──generate──────►│               │
  │                │   device keys   │               │
  │                │                 │               │
  │                │                 │──generate────►│
  │                │                 │  Ed25519()    │
  │                │                 │               │
  │                │                 │──generate────►│
  │                │                 │  Curve25519() │
  │                │                 │               │
  │                │◄────keys────────┤               │
  │                │                 │               │
  │                │──associate_with─┤               │
  │                │     user_DID    │               │
  │                │                 │               │
  │                │──register──────►│               │
  │                │   Device()      │               │
  │                │                 │               │
  │◄───Device ID───┤                 │               │
  │   & Keys       │                 │               │
```

## Data Flow

### DID Creation Flow

```
1. Request: create_peer_did()
   └─► Generate Ed25519 signing key
       └─► Store in platform keyring as "{did}-signing"
   └─► Generate X25519 encryption key
       └─► Store in platform keyring as "{did}-encryption"
   └─► Construct did:peer:2 identifier
       └─► Format: did:peer:2.V{signing}.E{encryption}
   └─► Build DID Document
       └─► Add verification methods
       └─► Add authentication relationships
       └─► Add key agreement relationships
   └─► Return (DID, DIDDocument)

2. Storage: Platform Keyring
   ├─► macOS/iOS: Keychain
   │   └─► Hardware: Secure Enclave (when available)
   ├─► Windows: Credential Manager
   │   └─► Encryption: DPAPI
   └─► Linux: Secret Service
       └─► Backend: gnome-keyring, kwallet, etc.
```

### DID Resolution Flow

```
1. Input: DID string
   └─► Parse DID
       └─► Extract method (peer, web, key)
       └─► Extract method-specific-id

2. Route to Resolver
   ├─► did:peer
   │   └─► Parse numalgo 2 format
   │   └─► Extract embedded keys
   │   └─► Build DID Document locally
   │   └─► Return document (no network)
   │
   ├─► did:web
   │   └─► Convert to HTTPS URL
   │   └─► Fetch /.well-known/did.json
   │   └─► Validate response
   │   └─► Return document
   │
   └─► did:key
       └─► Extract public key
       └─► Build DID Document from key
       └─► Return document (no network)

3. Output: DIDDocument
   └─► Verification methods
   └─► Relationships
   └─► Service endpoints
```

### Cryptographic Operations Flow

```
1. Signing
   └─► Input: DID + data
   └─► Retrieve signing key from keyring
   └─► Sign with Ed25519
       └─► Deterministic signature
       └─► 64 bytes output
   └─► Return signature

2. Verification
   └─► Input: DID + data + signature
   └─► Retrieve verification key
       ├─► From keyring (if local DID)
       └─► From resolved DID Document (if remote)
   └─► Verify with Ed25519
   └─► Return boolean

3. Key Agreement (Future)
   └─► Input: My DID + Their DID
   └─► Retrieve my X25519 secret
   └─► Retrieve their X25519 public
   └─► Compute shared secret
   └─► Derive encryption keys
   └─► Return session keys
```

## Security Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Security Layers                           │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  Layer 5: Application Security                              │
│  ┌──────────────────────────────────────────────────┐      │
│  │ - Rate limiting                                   │      │
│  │ - Access control                                  │      │
│  │ - Audit logging                                   │      │
│  └──────────────────────────────────────────────────┘      │
│                                                              │
│  Layer 4: Cryptographic Verification                        │
│  ┌──────────────────────────────────────────────────┐      │
│  │ - Signature verification                          │      │
│  │ - Challenge-response authentication               │      │
│  │ - Credential validation                           │      │
│  └──────────────────────────────────────────────────┘      │
│                                                              │
│  Layer 3: Memory Safety                                     │
│  ┌──────────────────────────────────────────────────┐      │
│  │ - Rust ownership model                            │      │
│  │ - Automatic zeroization                           │      │
│  │ - No unsafe code                                  │      │
│  └──────────────────────────────────────────────────┘      │
│                                                              │
│  Layer 2: Key Storage                                       │
│  ┌──────────────────────────────────────────────────┐      │
│  │ - Platform keyring                                │      │
│  │ - Encrypted at rest                               │      │
│  │ - OS access control                               │      │
│  └──────────────────────────────────────────────────┘      │
│                                                              │
│  Layer 1: Hardware Security (when available)                │
│  ┌──────────────────────────────────────────────────┐      │
│  │ - Secure Enclave (Apple)                          │      │
│  │ - TPM (Windows/Linux)                             │      │
│  │ - Hardware key isolation                          │      │
│  └──────────────────────────────────────────────────┘      │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

## Module Dependencies

```
mas-did
├── types.rs
│   └── DID, DIDUrl
│
├── error.rs
│   └── DIDError, Result
│
├── key_manager.rs
│   ├── depends on: types, error
│   ├── external: ed25519-dalek, x25519-dalek, keyring
│   └── provides: KeyPair, KeyManager, KeyType
│
├── did_document.rs
│   ├── depends on: types
│   └── provides: DIDDocument, VerificationMethod, Service
│
├── resolver.rs
│   ├── depends on: types, error, did_document
│   ├── external: reqwest, async-trait
│   └── provides: DIDResolver, ResolverRegistry, *Resolver
│
├── did_manager.rs
│   ├── depends on: types, error, key_manager, did_document, resolver
│   └── provides: DIDManager
│
└── verifiable_credential.rs
    ├── depends on: types, error, did_manager
    ├── external: chrono
    └── provides: VerifiableCredential, Credential, Issuer, Proof

mas-matrix
├── did_auth.rs
│   ├── depends on: mas-did::DID
│   ├── external: tokio, chrono, rand
│   └── provides: DIDAuthService, DIDMapping, AuthChallenge
│
└── device_keys.rs
    ├── depends on: mas-did::DID
    └── provides: DeviceKeys, DeviceManager, VerificationStatus
```

## Performance Characteristics

```
Operation                    Complexity    Time        Notes
─────────────────────────────────────────────────────────────
DID Creation                 O(1)          ~10ms       Key generation
DID Resolution (peer)        O(1)          <1ms        Local parsing
DID Resolution (web)         O(1)          ~100ms      Network dependent
DID Resolution (key)         O(1)          <1ms        Local generation
Sign Operation               O(1)          ~0.5ms      Ed25519
Verify Operation             O(1)          ~1ms        Ed25519
Key Storage                  O(1)          ~10ms       OS dependent
Key Retrieval                O(1)          ~5ms        OS dependent
Credential Creation          O(1)          ~1ms        Structure only
Credential Signing           O(1)          ~2ms        + serialization
Credential Verification      O(1)          ~3ms        + resolution
Challenge Creation           O(1)          <1ms        Random generation
Challenge Verification       O(1)          ~1ms        + key lookup
Device Registration          O(1)          <1ms        In-memory
Device Lookup by DID         O(n)          <1ms        n = devices
```

## Extensibility Points

1. **DID Methods**: Add new resolvers by implementing `DIDResolver` trait
2. **Key Storage**: Replace keyring with HSM, cloud KMS, or TEE
3. **Credential Types**: Create custom credential schemas
4. **Authentication**: Add additional authentication mechanisms
5. **Proof Types**: Support multiple signature schemes
6. **Service Endpoints**: Add protocol-specific service definitions

## Future Enhancements

```
Phase 3 (Future Work)
├── HSM Integration
│   ├── PKCS#11 support
│   ├── AWS KMS integration
│   └── Azure Key Vault integration
│
├── TEE Support
│   ├── Intel SGX
│   ├── ARM TrustZone
│   └── AMD SEV
│
├── Advanced Features
│   ├── Social recovery (Shamir Secret Sharing)
│   ├── Key rotation automation
│   ├── Credential revocation lists
│   └── Zero-knowledge proofs
│
└── E2EE Enhancement
    ├── MLS integration
    ├── Perfect forward secrecy
    └── Post-quantum cryptography
```

## References

- **W3C Standards**: DID Core, Verifiable Credentials
- **DID Methods**: did:peer, did:web, did:key specifications
- **Cryptography**: Ed25519, X25519 (Curve25519)
- **Libraries**: RustCrypto, tokio, serde
- **Matrix**: Matrix Protocol Specification
