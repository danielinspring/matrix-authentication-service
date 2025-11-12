# MAS-DID: Decentralized Identity Architecture

## Executive Summary

This document outlines the architectural design for **MAS-DID**, a re-engineering of the Matrix Authentication Service to support decentralized identity based on the "Newnal Web3 Telecom" specification. The goal is to replace traditional centralized authentication with a system built on Decentralized Identifiers (DIDs) and Verifiable Credentials (VCs).

## Vision

MAS-DID transforms the Matrix Authentication Service from a traditional identity provider into a **dual-purpose service**:

1. **OIDC-DID Bridge**: Translates DID-based authentication into standard OIDC tokens for Synapse (MSC3861 compatible)
2. **MLS Authentication Service**: Cryptographically binds user DIDs to MLS encryption keys for advanced E2EE

This architecture enables:
- **User sovereignty**: Users control their own identities via DIDs, not server-managed passwords
- **Privacy**: DIDs are never exposed as Matrix IDs; opaque MXIDs are used instead
- **Interoperability**: Standards-based approach using SIOPv2, OID4VP, and MLS
- **Advanced authorization**: VC-based room access control for dynamic, policy-driven permissions

## Key Design Principles

### 1. Standards-Based Approach
- **DID Core**: W3C Decentralized Identifiers
- **Verifiable Credentials**: W3C VC Data Model
- **SIOPv2**: Self-Issued OpenID Provider v2 for authentication
- **OID4VP**: OpenID for Verifiable Presentations
- **MLS (RFC 9420)**: Messaging Layer Security for E2EE
- **MIMI**: More Instant Messaging Interoperability

### 2. Privacy by Design
- DIDs are never used as Matrix user IDs
- Opaque identifiers (ULIDs) protect user privacy
- Synapse's `external_id` feature maps DIDs to MXIDs atomically

### 3. Backward Compatibility
- Maintains OIDC interface to Synapse (MSC3861)
- No changes required to Synapse deployment
- Gradual migration path from traditional auth

### 4. Extensibility
- Plugin architecture for VC verification
- Custom 3PID types (Newnal Mobile Address)
- Room-level authorization policies

## Architecture at a Glance

```
┌─────────────────────────────────────────────────────────────┐
│                         User's Wallet                        │
│                  (Self-Issued OP / SIOPv2)                   │
└────────────────────────┬────────────────────────────────────┘
                         │ DID Proof + VCs
                         │ (OID4VP)
                         ▼
┌─────────────────────────────────────────────────────────────┐
│                        MAS-DID Service                       │
│  ┌──────────────────────────────────────────────────────┐   │
│  │              Northbound: SIOPv2 RP                   │   │
│  │         (Requests DID proofs & VCs via OID4VP)       │   │
│  └──────────────────────────────────────────────────────┘   │
│                             │                                │
│                             ▼                                │
│  ┌──────────────────────────────────────────────────────┐   │
│  │         Core: DID/VC Verification Engine             │   │
│  │  • Verify DID signatures                             │   │
│  │  • Validate VCs (Newnal Address, MLS KeyPackage)     │   │
│  │  • Map DID ↔ MXID (via external_id)                  │   │
│  └──────────────────────────────────────────────────────┘   │
│                             │                                │
│                             ▼                                │
│  ┌──────────────────────────────────────────────────────┐   │
│  │            Southbound: OIDC Provider                 │   │
│  │       (Issues standard OIDC tokens to Synapse)       │   │
│  └──────────────────────────────────────────────────────┘   │
└────────────────────────┬────────────────────────────────────┘
                         │ OIDC Tokens
                         ▼
┌─────────────────────────────────────────────────────────────┐
│                      Matrix Synapse                          │
│                     (MSC3861 Delegated Auth)                 │
└─────────────────────────────────────────────────────────────┘
```

## Core Components

### 1. SIOPv2-OIDC Bridge
**Purpose**: Translate DID authentication to OIDC tokens

