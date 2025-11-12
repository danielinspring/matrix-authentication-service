# Core Requirements & Key Insights

## Overview

This document details the core requirements derived from the "Newnal Web3 Telecom" specification and the architectural insights that drive the MAS-DID design.

## Requirements from Newnal Specification

### Requirement 1: User-Owned ID ("Web3 ai-ID")

**Statement**: Users must authenticate using Decentralized Identifiers (DIDs) and Verifiable Credentials (VCs) that they control, not with server-managed passwords.

**Rationale**:
- **User Sovereignty**: Users own and control their identity, not the service provider
- **Portability**: DIDs can be used across multiple services
- **Security**: Private keys remain in user's wallet; server never sees them
- **Privacy**: Users can use different DIDs for different contexts

**Technical Implications**:
- No password storage or management required
- Authentication becomes proof-of-key-possession
- User's wallet becomes primary authentication device
- Service must support DID resolution and verification

**DID Methods**:
For MVP, we will support:
- `did:key` - Simple, self-contained DIDs derived from public keys
- Future: `did:web`, `did:ion`, or other methods as needed

**Example Flow**:
```
1. User initiates login
2. MAS-DID generates authentication challenge
3. User's wallet signs challenge with DID's private key
4. MAS-DID verifies signature against DID document's public key
5. Authentication succeeds if signature valid
```

### Requirement 2: New Identifier ("Newnal Mobile Address")

**Statement**: A new, user-creatable identifier called the "Newnal Mobile Address" (e.g., `+82N10...`) must be supported as a Matrix Third-Party ID (3PID).

**Rationale**:
- **Web3 Telecom Integration**: Bridges blockchain-based telecom systems with Matrix
- **User-Friendly**: Familiar phone-number-like format
- **Decentralized**: Not tied to traditional telcos
- **Flexible**: Users can create and manage multiple addresses

**Technical Implications**:
- Must extend Matrix 3PID system with new medium type: `m.id.newnal`
- Cannot use traditional SMS verification (decentralized)
- Verification achieved via Verifiable Credentials
- Requires custom account management UI (MAS disables client 3PID UI)

**3PID Structure**:
```json
{
  "medium": "m.id.newnal",
  "address": "+82N1012345678"
}
```

**Verification Credential**:
```json
{
  "@context": ["https://www.w3.org/2018/credentials/v1"],
  "type": ["VerifiableCredential", "NewnalAddressCredential"],
  "issuer": "did:example:newnal-registrar",
  "credentialSubject": {
    "id": "did:key:z6MkUserDID...",
    "newnalAddress": "+82N1012345678"
  }
}
```

**User Experience**:
1. User obtains Newnal Address from separate registrar service
2. Registrar issues `NewnalAddressVC` to user's wallet
3. During MAS-DID registration, user presents VC
4. MAS-DID verifies VC and associates address with user's MXID

### Requirement 3: Advanced E2EE (MLS & MIMI)

**Statement**: The system must use Messaging Layer Security (MLS) for end-to-end encryption and MIMI for interoperability.

**Rationale**:
- **Modern Cryptography**: MLS is IETF standard (RFC 9420) for group E2EE
- **Performance**: Better scalability than current Matrix E2EE for large groups
- **Interoperability**: MIMI enables cross-platform messaging
- **Security**: Continuous key rotation, post-compromise security

**Technical Implications**:
- Must integrate MLS protocol stack into Matrix
- Requires binding user identity (DID) to MLS keys
- MAS-DID must act as MLS Authentication Service
- Federation requires cross-domain key verification

**MLS Key Concepts**:
- **KeyPackage**: Pre-generated public key material for adding users to groups
- **Credentials**: Bind identity to keys (this is where DIDs come in)
- **Authentication Service (AS)**: Issues and validates credentials

**Why MAS-DID Must Be MLS AS**:
Without an AS, MLS groups face an identity problem:
- Alice has a KeyPackage with public key `K_a`
- Bob wants to add Alice to group
- How does Bob know `K_a` belongs to Alice?

