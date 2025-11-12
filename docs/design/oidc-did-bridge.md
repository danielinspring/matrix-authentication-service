# OIDC-DID Bridge Architecture

## Overview

The OIDC-DID Bridge is the core architectural pattern that enables MAS-DID to translate decentralized identity (DID/VC) authentication into standard OpenID Connect tokens that Matrix Synapse expects.

This pattern **inverts** the traditional OIDC flow by making the user's wallet the identity provider and MAS-DID the relying party, while simultaneously presenting MAS-DID as an OIDC provider to Synapse.

## The Bridge Pattern

### Traditional OIDC Flow (Current MAS)

```
┌────────┐              ┌────────┐              ┌─────────┐
│ Client │─────────────▶│  MAS   │─────────────▶│ Synapse │
└────────┘              │  (RP)  │              └─────────┘
    │                   └────────┘
    │                        │
    │                        ▼
    │                   ┌──────────┐
    └──────────────────▶│ Upstream │
                        │   IdP    │
                        │  (OP)    │
                        └──────────┘
```

In this model:
- **Upstream IdP** = OpenID Provider (OP)
- **MAS** = Relying Party (RP)
- User authenticates to upstream IdP, MAS receives tokens

### MAS-DID Bridge Pattern (Inverted)

```
┌────────┐              ┌──────────────┐              ┌─────────┐
│ Client │─────────────▶│   MAS-DID    │─────────────▶│ Synapse │
└────────┘              │              │              └─────────┘
    │                   │ Northbound:  │
    │                   │  SIOPv2 RP   │
    │                   │  OID4VP Ver. │
    │                   │              │
    │                   │ Southbound:  │
    │                   │  OIDC OP     │
    │                   └──────────────┘
    │                         ▲
    │                         │
    │     DID Proofs + VCs    │
    ▼                         │
┌─────────────────────────────┘
│  User's Wallet
│  (Self-Issued OP)
│  - Holds private keys
│  - Signs auth responses
│  - Presents VCs
└─────────────────────────────┘
```

In this model:
- **User's Wallet** = Self-Issued OpenID Provider (SIOPv2)
- **MAS-DID Northbound** = Relying Party (RP) + OID4VP Verifier
- **MAS-DID Southbound** = OpenID Provider (OP) to Synapse

## SIOPv2: Self-Issued OpenID Provider v2

### What is SIOPv2?

SIOPv2 is an OpenID Connect extension where the user's own wallet acts as the identity provider. Instead of delegating to a centralized IdP, the user **self-issues** authentication responses signed with their own keys.

**Key Properties**:
- User controls private keys (in wallet/app)
- No centralized IdP required
- Authentication = proof of key possession
- Perfect fit for DID-based systems

### SIOPv2 Flow

```
1. Client initiates login
   ↓
2. MAS-DID generates authorization request with:
   - response_type: "id_token"
   - client_id: MAS-DID's DID
   - nonce: cryptographic challenge
   - registration: { id_token_signed_response_alg: ["ES256"] }
   ↓
3. Request presented as QR code or deep link
   ↓
4. User's wallet scans QR code
   ↓
5. Wallet displays auth request to user
   ↓
6. User approves
   ↓
7. Wallet generates self-signed ID token:
   {
     "iss": "did:key:z6MkUserDID...",      // Issuer is user's DID
     "sub": "did:key:z6MkUserDID...",      // Subject is user's DID
     "aud": "did:key:z6MkMASDID...",       // Audience is MAS-DID
     "nonce": "original_nonce",            // Replay protection
     "iat": 1234567890,
     "exp": 1234567900
   }
   ↓
8. Wallet signs token with user's DID private key
   ↓
9. Wallet sends response to MAS-DID callback URL
   ↓
10. MAS-DID verifies:
    - Token signature (against DID document public key)
    - Nonce matches
    - Expiration valid
    - Audience is MAS-DID
    ↓
11. Authentication succeeds: user proved control of DID
```

## OID4VP: OpenID for Verifiable Presentations

### What is OID4VP?

OID4VP extends SIOPv2 to request **Verifiable Credentials** (VCs) in addition to authentication. MAS-DID uses this to request proof of Newnal Address ownership, organizational credentials, etc.

