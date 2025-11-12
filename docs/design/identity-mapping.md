# Identity Mapping & 3PID Design

## Overview

This document describes how MAS-DID maps Decentralized Identifiers (DIDs) to Matrix User IDs (MXIDs) and implements the custom "Newnal Mobile Address" Third-Party ID (3PID) system.

## The Privacy Problem

**Bad Approach**: Use DID as Matrix user ID localpart

```
DID: did:key:z6MkpTHR8VNsBxYAAWHut2Geadd9jSwuBV8xRoAnwWsdvktH
↓
MXID: @did:key:z6MkpTHR8VNsBxYAAWHut2Geadd9jSwuBV8xRoAnwWsdvktH:domain.com
```

**Problems**:
1. **Privacy Violation**: DID exposed in every Matrix message
2. **Linkability**: Anyone can track user across services via DID
3. **Immutability**: Cannot change MXID if DID compromised
4. **Usability**: 60+ character user IDs are unusable
5. **Breaking Change**: Fundamental Matrix ID format violation

## The Solution: external_id Mapping

Synapse's Admin API provides an `external_id` feature specifically designed for this use case. It allows atomic association of external identifiers with Matrix user IDs.

### How external_id Works

**API Endpoint**: `PUT /_synapse/admin/v2/users/{user_id}`

**Request Body**:
```json
{
  "external_ids": [
    {
      "auth_provider": "mas-did",
      "external_id": "did:key:z6MkpTHR8VNsBxYAAWHut2Geadd9jSwuBV8xRoAnwWsdvktH"
    }
  ]
}
```

**Result**:
- DID stored in `user_external_ids` table
- Indexed for fast lookup
- Unique constraint enforced
- Atomic transaction with user creation

### Lookup During Login

**API Endpoint**: `GET /_synapse/admin/v1/auth_providers/{provider}/users/{external_id}`

**Example**:
```
GET /_synapse/admin/v1/auth_providers/mas-did/users/did:key:z6Mk...
```

**Response**:
```json
{
  "user_id": "@01HQXYZ789ABCDEF:domain.com"
}
```

**If not found**: HTTP 404

## Opaque MXID Generation

MAS-DID generates opaque, privacy-preserving Matrix user IDs using ULIDs (Universally Unique Lexicographically Sortable Identifiers).

### Why ULIDs?

- **Unique**: 128-bit random identifier
- **Sortable**: Lexicographic ordering (contains timestamp)
- **URL-Safe**: Base32 encoded (26 characters)
- **Collision-Resistant**: 2^128 possible values
- **No PII**: Contains no user information

### ULID Format

```
01HQXYZ789ABCDEFGHJKMNPQRS
```

Structure:
- First 10 characters: timestamp (millisecond precision)
- Last 16 characters: randomness

### Implementation

```rust
use ulid::Ulid;

fn generate_opaque_mxid(domain: &str) -> String {
    let ulid = Ulid::new();
    format!("@{}:{}", ulid.to_string().to_lowercase(), domain)
}

// Example output: @01hqxyz789abcdefghjkmnpqrs:domain.com
```

### Benefits

1. **Privacy**: No correlation with DID or other identifiers
2. **Stability**: MXID never needs to change
3. **Compatibility**: Valid Matrix user ID format
4. **Sortable**: Creation order preserved (useful for admin queries)

## Registration Flow

### New User Registration

When a user registers for the first time, MAS-DID performs an atomic operation:

```rust
use synapse_admin_api::AdminApi;

async fn register_user_with_did(
    admin_api: &AdminApi,
    did: &str,
    newnal_address: Option<&str>,
    domain: &str,
) -> Result<String> {
    // Generate opaque MXID
    let mxid = generate_opaque_mxid(domain);

    // Build user creation request
    let mut request = CreateUserRequest::new(mxid.clone());

    // Set external_id (DID mapping)
    request.external_ids(vec![
        ExternalId {
            auth_provider: "mas-did".to_string(),
            external_id: did.to_string(),
        }
    ]);

    // Add Newnal Address as 3PID if provided
    if let Some(address) = newnal_address {
        request.threepids(vec![
            ThreePid {
                medium: "m.id.newnal".to_string(),
                address: address.to_string(),
            }
        ]);
    }

    // Create user atomically
    admin_api.create_user(request).await?;

    info!(
        mxid = %mxid,
        did = %did,
        newnal_address = ?newnal_address,
        "User registered successfully"
    );

    Ok(mxid)
}
```

### Atomicity Guarantees

Synapse's Admin API ensures:
1. User creation + external_id mapping is atomic
2. If external_id already exists → error (no duplicate DIDs)
3. If MXID already exists → error (no collisions)
4. Transaction rollback on any failure