**Traditional Solution**: Manual fingerprint verification (doesn't scale)

**MAS-DID Solution**:
1. MAS-DID issues `MlsKeyPackageCredential` VC
2. VC cryptographically binds: `did:key:Alice` → `K_a` → `@alice:domain.com`
3. Bob verifies VC signature from trusted MAS-DID
4. Bob gains cryptographic proof that key belongs to Alice

## Key Architectural Conclusion

From these three requirements, we derive a critical architectural insight:

### MAS-DID Must Serve Two Distinct Roles

#### Role 1: OIDC-DID Bridge

**Purpose**: Translate DID authentication into OIDC tokens for Synapse

**Why**:
- Matrix Synapse expects standard OIDC provider (MSC3861)
- Users authenticate with DIDs, not OIDC
- Bridge pattern reconciles these two worlds

**Behavior**:
- **Southbound** (to Synapse): Acts as OIDC Provider (OP)
- **Northbound** (to Wallet): Acts as SIOPv2 Relying Party (RP)

#### Role 2: MLS Authentication Service

**Purpose**: Cryptographically bind DIDs to MLS encryption keys

**Why**:
- MLS requires trusted authority to validate identity-to-key bindings
- Without this, MLS groups have no identity assurance
- Enables automatic trust without manual verification

**Behavior**:
- Issues `MlsKeyPackageCredential` VCs
- Verifies key ownership during credential issuance
- Provides verification endpoint for clients

### Architecture Diagram

```
┌────────────────────────────────────────────────────────────┐
│                      MAS-DID Service                       │
│                                                            │
│  ┌──────────────────────────────────────────────────────┐ │
│  │              Role 1: OIDC-DID Bridge                 │ │
│  │                                                      │ │
│  │  Northbound: SIOPv2 RP + OID4VP Verifier            │ │
│  │       ↕                                              │ │
│  │  Southbound: OIDC Provider (MSC3861)                │ │
│  └──────────────────────────────────────────────────────┘ │
│                                                            │
│  ┌──────────────────────────────────────────────────────┐ │
│  │         Role 2: MLS Authentication Service           │ │
│  │                                                      │ │
│  │  • Issue MlsKeyPackageCredential VCs                │ │
│  │  • Bind DIDs to MLS KeyPackages                     │ │
│  │  • Provide credential verification endpoint         │ │
│  └──────────────────────────────────────────────────────┘ │
│                                                            │
│  ┌──────────────────────────────────────────────────────┐ │
│  │              Shared Core Components                  │ │
│  │                                                      │ │
│  │  • DID Resolution & Verification                    │ │
│  │  • VC Validation Engine                             │ │
│  │  • DID ↔ MXID Mapping (external_id)                 │ │
│  │  • Storage Layer (PostgreSQL)                       │ │
│  └──────────────────────────────────────────────────────┘ │
└────────────────────────────────────────────────────────────┘
```

## Design Constraints

### Constraint 1: MSC3861 Compatibility
**Implication**: MAS-DID's southbound interface must be standard OIDC
- Cannot modify Synapse
- Must issue compliant OIDC tokens
- Must support standard OIDC discovery

### Constraint 2: MAS 3PID Exclusive Control
**Implication**: When MAS controls authentication, clients cannot modify 3PIDs
- Must provide separate account management web UI
- 3PID operations via custom admin endpoints
- Users manage Newnal Addresses through dedicated interface

### Constraint 3: Privacy Requirements
**Implication**: DIDs must never be exposed in Matrix protocol
- Use opaque MXIDs (ULIDs)
- DID mapping kept server-side only
- Client never sees DID in Matrix context

### Constraint 4: Federation Support
**Implication**: Design must support cross-domain scenarios
- MLS credentials must be verifiable across servers
- DID resolution must work globally
- Trust model must support federated AS

## Success Criteria

A successful MAS-DID implementation must:

1. ✅ **Authenticate users via DIDs** without passwords
2. ✅ **Map DIDs to MXIDs** using `external_id` atomically
3. ✅ **Support Newnal Mobile Address** as custom 3PID
4. ✅ **Issue OIDC tokens** compatible with MSC3861
5. ✅ **Bind DIDs to MLS keys** via MlsKeyPackageCredential VCs
6. ✅ **Preserve privacy** by hiding DIDs from Matrix protocol
7. ✅ **Enable federation** through MIMI/MLS interoperability
8. ✅ **Provide account management UI** for 3PID operations

## Non-Requirements

For clarity, the following are explicitly **not** requirements:

- ❌ Supporting traditional username/password authentication
- ❌ Backward compatibility with pre-MSC3861 Synapse
- ❌ General-purpose identity provider features
- ❌ Support for SAML, LDAP, or other protocols (use separate IdP bridge)
- ❌ Traditional 2FA (TOTP, WebAuthn) - authentication is already key-based
- ❌ Using DID as Matrix user ID localpart

## Risks and Mitigations

### Risk 1: DID Resolution Availability
**Risk**: DID resolution may fail if external services unavailable

**Mitigation**:
- Prioritize `did:key` (self-contained, no resolution needed)
- Cache resolved DID documents
- Implement fallback mechanisms

### Risk 2: VC Revocation
**Risk**: Issued VCs may need revocation (e.g., compromised key)

**Mitigation**:
- Implement revocation list checking
- Support short-lived VCs with re-issuance
- Monitor W3C VC revocation standards

### Risk 3: MLS Adoption Timeline
**Risk**: Matrix MLS support may take time to mature

**Mitigation**:
- Phase MLS AS implementation after core auth (Phase 3)
- Design as optional enhancement
- Provide gradual migration path

### Risk 4: Wallet UX Complexity
**Risk**: Users may struggle with wallet-based authentication

**Mitigation**:
- Provide clear onboarding documentation
- Support QR code flows for mobile wallets
- Design fallback recovery mechanisms

## Next Steps

With requirements established, proceed to:
1. [OIDC-DID Bridge Architecture](./oidc-did-bridge.md)
2. [Identity Mapping Design](./identity-mapping.md)
3. [Authentication Flows](./auth-flows.md)