### OID4VP Flow

```
1. MAS-DID generates authorization request with:
   - All SIOPv2 parameters PLUS
   - presentation_definition: {
       "id": "newnal-registration",
       "input_descriptors": [{
         "id": "newnal_address",
         "constraints": {
           "fields": [{
             "path": ["$.type"],
             "filter": { "const": "NewnalAddressCredential" }
           }]
         }
       }]
     }
   ↓
2. User's wallet sees presentation request
   ↓
3. Wallet searches user's VCs for matching credential
   ↓
4. Wallet displays: "MAS-DID requests your Newnal Address"
   ↓
5. User approves
   ↓
6. Wallet creates Verifiable Presentation (VP):
   {
     "@context": ["https://www.w3.org/2018/credentials/v1"],
     "type": ["VerifiablePresentation"],
     "holder": "did:key:z6MkUserDID...",
     "verifiableCredential": [{
       "@context": ["https://www.w3.org/2018/credentials/v1"],
       "type": ["VerifiableCredential", "NewnalAddressCredential"],
       "issuer": "did:example:newnal-registrar",
       "credentialSubject": {
         "id": "did:key:z6MkUserDID...",
         "newnalAddress": "+82N1012345678"
       },
       "proof": { ... }  // VC issuer's signature
     }],
     "proof": { ... }  // User's signature over VP
   }
   ↓
7. Wallet includes VP in authentication response (vp_token)
   ↓
8. MAS-DID verifies:
   - VP signature (holder's signature)
   - VC signature (issuer's signature)
   - VC not expired
   - Credential subject DID matches authenticated DID
   - VC meets presentation_definition requirements
   ↓
9. MAS-DID extracts Newnal Address: "+82N1012345678"
```

## Northbound Interface: SIOPv2 RP + OID4VP Verifier

### Responsibilities

1. **Generate Authorization Requests**
   - Create SIOPv2 requests with proper nonces
   - Include presentation_definition when VCs required
   - Render as QR codes or deep links

2. **Verify Authentication Responses**
   - Validate self-issued ID token signatures
   - Check nonce, expiration, audience
   - Resolve and verify DIDs

3. **Verify Verifiable Presentations**
   - Validate VP holder signature
   - Validate embedded VC issuer signatures
   - Check VC expiration and revocation
   - Verify presentation matches requested definition

4. **Extract User Information**
   - DID from self-issued token
   - Credentials from VPs (e.g., Newnal Address)
   - Additional claims as needed

### Key Endpoints

#### `GET /auth/siop/authorize`
Initiate SIOPv2 authentication

**Parameters**:
- `presentation_definition` (optional): JSON object specifying required VCs
- `redirect_uri`: Where to redirect after authentication

**Response**:
- Renders HTML page with QR code
- QR code contains: `openid://?request_uri=https://mas-did.example.com/auth/siop/request/abc123`

#### `GET /auth/siop/request/{request_id}`
Retrieve request object by reference (for mobile wallets)

**Response**:
```json
{
  "response_type": "id_token",
  "client_id": "did:key:z6MkMASDID...",
  "redirect_uri": "https://mas-did.example.com/auth/siop/callback",
  "nonce": "abc123xyz789",
  "scope": "openid",
  "response_mode": "post",
  "registration": {
    "id_token_signed_response_alg": ["ES256", "EdDSA"]
  },
  "presentation_definition": { ... }
}
```

#### `POST /auth/siop/callback`
Receive authentication response from wallet

**Request Body** (form-encoded):
- `id_token`: Self-issued JWT
- `vp_token` (optional): Verifiable Presentation JWT

**Processing**:
1. Verify `id_token` signature against DID document
2. Verify `vp_token` if present
3. Store verified DID and credentials in session
4. Redirect to original `redirect_uri`

### Implementation with `openid4vc` Crate