### Error Handling

**DID Already Registered**:
```
HTTP 409 Conflict
{
  "errcode": "M_USER_IN_USE",
  "error": "external_id already mapped to different user"
}
```

**MXID Collision** (extremely rare with ULIDs):
```
HTTP 409 Conflict
{
  "errcode": "M_USER_IN_USE",
  "error": "User ID already taken"
}
```

## Login Flow

### Existing User Login

When a user logs in with their DID, MAS-DID looks up the associated MXID:

```rust
async fn lookup_user_by_did(
    admin_api: &AdminApi,
    did: &str,
) -> Result<Option<String>> {
    match admin_api.get_user_by_external_id("mas-did", did).await {
        Ok(user) => Ok(Some(user.user_id)),
        Err(e) if e.status_code() == 404 => Ok(None),
        Err(e) => Err(e),
    }
}

async fn handle_login(
    admin_api: &AdminApi,
    verified_did: &str,
) -> Result<LoginResult> {
    match lookup_user_by_did(admin_api, verified_did).await? {
        Some(mxid) => {
            info!(did = %verified_did, mxid = %mxid, "User logged in");
            Ok(LoginResult::Success { mxid })
        }
        None => {
            warn!(did = %verified_did, "DID not registered");
            Ok(LoginResult::NotRegistered)
        }
    }
}
```

### Handling Unregistered DIDs

If DID lookup returns 404, user is not registered:

**Option 1**: Redirect to registration flow
```rust
if lookup.is_none() {
    return Redirect::to("/register?did={verified_did}");
}
```

**Option 2**: Auto-register (if policy allows)
```rust
if lookup.is_none() && auto_registration_enabled() {
    let mxid = register_user_with_did(admin_api, verified_did, None, domain).await?;
    return Ok(LoginResult::Success { mxid });
}
```

## Newnal Mobile Address (3PID)

### Custom 3PID Type

Matrix supports arbitrary 3PID types. MAS-DID introduces `m.id.newnal`:

**Standard 3PIDs**:
- `email`: Email addresses
- `msisdn`: Phone numbers

**Custom 3PID**:
- `m.id.newnal`: Newnal Mobile Addresses (e.g., `+82N1012345678`)

### 3PID Structure

```json
{
  "medium": "m.id.newnal",
  "address": "+82N1012345678"
}
```

**Format Requirements**:
- Must start with `+`
- Followed by country code
- Followed by `N` (Newnal indicator)
- Followed by digits
- Example: `+82N1012345678` (South Korea)

### Verification via Verifiable Credentials

**Traditional 3PID Verification**:
- Email: MAS sends verification email with token
- Phone: MAS sends SMS with code

**Newnal Address Verification**:
- User presents `NewnalAddressVC` during registration
- MAS-DID verifies VC issuer signature
- MAS-DID checks VC not expired/revoked
- MAS-DID extracts Newnal Address from VC
- No network call to external verifier needed

### NewnalAddressVC Schema

```json
{
  "@context": [
    "https://www.w3.org/2018/credentials/v1",
    "https://newnal.example/credentials/v1"
  ],
  "type": ["VerifiableCredential", "NewnalAddressCredential"],
  "issuer": "did:example:newnal-registrar",
  "issuanceDate": "2024-01-01T00:00:00Z",
  "expirationDate": "2025-01-01T00:00:00Z",
  "credentialSubject": {
    "id": "did:key:z6MkUserDID...",
    "newnalAddress": "+82N1012345678"
  },
  "proof": {
    "type": "Ed25519Signature2020",
    "created": "2024-01-01T00:00:00Z",
    "verificationMethod": "did:example:newnal-registrar#key-1",
    "proofPurpose": "assertionMethod",
    "proofValue": "z3F..."
  }
}
```

### Verification Logic

```rust
use ssi::vc::{Credential, VerificationResult};
use ssi::did::DIDResolver;

async fn verify_newnal_address_vc(
    vc: &Credential,
    holder_did: &str,
    trusted_issuers: &[String],
) -> Result<String> {
    // 1. Verify issuer is trusted
    if !trusted_issuers.contains(&vc.issuer.to_string()) {
        return Err(anyhow!("Untrusted issuer"));
    }

    // 2. Verify VC signature
    let resolver = DIDResolver::default();
    let result = vc.verify(None, &resolver).await;
    if !result.errors.is_empty() {
        return Err(anyhow!("VC signature verification failed"));
    }

    // 3. Verify not expired
    if let Some(exp) = &vc.expiration_date {
        if exp < &chrono::Utc::now() {
            return Err(anyhow!("VC expired"));
        }
    }

    // 4. Verify credential subject matches holder
    let subject_id = vc.credential_subject
        .get("id")
        .ok_or(anyhow!("No subject id"))?
        .as_str()
        .ok_or(anyhow!("Invalid subject id"))?;

    if subject_id != holder_did {
        return Err(anyhow!("Subject DID mismatch"));
    }

    // 5. Extract Newnal Address
    let newnal_address = vc.credential_subject
        .get("newnalAddress")
        .ok_or(anyhow!("No newnalAddress"))?
        .as_str()
        .ok_or(anyhow!("Invalid newnalAddress"))?;

    // 6. Validate format
    if !newnal_address.starts_with('+') || !newnal_address.contains('N') {
        return Err(anyhow!("Invalid Newnal Address format"));
    }

    Ok(newnal_address.to_string())
}
```

