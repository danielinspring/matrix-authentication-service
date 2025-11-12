# MLS Authentication Service

## Overview

This document describes MAS-DID's role as an MLS (Messaging Layer Security) Authentication Service. The MLS AS cryptographically binds user DIDs to their MLS encryption keys, enabling automatic trust establishment in end-to-end encrypted groups.

## The Problem: MLS Identity Gap

### What is MLS?

**Messaging Layer Security (RFC 9420)** is an IETF standard for end-to-end encrypted group messaging. It provides:
- **Continuous Key Rotation**: Keys change with each message
- **Post-Compromise Security**: Old keys cannot decrypt new messages
- **Forward Secrecy**: New keys cannot decrypt old messages
- **Scalability**: Efficient for large groups (unlike current Matrix E2EE)

### The Identity Problem

MLS provides **encryption** but not **identity verification**. Here's the problem:

```
1. Alice wants to add Bob to encrypted group
2. Bob has MLS KeyPackage with public key K_b
3. Question: How does Alice know K_b belongs to Bob?
```

**Without MLS AS**:
- Alice must manually verify Bob's key fingerprint
- Requires out-of-band communication
- Doesn't scale for groups
- Easy to social engineer

**With MLS AS (MAS-DID)**:
- MAS-DID issues signed certificate binding Bob's DID to K_b
- Alice verifies certificate signature (trusts MAS-DID)
- Alice gains cryptographic proof K_b belongs to Bob
- Automatic, no manual steps required

## MLS AS Architecture

### Core Concept

MAS-DID issues **Verifiable Credentials** that bind a user's **DID** to their **MLS KeyPackage**:

```
┌──────────────────────────────────────────────────────┐
│          MlsKeyPackageCredential VC                  │
│                                                      │
│  Issuer: MAS-DID (did:key:z6MkMAS...)               │
│  Subject: User DID (did:key:z6MkUser...)            │
│  Claims:                                             │
│    • mlsKeyPackage: <base64 KeyPackage>             │
│    • deviceId: "alice_laptop_001"                   │
│    • issuedAt: "2024-01-15T10:30:00Z"               │
│    • expiresAt: "2024-02-15T10:30:00Z"              │
│                                                      │
│  Signature: <MAS-DID's signature>                   │
└──────────────────────────────────────────────────────┘
```

**What this proves**:
- User with `did:key:z6MkUser...` authenticated to MAS-DID
- User proved ownership of private key for MLS KeyPackage
- MAS-DID cryptographically attests to this binding
- Anyone trusting MAS-DID can verify this proof

### Trust Model

```
                    ┌─────────────────┐
                    │    MAS-DID      │
                    │  (Root of Trust)│
                    └────────┬────────┘
                             │ issues VCs
                             │ with signature
                             │
        ┌────────────────────┼────────────────────┐
        │                    │                    │
        ▼                    ▼                    ▼
┌───────────────┐    ┌───────────────┐    ┌───────────────┐
│ Alice's       │    │ Bob's         │    │ Carol's       │
│ Credential    │    │ Credential    │    │ Credential    │
│               │    │               │    │               │
│ DID → KeyPkg  │    │ DID → KeyPkg  │    │ DID → KeyPkg  │
└───────────────┘    └───────────────┘    └───────────────┘
```

**Trust Chain**:
1. Users trust their homeserver's MAS-DID
2. MAS-DID issues credentials binding DIDs to keys
3. Users verify credentials via MAS-DID signature
4. Transitive trust: Alice trusts Bob's key because Alice trusts MAS-DID

## MLS Key Concepts

### KeyPackage

An MLS **KeyPackage** is a bundle of cryptographic material published by a client:

**Contents**:
- Protocol version
- Cipher suite
- Init key (public key for this device)
- Leaf node (identity + capabilities)
- Extensions
- Signature (by device's private key)

**Lifecycle**:
1. Client generates KeyPackage for each device
2. Client uploads to server
3. Server distributes to group members
4. Group uses KeyPackage to add device
5. KeyPackage consumed (single-use)

### Credential in MLS Terminology

MLS RFC 9420 defines a **Credential** as:

```rust
pub enum Credential {
    Basic(BasicCredential),      // Just a user ID
    X509(Certificate),           // X.509 cert
    External(ExternalCredential) // Custom type - THIS IS WHERE DID/VC FITS
}
```

**Our Approach**: Use `ExternalCredential` type to carry our VC.

## Integration with mls-rs

### Why mls-rs?

`mls-rs` from AWS Labs is ideal because:
- **RFC 9420 compliant**: Full MLS protocol implementation
- **Custom validation hooks**: Allows plugging in VC verification
- **External credential support**: Designed for custom credential types
- **Well-tested**: Used in production AWS systems

### Custom Validation Hook

`mls-rs` provides `IdentityProvider` trait:

```rust
use mls_rs::identity::IdentityProvider;
use mls_rs::identity::SigningIdentity;

#[async_trait]
pub trait IdentityProvider: Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;

    /// Verify a signing identity is valid
    async fn validate_member(
        &self,
        signing_identity: &SigningIdentity,
        timestamp: Option<u64>,
    ) -> Result<(), Self::Error>;

    /// Verify an identity for external senders
    async fn validate_external_sender(
        &self,
        signing_identity: &SigningIdentity,
    ) -> Result<(), Self::Error>;
}
```

### Our Implementation

```rust
use mls_rs::identity::IdentityProvider;
use ssi::vc::Credential;
use ssi::did::DIDResolver;

pub struct DIDIdentityProvider {
    did_resolver: DIDResolver,
    trusted_issuers: Vec<String>, // MAS-DID DIDs
    vc_verifier: VCVerifier,
}

#[async_trait]
impl IdentityProvider for DIDIdentityProvider {
    type Error = anyhow::Error;

    async fn validate_member(
        &self,
        signing_identity: &SigningIdentity,
        timestamp: Option<u64>,
    ) -> Result<(), Self::Error> {
        // 1. Extract VC from signing identity
        let credential_data = signing_identity.credential.as_ref();
        let vc: Credential = serde_json::from_slice(credential_data)
            .context("Failed to parse credential as VC")?;

        // 2. Verify VC signature
        self.vc_verifier.verify_vc(&vc, &self.did_resolver).await
            .context("VC signature verification failed")?;

        // 3. Verify issuer is trusted (our MAS-DID)
        if !self.trusted_issuers.contains(&vc.issuer.to_string()) {
            return Err(anyhow!("Untrusted VC issuer: {}", vc.issuer));
        }

        // 4. Verify expiration
        if let Some(ts) = timestamp {
            if let Some(exp) = &vc.expiration_date {
                let exp_ts = exp.timestamp() as u64;
                if ts > exp_ts {
                    return Err(anyhow!("VC expired"));
                }
            }
        }

        // 5. Extract and verify KeyPackage from VC
        let mls_key_package = vc.credential_subject
            .get("mlsKeyPackage")
            .ok_or(anyhow!("No mlsKeyPackage in VC"))?
            .as_str()
            .ok_or(anyhow!("Invalid mlsKeyPackage"))?;

        let key_package_bytes = base64::decode(mls_key_package)?;

        // 6. Verify KeyPackage signature
        let key_package = mls_rs::KeyPackage::from_bytes(&key_package_bytes)?;
        key_package.verify_signature()
            .context("KeyPackage signature verification failed")?;

        // 7. Verify KeyPackage matches the one in signing identity
        if signing_identity.public_key != key_package.init_key() {
            return Err(anyhow!("KeyPackage mismatch"));
        }

        info!(
            did = %vc.credential_subject.get("id").unwrap(),
            "Member identity validated successfully"
        );

        Ok(())
    }

    async fn validate_external_sender(
        &self,
        signing_identity: &SigningIdentity,
    ) -> Result<(), Self::Error> {
        // Same validation logic
        self.validate_member(signing_identity, None).await
    }
}
```

## VC Schema: MlsKeyPackageCredential

### Full Schema

```json
{
  "@context": [
    "https://www.w3.org/2018/credentials/v1",
    "https://matrix.org/credentials/mls/v1"
  ],
  "type": ["VerifiableCredential", "MlsKeyPackageCredential"],
  "issuer": "did:key:z6MkMAS...",
  "issuanceDate": "2024-01-15T10:30:00Z",
  "expirationDate": "2024-02-15T10:30:00Z",
  "credentialSubject": {
    "id": "did:key:z6MkUser...",
    "mlsKeyPackage": "base64-encoded-KeyPackage",
    "deviceId": "alice_laptop_001",
    "matrixUserId": "@alice:domain.com"
  },
  "proof": {
    "type": "Ed25519Signature2020",
    "created": "2024-01-15T10:30:00Z",
    "verificationMethod": "did:key:z6MkMAS...#key-1",
    "proofPurpose": "assertionMethod",
    "proofValue": "z5a..."
  }
}
```

### Field Descriptions

| Field | Description |
|-------|-------------|
| `issuer` | MAS-DID's DID (the Authentication Service) |
| `credentialSubject.id` | User's DID |
| `credentialSubject.mlsKeyPackage` | Base64-encoded MLS KeyPackage |
| `credentialSubject.deviceId` | Device identifier (e.g., "alice_phone_1") |
| `credentialSubject.matrixUserId` | Matrix user ID (for convenience) |
| `issuanceDate` | When credential issued |
| `expirationDate` | When credential expires (30 days recommended) |

## Credential Issuance Flow

### When to Issue

Credentials are issued when:
1. **New Device Login**: User authenticates new device
2. **KeyPackage Rotation**: User generates new KeyPackage
3. **Credential Expiration**: Old credential expires (re-issue)

### Issuance Sequence

```
┌────────┐           ┌─────────┐           ┌────────┐
│ Client │           │ MAS-DID │           │ Wallet │
└───┬────┘           └────┬────┘           └───┬────┘
    │                     │                    │
    │ 1. Generate         │                    │
    │    MLS KeyPackage   │                    │
    │                     │                    │
    │ 2. Request          │                    │
    │    Credential       │                    │
    ├────────────────────>│                    │
    │                     │                    │
    │                     │ 3. Challenge user  │
    │                     │    to prove DID    │
    │                     ├───────────────────>│
    │                     │                    │
    │                     │ 4. Signed proof    │
    │                     │<───────────────────┤
    │                     │                    │
    │                     │ 5. Verify DID      │
    │                     │                    │
    │                     │ 6. Verify KeyPkg   │
    │                     │    signature       │
    │                     │                    │
    │                     │ 7. Issue VC        │
    │                     │    binding         │
    │                     │    DID→KeyPackage  │
    │                     │                    │
    │ 8. Return VC        │                    │
    │<────────────────────┤                    │
    │                     │                    │
    │ 9. Store VC         │                    │
```

### API Endpoint

#### `POST /mls/credential/issue`

Request credential for new KeyPackage.

**Request Body**:
```json
{
  "keyPackage": "base64-encoded-KeyPackage",
  "deviceId": "alice_laptop_001",
  "proof": {
    "type": "SIOPv2",
    "idToken": "eyJ..."
  }
}
```

**Processing**:
```rust
async fn issue_mls_credential(
    Json(request): Json<IssueCredentialRequest>,
    session: Session,
) -> Result<Json<CredentialResponse>> {
    // 1. Verify user authentication
    let did = verify_authentication(&request.proof).await?;

    // 2. Decode and verify KeyPackage
    let key_package_bytes = base64::decode(&request.key_package)?;
    let key_package = mls_rs::KeyPackage::from_bytes(&key_package_bytes)?;

    // 3. Verify KeyPackage signature
    key_package.verify_signature()
        .context("Invalid KeyPackage signature")?;

    // 4. Verify KeyPackage not expired
    if key_package.is_expired() {
        return Err(anyhow!("KeyPackage expired"));
    }

    // 5. Lookup user's MXID
    let mxid = lookup_mxid_by_did(&did).await?
        .ok_or(anyhow!("DID not registered"))?;

    // 6. Build credential
    let credential = build_mls_credential(
        &did,
        &request.key_package,
        &request.device_id,
        &mxid,
    ).await?;

    // 7. Sign credential with MAS-DID's key
    let signed_credential = sign_credential(&credential).await?;

    // 8. Store in database
    store_mls_credential(&did, &request.device_id, &signed_credential).await?;

    info!(
        did = %did,
        device_id = %request.device_id,
        "MLS credential issued"
    );

    Ok(Json(CredentialResponse {
        credential: signed_credential,
    }))
}
```

### Credential Building

```rust
use ssi::vc::{Credential, CredentialSubject, Proof};
use chrono::{Duration, Utc};

async fn build_mls_credential(
    user_did: &str,
    key_package_b64: &str,
    device_id: &str,
    mxid: &str,
) -> Result<Credential> {
    let now = Utc::now();
    let expiration = now + Duration::days(30);

    let mut credential = Credential {
        context: vec![
            "https://www.w3.org/2018/credentials/v1".to_string(),
            "https://matrix.org/credentials/mls/v1".to_string(),
        ],
        type_: vec![
            "VerifiableCredential".to_string(),
            "MlsKeyPackageCredential".to_string(),
        ],
        issuer: get_mas_did().await?,
        issuance_date: Some(now),
        expiration_date: Some(expiration),
        credential_subject: json!({
            "id": user_did,
            "mlsKeyPackage": key_package_b64,
            "deviceId": device_id,
            "matrixUserId": mxid,
        }),
        proof: None, // Will be added during signing
    };

    Ok(credential)
}

async fn sign_credential(credential: &Credential) -> Result<Credential> {
    let signing_key = get_mas_signing_key().await?;
    let did_resolver = DIDResolver::default();

    let mut signed = credential.clone();
    signed.generate_proof(
        &signing_key,
        &did_resolver,
        ProofOptions {
            proof_purpose: ProofPurpose::AssertionMethod,
            verification_method: format!("{}#key-1", get_mas_did().await?),
            ..Default::default()
        },
    ).await?;

    Ok(signed)
}
```

## Credential Verification Flow

### When to Verify

Credentials are verified when:
1. **Adding Member to Group**: Before accepting new member
2. **Joining Group**: Existing members verify new joiner
3. **Message Reception**: Optionally verify sender (policy-dependent)

### Verification Sequence

```
┌────────┐           ┌─────────┐           ┌─────────┐
│ Alice  │           │   Bob   │           │ MAS-DID │
│        │           │         │           │  (AS)   │
└───┬────┘           └────┬────┘           └────┬────┘
    │                     │                     │
    │ 1. Wants to add     │                     │
    │    Bob to group     │                     │
    │                     │                     │
    │ 2. Request Bob's    │                     │
    │    MLS credential   │                     │
    ├────────────────────>│                     │
    │                     │                     │
    │ 3. Send credential  │                     │
    │<────────────────────┤                     │
    │                     │                     │
    │ 4. Verify:          │                     │
    │    • VC signature   │                     │
    │    • Issuer is      │                     │
    │      trusted AS     │                     │
    │    • Not expired    │                     │
    │    • KeyPackage     │                     │
    │      valid          │                     │
    │                     │                     │
    │ 5. Add Bob using    │                     │
    │    KeyPackage       │                     │
    ├────────────────────>│                     │
```

### Verification Logic

Handled by `DIDIdentityProvider` (shown earlier), but key steps:

```rust
async fn verify_mls_credential(
    vc: &Credential,
    trusted_as_dids: &[String],
) -> Result<VerifiedCredential> {
    // 1. Verify VC signature
    let resolver = DIDResolver::default();
    let verification_result = vc.verify(None, &resolver).await;

    if !verification_result.errors.is_empty() {
        return Err(anyhow!("VC signature verification failed"));
    }

    // 2. Verify issuer is trusted AS
    if !trusted_as_dids.contains(&vc.issuer.to_string()) {
        return Err(anyhow!("Untrusted issuer"));
    }

    // 3. Verify not expired
    if let Some(exp) = &vc.expiration_date {
        if exp < &Utc::now() {
            return Err(anyhow!("Credential expired"));
        }
    }

    // 4. Extract KeyPackage
    let key_package_b64 = vc.credential_subject
        .get("mlsKeyPackage")
        .ok_or(anyhow!("No mlsKeyPackage"))?
        .as_str()
        .ok_or(anyhow!("Invalid mlsKeyPackage"))?;

    let key_package_bytes = base64::decode(key_package_b64)?;
    let key_package = mls_rs::KeyPackage::from_bytes(&key_package_bytes)?;

    // 5. Verify KeyPackage signature
    key_package.verify_signature()?;

    // 6. Extract DID
    let user_did = vc.credential_subject
        .get("id")
        .ok_or(anyhow!("No id in credentialSubject"))?
        .as_str()
        .ok_or(anyhow!("Invalid id"))?;

    Ok(VerifiedCredential {
        did: user_did.to_string(),
        key_package,
        device_id: vc.credential_subject
            .get("deviceId")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
    })
}
```

## Credential Distribution

### Discovery Endpoint

Clients need to discover other users' MLS credentials.

#### `GET /mls/credential/user/{mxid}`

Retrieve MLS credentials for a user.

**Response**:
```json
{
  "credentials": [
    {
      "deviceId": "alice_laptop_001",
      "credential": { /* full VC */ }
    },
    {
      "deviceId": "alice_phone_1",
      "credential": { /* full VC */ }
    }
  ]
}
```

**Implementation**:
```rust
async fn get_user_credentials(
    Path(mxid): Path<String>,
) -> Result<Json<CredentialsResponse>> {
    // Lookup DID by MXID
    let did = lookup_did_by_mxid(&mxid).await?
        .ok_or(anyhow!("User not found"))?;

    // Query active credentials for this DID
    let credentials = query_mls_credentials(&did).await?;

    Ok(Json(CredentialsResponse {
        credentials: credentials.into_iter().map(|c| DeviceCredential {
            device_id: c.device_id,
            credential: c.vc,
        }).collect(),
    }))
}
```

### Caching Strategy

Clients should cache credentials but revalidate periodically:

```rust
struct CredentialCache {
    cache: HashMap<String, CachedCredential>,
}

struct CachedCredential {
    credential: Credential,
    fetched_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
}

impl CredentialCache {
    async fn get_or_fetch(
        &mut self,
        mxid: &str,
        device_id: &str,
    ) -> Result<Credential> {
        let key = format!("{}:{}", mxid, device_id);

        // Check cache
        if let Some(cached) = self.cache.get(&key) {
            if Utc::now() < cached.expires_at {
                return Ok(cached.credential.clone());
            }
        }

        // Fetch fresh credential
        let credential = fetch_credential(mxid, device_id).await?;
        let expires_at = credential.expiration_date
            .unwrap_or_else(|| Utc::now() + Duration::days(1));

        self.cache.insert(key, CachedCredential {
            credential: credential.clone(),
            fetched_at: Utc::now(),
            expires_at,
        });

        Ok(credential)
    }
}
```

## Revocation

### When to Revoke

Credentials must be revoked when:
1. **Device Compromised**: User reports device stolen
2. **User Deactivated**: Account closed
3. **Key Compromise**: Private key potentially exposed
4. **Policy Violation**: User violates terms of service

### Revocation Mechanism

Use **Status List 2021** for efficient revocation:

```json
{
  "@context": "https://www.w3.org/2018/credentials/v1",
  "type": ["VerifiableCredential", "StatusList2021Credential"],
  "issuer": "did:key:z6MkMAS...",
  "issuanceDate": "2024-01-01T00:00:00Z",
  "credentialSubject": {
    "id": "https://mas-did.example.com/mls/status/1",
    "type": "StatusList2021",
    "statusPurpose": "revocation",
    "encodedList": "H4sIAAAAAAAA..." // Compressed bitstring
  }
}
```

**How it works**:
1. Each credential includes `credentialStatus` field
2. Field points to bit position in status list
3. Verifier fetches status list
4. Verifier checks if bit is set (1 = revoked, 0 = valid)

**In our MLS credential**:
```json
{
  "@context": [...],
  "type": ["VerifiableCredential", "MlsKeyPackageCredential"],
  "credentialSubject": { ... },
  "credentialStatus": {
    "id": "https://mas-did.example.com/mls/status/1#12345",
    "type": "StatusList2021Entry",
    "statusPurpose": "revocation",
    "statusListIndex": "12345",
    "statusListCredential": "https://mas-did.example.com/mls/status/1"
  }
}
```

### Revocation API

#### `POST /mls/credential/revoke`

Revoke a credential.

**Request**:
```json
{
  "deviceId": "alice_laptop_001",
  "reason": "Device reported stolen"
}
```

**Implementation**:
```rust
async fn revoke_credential(
    Json(request): Json<RevokeRequest>,
    session: Session,
) -> Result<StatusCode> {
    // Verify user authenticated
    let did = session.get::<String>("authenticated_did")?
        .ok_or(anyhow!("Not authenticated"))?;

    // Lookup credential
    let credential = get_mls_credential(&did, &request.device_id).await?
        .ok_or(anyhow!("Credential not found"))?;

    // Extract status list index
    let status_index = credential.credential_status
        .as_ref()
        .and_then(|s| s.get("statusListIndex"))
        .and_then(|v| v.as_u64())
        .ok_or(anyhow!("No status index"))?;

    // Update status list (set bit)
    set_status_list_bit(1, status_index as usize, true).await?;

    // Record revocation
    store_revocation_record(&did, &request.device_id, &request.reason).await?;

    info!(
        did = %did,
        device_id = %request.device_id,
        reason = %request.reason,
        "MLS credential revoked"
    );

    Ok(StatusCode::NO_CONTENT)
}
```

## Federation: Cross-Domain Trust

### The Federation Problem

```
Alice@server-A wants to message Bob@server-B
- Alice trusts server-A's MAS-DID
- Bob trusts server-B's MAS-DID
- How can Alice trust Bob's credentials issued by server-B?
```

### Solution: Trust Registry

Servers maintain list of trusted MAS-DID instances:

```rust
struct TrustRegistry {
    trusted_issuers: HashMap<String, TrustedIssuer>,
}

struct TrustedIssuer {
    domain: String,
    did: String,
    added_at: DateTime<Utc>,
    verified: bool,
}

impl TrustRegistry {
    async fn is_trusted(&self, issuer_did: &str) -> bool {
        self.trusted_issuers.values()
            .any(|i| i.did == issuer_did && i.verified)
    }

    async fn add_trusted_domain(&mut self, domain: &str) -> Result<()> {
        // Fetch domain's MAS-DID via .well-known
        let did = fetch_mas_did_for_domain(domain).await?;

        // Verify DID resolves
        let resolver = DIDResolver::default();
        resolver.resolve(&did, &Default::default()).await?;

        self.trusted_issuers.insert(domain.to_string(), TrustedIssuer {
            domain: domain.to_string(),
            did,
            added_at: Utc::now(),
            verified: true,
        });

        Ok(())
    }
}
```

### Federation Discovery

#### `.well-known/matrix/mas-did`

Servers advertise their MAS-DID:

```json
{
  "did": "did:key:z6MkMAS...",
  "mls_as_enabled": true,
  "credential_endpoint": "https://mas-did.server-a.com/mls/credential/user/{mxid}"
}
```

**Verification**:
```rust
async fn verify_federated_credential(
    vc: &Credential,
    trust_registry: &TrustRegistry,
) -> Result<()> {
    // Extract issuer
    let issuer = vc.issuer.to_string();

    // Check trust registry
    if !trust_registry.is_trusted(&issuer).await {
        return Err(anyhow!("Untrusted federated issuer"));
    }

    // Standard verification
    verify_mls_credential(vc, &[issuer]).await
}
```

## Client Integration

### Matrix Client Changes

Clients need minimal changes:

1. **On Device Login**:
   ```rust
   // Generate KeyPackage
   let key_package = mls_client.generate_key_package().await?;

   // Request credential from MAS-DID
   let credential = mas_client.request_mls_credential(key_package).await?;

   // Store credential
   storage.store_mls_credential(credential).await?;
   ```

2. **When Adding User to Group**:
   ```rust
   // Fetch user's credentials
   let credentials = mas_client.get_mls_credentials(user_id).await?;

   // Verify each credential
   for cred in credentials {
       verify_mls_credential(&cred.credential).await?;
   }

   // Use verified KeyPackage to add to group
   mls_group.add_member(credentials[0].key_package).await?;
   ```

3. **Periodic Re-verification** (optional):
   ```rust
   // Check revocation status before important operations
   for member in group.members() {
       let cred = get_member_credential(member).await?;
       check_credential_status(&cred).await?;
   }
   ```

## Security Considerations

### KeyPackage Proof-of-Possession

Critical: User must prove they own private key for KeyPackage before issuance.

**Current flow**:
1. User generates KeyPackage
2. KeyPackage includes signature (by private key)
3. MAS-DID verifies signature
4. This proves possession

**Attack prevented**: User cannot request credential for someone else's KeyPackage.

### Credential Binding

VC must cryptographically bind DID to KeyPackage:

```rust
// In verify_mls_credential:
// Extract DID from VC
let claimed_did = vc.credential_subject.get("id")?;

// Extract KeyPackage
let key_package = extract_key_package(vc)?;

// Verify KeyPackage signature uses key from DID document
let did_doc = resolve_did(claimed_did).await?;
let did_public_key = did_doc.verification_method[0].public_key;

// This verification ensures no one can claim someone else's KeyPackage
verify_key_package_matches_did(&key_package, &did_public_key)?;
```

### Time-of-Check-Time-of-Use (TOCTOU)

**Risk**: Credential valid during verification but revoked before use.

**Mitigation**:
- Short credential lifetimes (30 days max)
- Check revocation status immediately before sensitive operations
- Use cached status lists with short TTL (5 minutes)

### Replay Attacks

**Risk**: Attacker reuses old credential.

**Prevention**:
- Expiration dates enforced
- KeyPackages single-use (MLS protocol requirement)
- Once KeyPackage consumed, credential no longer useful

## Performance Optimization

### Batch Verification

When verifying group of credentials:

```rust
async fn verify_credentials_batch(
    credentials: Vec<Credential>,
) -> Result<Vec<VerifiedCredential>> {
    let mut set = JoinSet::new();

    for cred in credentials {
        set.spawn(async move {
            verify_mls_credential(&cred).await
        });
    }

    let mut verified = Vec::new();
    while let Some(result) = set.join_next().await {
        verified.push(result??);
    }

    Ok(verified)
}
```

### Status List Caching

```rust
struct StatusListCache {
    cache: HashMap<String, CachedStatusList>,
    ttl: Duration,
}

impl StatusListCache {
    async fn is_revoked(&mut self, cred: &Credential) -> Result<bool> {
        let status = cred.credential_status.as_ref()
            .ok_or(anyhow!("No status"))?;

        let list_url = status.get("statusListCredential")
            .and_then(|v| v.as_str())
            .ok_or(anyhow!("No list URL"))?;

        let index = status.get("statusListIndex")
            .and_then(|v| v.as_u64())
            .ok_or(anyhow!("No index"))?;

        // Get cached or fetch
        let status_list = self.get_or_fetch(list_url).await?;

        // Check bit
        Ok(status_list.is_set(index as usize))
    }
}
```

## Next Steps

- [VC-Based Authorization](./vc-authorization.md): Room access control
- [Federation & Roadmap](./federation-roadmap.md): Implementation plan