```rust
use openid4vc::siop::{RelyingParty, AuthorizationRequest};
use openid4vc::oid4vp::{PresentationDefinition, InputDescriptor};

// Initialize Relying Party
let rp = RelyingParty::new(
    "did:key:z6MkMASDID...",  // MAS-DID's DID
    "https://mas-did.example.com",
).await?;

// Create presentation definition for Newnal Address
let presentation_def = PresentationDefinition {
    id: "newnal-registration".to_string(),
    input_descriptors: vec![
        InputDescriptor {
            id: "newnal_address".to_string(),
            constraints: Constraints {
                fields: vec![
                    Field {
                        path: vec!["$.type".to_string()],
                        filter: json!({"const": "NewnalAddressCredential"}),
                    }
                ],
            },
        }
    ],
};

// Create authorization request
let auth_req = rp.create_authorization_request(
    AuthorizationRequestParams {
        presentation_definition: Some(presentation_def),
        nonce: generate_nonce(),
        redirect_uri: "https://mas-did.example.com/auth/siop/callback",
    }
).await?;

// Generate QR code with request URI
let request_uri = format!(
    "openid://?request_uri=https://mas-did.example.com/auth/siop/request/{}",
    auth_req.id
);
```

## Southbound Interface: OIDC Provider

### Responsibilities

1. **Provide OIDC Discovery**
   - `.well-known/openid-configuration`
   - JWKS endpoint for token verification

2. **Handle Authorization Code Flow**
   - Standard OIDC authorization endpoint
   - Token exchange endpoint

3. **Issue OIDC Tokens**
   - ID tokens with Matrix-specific claims
   - Access tokens for Synapse API

4. **Maintain MSC3861 Compatibility**
   - All claims expected by Synapse
   - Token format and signing requirements

### Key Endpoints

#### `GET /.well-known/openid-configuration`
OIDC discovery document

**Response**:
```json
{
  "issuer": "https://mas-did.example.com",
  "authorization_endpoint": "https://mas-did.example.com/authorize",
  "token_endpoint": "https://mas-did.example.com/oauth2/token",
  "jwks_uri": "https://mas-did.example.com/oauth2/keys.json",
  "response_types_supported": ["code"],
  "grant_types_supported": ["authorization_code", "refresh_token"],
  "subject_types_supported": ["public"],
  "id_token_signing_alg_values_supported": ["RS256"],
  "scopes_supported": ["openid", "urn:matrix:org.matrix.msc2967.client:api:*"]
}
```

#### `GET /authorize`
Standard OIDC authorization endpoint

**Flow**:
1. Synapse redirects user to MAS-DID with OIDC parameters
2. MAS-DID checks for existing session
3. If no session: redirect to SIOPv2 flow (northbound)
4. After SIOPv2 completes: map DID to MXID
5. Generate authorization code
6. Redirect back to Synapse with code

#### `POST /oauth2/token`
Token exchange endpoint

**Request**:
```
grant_type=authorization_code
&code=xyz...
&redirect_uri=https://synapse.example.com/_synapse/client/oidc/callback
&client_id=synapse_client
&client_secret=secret
```

**Response**:
```json
{
  "access_token": "syt_...",
  "token_type": "Bearer",
  "expires_in": 3600,
  "refresh_token": "syr_...",
  "id_token": "eyJhbGc..."
}
```

**ID Token Claims**:
```json
{
  "iss": "https://mas-did.example.com",
  "sub": "01HQXYZ789...",  // Opaque MXID (without @domain)
  "aud": "synapse_client",
  "iat": 1234567890,
  "exp": 1234567900,
  "nonce": "...",
  "preferred_username": "alice",  // If available
  "email": "+82N1012345678",      // Newnal Address in email field
  "email_verified": true
}
```

### Mapping Layer

Between northbound and southbound, MAS-DID performs critical mapping:

```
┌─────────────────────────────────────────┐
│         Northbound Input                │
│  • DID: did:key:z6MkUserDID...         │
│  • Newnal Address: +82N1012345678      │
│  • VCs: [NewnalAddressCredential]      │
└────────────────┬────────────────────────┘
                 │
                 ▼
     ┌───────────────────────┐
     │   Mapping Logic       │
     │                       │
     │ 1. Lookup MXID by DID │
     │    (external_id API)  │
     │                       │
     │ 2. If new user:       │
     │    - Create MXID      │
     │    - Store mapping    │
     │    - Add 3PIDs        │
     │                       │
     │ 3. Build OIDC claims  │
     └───────────┬───────────┘
                 │
                 ▼
┌─────────────────────────────────────────┐
│         Southbound Output               │
│  • sub: 01HQXYZ789...                  │
│  • email: +82N1012345678               │
│  • email_verified: true                │
└─────────────────────────────────────────┘
```