### Registration with Newnal Address

```rust
async fn register_with_newnal_address(
    admin_api: &AdminApi,
    did: &str,
    vc: &Credential,
    trusted_issuers: &[String],
    domain: &str,
) -> Result<String> {
    // Verify VC and extract address
    let newnal_address = verify_newnal_address_vc(vc, did, trusted_issuers).await?;

    // Check address not already taken
    if address_exists(&newnal_address).await? {
        return Err(anyhow!("Newnal Address already registered"));
    }

    // Create user with both DID mapping and 3PID
    let mxid = register_user_with_did(
        admin_api,
        did,
        Some(&newnal_address),
        domain,
    ).await?;

    Ok(mxid)
}
```

## 3PID Management UI

### The Problem

Per MSC3861, when MAS controls authentication, **Matrix clients cannot modify 3PIDs**. This is by design:
- MAS has exclusive control over user accounts
- Clients cannot directly call Synapse's 3PID endpoints
- All account modifications must go through MAS

### The Solution

MAS-DID must provide a **separate web-based account management UI** (e.g., `account.domain.com`).

### Account Management Endpoints

#### `GET /account`
Landing page, requires authentication

**Response**: HTML page with:
- Current Newnal Addresses
- "Add Newnal Address" button
- "Remove Newnal Address" button per address

#### `POST /account/newnal-address/add`
Add new Newnal Address

**Flow**:
1. User clicks "Add Newnal Address"
2. MAS-DID displays QR code requesting `NewnalAddressVC`
3. User scans QR with wallet
4. Wallet presents VC
5. MAS-DID verifies VC
6. MAS-DID calls Synapse Admin API to add 3PID
7. Success page shown

**Request** (after VC verification):
```rust
async fn add_newnal_address(
    admin_api: &AdminApi,
    mxid: &str,
    newnal_address: &str,
) -> Result<()> {
    // Get current user info
    let mut user = admin_api.get_user(mxid).await?;

    // Add new 3PID
    user.threepids.push(ThreePid {
        medium: "m.id.newnal".to_string(),
        address: newnal_address.to_string(),
    });

    // Update user
    admin_api.update_user(mxid, UpdateUserRequest {
        threepids: Some(user.threepids),
        ..Default::default()
    }).await?;

    Ok(())
}
```

#### `POST /account/newnal-address/remove`
Remove existing Newnal Address

**Request Body**:
```json
{
  "address": "+82N1012345678"
}
```

**Processing**:
```rust
async fn remove_newnal_address(
    admin_api: &AdminApi,
    mxid: &str,
    address: &str,
) -> Result<()> {
    let mut user = admin_api.get_user(mxid).await?;

    // Remove 3PID
    user.threepids.retain(|pid| {
        !(pid.medium == "m.id.newnal" && pid.address == address)
    });

    // Update user
    admin_api.update_user(mxid, UpdateUserRequest {
        threepids: Some(user.threepids),
        ..Default::default()
    }).await?;

    Ok(())
}
```

### Authentication for Account Management UI

Users access account management UI via:

1. **Same-Session**: After login, provide link to `/account`
2. **Direct Access**: User navigates to `account.domain.com`, MAS-DID initiates SIOPv2 auth
3. **Token-Based**: Element includes account management link with short-lived token

## 3PID Uniqueness Constraints

### Enforcing Uniqueness

Newnal Addresses must be unique (one user per address):

```rust
async fn check_address_unique(
    admin_api: &AdminApi,
    address: &str,
) -> Result<bool> {
    // Query all users with this 3PID
    let users = admin_api.query_users(QueryParams {
        threepid_medium: Some("m.id.newnal".to_string()),
        threepid_address: Some(address.to_string()),
        ..Default::default()
    }).await?;

    Ok(users.is_empty())
}
```

**Before Registration**:
- Check address not in use
- Reject if duplicate

**Before Add**:
- Check address not assigned to other user
- Allow if already assigned to same user (idempotent)

### Synapse Behavior