**Flow**:
- User initiates login via Matrix client
- MAS-DID acts as OIDC Provider (to Synapse) and SIOPv2 Relying Party (to user's wallet)
- User's wallet acts as Self-Issued OpenID Provider
- MAS-DID verifies DID control, issues OIDC token for mapped MXID

**Benefits**:
- Zero changes to Synapse
- Standards-based authentication
- User-controlled identity

### 2. Identity Mapping (external_id)
**Purpose**: Privacy-preserving DID-to-MXID mapping

**Implementation**:
- On registration: Create opaque MXID (e.g., `@01HQXYZ...:domain.com`)
- Store mapping: `did:key:z6Mk...` → `@01HQXYZ...:domain.com` in Synapse's `external_ids`
- On login: Lookup MXID via DID, issue OIDC token for found user

**Benefits**:
- DIDs never exposed in Matrix protocol
- Privacy preserved
- Atomic operations via Synapse Admin API

### 3. Newnal Mobile Address (3PID)
**Purpose**: Custom identifier system for "Newnal Web3 Telecom"

**Implementation**:
- New 3PID type: `m.id.newnal` (e.g., `+82N10...`)
- Verification via VC: Users present `NewnalAddressVC` during registration
- Custom account management UI for 3PID operations

**Benefits**:
- Decentralized verification (no SMS/email)
- Compliant with MAS 3PID exclusive control
- User-managed identity attributes

### 4. MLS Authentication Service
**Purpose**: Cryptographically bind DIDs to encryption keys

**Implementation**:
- Issue `MlsKeyPackageCredential` VCs
- VC contains: DID + MLS KeyPackage + MAS-DID signature
- Clients verify VC when adding users to E2EE groups

**Benefits**:
- Automatic trust establishment
- No manual fingerprint verification
- Cross-domain federation support (MIMI)

### 5. VC-Based Authorization
**Purpose**: Dynamic, policy-driven room access control

**Implementation**:
- New join rule: `m.join.vc`
- Room state contains OID4VP `presentation_definition`
- Users present required VCs to join restricted rooms

**Benefits**:
- Fine-grained access control
- Organizational policies enforceable
- Dynamic membership based on credentials

## Technology Stack

### Rust Ecosystem
- **Base**: Fork of existing MAS codebase
- **DID/VC Primitives**: `ssi` and `didkit` (SpruceID)
- **OID4VC Protocols**: `openid4vc` (impierce) - SIOPv2, OID4VP, OID4VCI
- **MLS**: `mls-rs` (AWS Labs) - RFC 9420 implementation with custom validation hooks

### Existing MAS Dependencies
- **Database**: PostgreSQL via `sqlx`
- **Web Framework**: Axum (Tokio ecosystem)
- **Templates**: Tera
- **Cryptography**: RustCrypto suite

## Documentation Structure

This design is detailed across multiple documents:

1. **[Requirements & Insights](./requirements.md)**: Core requirements from Newnal spec
2. **[OIDC-DID Bridge](./oidc-did-bridge.md)**: SIOPv2-OIDC bridge architecture
3. **[Identity Mapping](./identity-mapping.md)**: DID-to-MXID mapping and 3PID design
4. **[Authentication Flows](./auth-flows.md)**: Registration and login sequences
5. **[MLS Authentication Service](./mls-as.md)**: E2EE key binding design
6. **[VC-Based Authorization](./vc-authorization.md)**: Room access control
7. **[Federation & Roadmap](./federation-roadmap.md)**: MIMI integration and phased implementation

## Next Steps

1. Review and validate design with stakeholders
2. Set up development branch and toolchain
3. Begin Phase 1 implementation (MVP login flow)
4. Iterate based on testing and feedback

## References

- [MSC3861: OIDC Delegated Authentication](https://github.com/matrix-org/matrix-spec-proposals/pull/3861)
- [W3C DID Core](https://www.w3.org/TR/did-core/)
- [W3C Verifiable Credentials](https://www.w3.org/TR/vc-data-model/)
- [SIOPv2 Spec](https://openid.net/specs/openid-connect-self-issued-v2-1_0.html)
- [OID4VP Spec](https://openid.net/specs/openid-4-verifiable-presentations-1_0.html)
- [RFC 9420: MLS Protocol](https://www.rfc-editor.org/rfc/rfc9420)
- [MIMI Working Group](https://datatracker.ietf.org/wg/mimi/about/)