## Security Considerations

### DID Verification

**Must verify**:
- DID document is well-formed
- DID method is supported (e.g., `did:key`)
- Public key in DID document matches signature
- DID not on revocation list (if applicable)

**DID Resolution**:
```rust
use ssi::did::{DIDResolver, DIDMethod};
use ssi::did_resolve::ResolutionInputMetadata;

async fn verify_did_signature(
    did: &str,
    message: &[u8],
    signature: &[u8],
) -> Result<bool> {
    // Resolve DID to DID document
    let did_resolver = DIDResolver::default();
    let (res_meta, doc, _) = did_resolver
        .resolve(did, &ResolutionInputMetadata::default())
        .await;

    if res_meta.error.is_some() {
        return Err(anyhow!("DID resolution failed"));
    }

    // Extract verification method
    let verification_method = doc
        .verification_method
        .first()
        .ok_or(anyhow!("No verification method"))?;

    // Verify signature
    verification_method.verify(message, signature).await
}
```

### VC Verification

**Must verify**:
- VC issuer is trusted
- VC signature valid (issuer's signature)
- VP signature valid (holder's signature)
- VC not expired
- VC not revoked
- Credential subject DID matches authenticated DID

**Trust Model**:
- MAS-DID maintains list of trusted VC issuers
- For Newnal Addresses: trust Newnal registrar DIDs
- For org credentials: trust org-specific issuers
- Support for configurable trust registries

### Nonce Management

**Requirements**:
- Generate cryptographically random nonces
- Store nonces with expiration (e.g., 10 minutes)
- Single use only (prevent replay attacks)
- Associate with session to prevent token substitution

### Replay Prevention

**Mechanisms**:
- Nonce verification (as above)
- Check token issuance time (`iat` claim)
- Require token expiration (`exp` claim)
- Reject tokens issued in future

## Performance Considerations

### DID Caching

DID documents should be cached to avoid repeated resolution:

```rust
use lru::LruCache;

struct DIDCache {
    cache: LruCache<String, DIDDocument>,
    ttl: Duration,
}

impl DIDCache {
    async fn resolve(&mut self, did: &str) -> Result<DIDDocument> {
        if let Some(doc) = self.cache.get(did) {
            return Ok(doc.clone());
        }

        let doc = resolve_did(did).await?;
        self.cache.put(did.to_string(), doc.clone());
        Ok(doc)
    }
}
```

### Parallel Verification

When multiple VCs presented, verify in parallel:

```rust
use tokio::task::JoinSet;

async fn verify_vcs(vcs: Vec<VC>) -> Result<Vec<VerifiedVC>> {
    let mut set = JoinSet::new();

    for vc in vcs {
        set.spawn(async move {
            verify_vc(vc).await
        });
    }

    let mut verified = Vec::new();
    while let Some(result) = set.join_next().await {
        verified.push(result??);
    }

    Ok(verified)
}
```

## Error Handling

### User-Facing Errors

When authentication fails, provide clear error messages:

- **DID Verification Failed**: "Unable to verify your identity. Please check your wallet is using a supported DID method."
- **VC Missing**: "Registration requires a Newnal Address credential. Please obtain one from the registrar."
- **VC Expired**: "Your Newnal Address credential has expired. Please obtain a new one."
- **VC Invalid**: "Unable to verify your credential. Please contact the issuer."

### Logging and Monitoring

Log key events for debugging and security monitoring:

```rust
use tracing::{info, warn, error};

// Successful auth
info!(
    did = %user_did,
    mxid = %matrix_id,
    "User authenticated successfully"
);

// Verification failure
warn!(
    did = %user_did,
    reason = "signature_invalid",
    "DID verification failed"
);

// Security event
error!(
    did = %user_did,
    nonce = %nonce,
    "Replay attack detected"
);
```

## Next Steps

- [Identity Mapping & 3PID Design](./identity-mapping.md)
- [Authentication Flows](./auth-flows.md)