Synapse does **not** enforce 3PID uniqueness at database level. MAS-DID must enforce:

```rust
async fn add_newnal_address_safe(
    admin_api: &AdminApi,
    mxid: &str,
    address: &str,
) -> Result<()> {
    // Check uniqueness
    let users = query_users_by_3pid(admin_api, "m.id.newnal", address).await?;

    if !users.is_empty() && !users.contains(&mxid.to_string()) {
        return Err(anyhow!("Address already in use by another user"));
    }

    // Safe to add
    add_newnal_address(admin_api, mxid, address).await
}
```

## Data Model

### Synapse Tables

**user_external_ids**:
```sql
CREATE TABLE user_external_ids (
    auth_provider TEXT NOT NULL,
    external_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    UNIQUE (auth_provider, external_id)
);
```

**user_threepids**:
```sql
CREATE TABLE user_threepids (
    user_id TEXT NOT NULL,
    medium TEXT NOT NULL,
    address TEXT NOT NULL,
    validated_at BIGINT NOT NULL,
    added_at BIGINT NOT NULL
);
```

### MAS-DID Storage

MAS-DID maintains its own tables for:

**trusted_vc_issuers**:
```sql
CREATE TABLE trusted_vc_issuers (
    id UUID PRIMARY KEY,
    issuer_did TEXT NOT NULL UNIQUE,
    credential_type TEXT NOT NULL,
    added_at TIMESTAMPTZ NOT NULL
);
```

Example data:
```sql
INSERT INTO trusted_vc_issuers VALUES
('...', 'did:example:newnal-registrar', 'NewnalAddressCredential', NOW());
```

**vc_revocations**:
```sql
CREATE TABLE vc_revocations (
    id UUID PRIMARY KEY,
    credential_id TEXT NOT NULL UNIQUE,
    revoked_at TIMESTAMPTZ NOT NULL,
    reason TEXT
);
```

## Security Considerations

### DID Binding Attacks

**Attack**: Attacker presents someone else's `NewnalAddressVC`

**Prevention**:
- VC's `credentialSubject.id` must match authenticated DID
- Verify during registration and 3PID add operations

### VC Replay Attacks

**Attack**: Attacker reuses intercepted VC

**Prevention**:
- VCs wrapped in Verifiable Presentations (VPs)
- VP includes challenge/nonce specific to request
- VP signature proves holder controls DID

### Address Squatting

**Attack**: User registers multiple DIDs with same Newnal Address

**Prevention**:
- Enforce address uniqueness before registration
- One address per user (multiple users cannot share)

### Revocation

**Requirement**: Support VC revocation

**Implementation Options**:
1. **Revocation List 2020**: Check status list URL in VC
2. **Status List 2021**: Bitstring-based revocation
3. **Online Status Check**: Query issuer's status endpoint

```rust
async fn check_vc_revoked(vc: &Credential) -> Result<bool> {
    if let Some(status) = &vc.credential_status {
        match status.type_.as_str() {
            "RevocationList2020Status" => {
                // Fetch and check revocation list
                check_revocation_list(status).await
            }
            "StatusList2021Entry" => {
                // Check bitstring status
                check_status_list(status).await
            }
            _ => {
                warn!("Unsupported credential status type");
                Ok(false) // Fail open or closed based on policy
            }
        }
    } else {
        Ok(false) // No revocation mechanism
    }
}
```

## Performance Optimization

### DID Lookup Caching

Cache DID → MXID mappings in Redis:

```rust
async fn lookup_user_cached(
    admin_api: &AdminApi,
    cache: &RedisPool,
    did: &str,
) -> Result<Option<String>> {
    // Check cache
    if let Some(mxid) = cache.get(format!("did_map:{}", did)).await? {
        return Ok(Some(mxid));
    }

    // Query Synapse
    let mxid = lookup_user_by_did(admin_api, did).await?;

    // Cache result (30 day TTL)
    if let Some(ref mxid) = mxid {
        cache.set_ex(
            format!("did_map:{}", did),
            mxid,
            30 * 24 * 60 * 60,
        ).await?;
    }

    Ok(mxid)
}
```

### Batch 3PID Queries

When checking multiple addresses:

```rust
async fn check_addresses_available(
    admin_api: &AdminApi,
    addresses: &[String],
) -> Result<HashMap<String, bool>> {
    let mut results = HashMap::new();

    // Batch query (if API supports)
    for address in addresses {
        let available = check_address_unique(admin_api, address).await?;
        results.insert(address.clone(), available);
    }

    Ok(results)
}
```

## Next Steps

- [Authentication Flows](./auth-flows.md): Detailed registration and login sequences
- [MLS Authentication Service](./mls-as.md): Binding DIDs to encryption keys
