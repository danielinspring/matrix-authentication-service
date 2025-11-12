# Federation & Implementation Roadmap

## Overview

This document describes how MAS-DID enables federated identity across Matrix homeservers and outlines a phased implementation plan for building the complete system.

## Federation Architecture

### The Federation Challenge

Traditional Matrix federation handles **message routing**. MAS-DID adds **identity federation**:

```
Server A (domain-a.com)          Server B (domain-b.com)
┌─────────────────────┐          ┌─────────────────────┐
│ Alice               │          │ Bob                 │
│ DID: did:key:A...   │────────▶ │ DID: did:key:B...   │
│ MXID: @alice:a.com  │  Trust?  │ MXID: @bob:b.com    │
└─────────────────────┘          └─────────────────────┘
        │                                │
        ▼                                ▼
┌─────────────────────┐          ┌─────────────────────┐
│ MAS-DID (A)         │          │ MAS-DID (B)         │
│ Issuer: did:web:a   │◀────────▶│ Issuer: did:web:b   │
└─────────────────────┘   Trust   └─────────────────────┘
```

**Questions to Answer**:
1. How does Alice verify Bob's DID?
2. How does Alice trust Bob's MLS KeyPackage?
3. How does Alice verify Bob's room access VCs?

### Trust Model

**Principle**: Transitivity of Trust

```
Alice trusts server-a.com
→ Alice trusts server-a's MAS-DID
→ Alice trusts VCs issued by server-a's MAS-DID

Bob trusts server-b.com
→ Bob trusts server-b's MAS-DID
→ Bob trusts VCs issued by server-b's MAS-DID

For cross-domain trust:
If Alice's server trusts Bob's server
→ Alice can trust VCs from Bob's MAS-DID
```

### Trust Registry

Each MAS-DID maintains a **trust registry** of other servers:

```rust
pub struct FederatedTrustRegistry {
    // Domain → Trusted MAS-DID
    trusted_servers: HashMap<String, TrustedServer>,
    // Automatic trust for same-deployment servers
    auto_trust_enabled: bool,
}

pub struct TrustedServer {
    domain: String,
    mas_did: String,
    added_at: DateTime<Utc>,
    verified: bool,
    trust_level: TrustLevel,
}

pub enum TrustLevel {
    Full,        // Trust all credentials
    Limited,     // Trust only specific credential types
    Verify,      // Trust but verify on each use
}
```

### Federation Discovery

#### .well-known/matrix/mas-did

Each server advertises its MAS-DID configuration:

**URL**: `https://domain.com/.well-known/matrix/mas-did`

**Response**:
```json
{
  "did": "did:web:domain.com",
  "version": "1.0",
  "capabilities": {
    "siop_v2": true,
    "oid4vp": true,
    "mls_as": true,
    "vc_authorization": true
  },
  "endpoints": {
    "issuer": "https://mas-did.domain.com",
    "mls_credential": "https://mas-did.domain.com/mls/credential/user/{mxid}",
    "vc_verification": "https://mas-did.domain.com/vc/verify"
  },
  "supported_did_methods": ["did:key", "did:web"],
  "supported_vc_types": [
    "MlsKeyPackageCredential",
    "NewnalAddressCredential"
  ]
}
```

**Discovery Flow**:
```rust
async fn discover_server_mas_did(domain: &str) -> Result<ServerConfig> {
    let url = format!("https://{}/.well-known/matrix/mas-did", domain);

    let response = reqwest::get(&url).await?;

    if response.status() != 200 {
        return Err(anyhow!("Server does not support MAS-DID"));
    }

    let config: ServerConfig = response.json().await?;

    // Verify DID resolves
    let resolver = DIDResolver::default();
    resolver.resolve(&config.did, &Default::default()).await?;

    Ok(config)
}
```

### Cross-Domain MLS Authentication

**Scenario**: Alice@server-a wants to add Bob@server-b to encrypted group

**Flow**:
```
1. Alice's client requests Bob's MLS credentials
   ↓
2. Alice's MAS-DID checks trust registry
   ↓
3. If server-b trusted, query Bob's credentials
   GET https://server-b.com/.../mls/credential/user/@bob:server-b.com
   ↓
4. Server-b returns MlsKeyPackageCredential VC
   {
     "issuer": "did:web:server-b.com",
     "credentialSubject": {
       "id": "did:key:Bob...",
       "mlsKeyPackage": "..."
     }
   }
   ↓
5. Alice's client verifies VC
   - Signature from server-b's MAS-DID ✓
   - Server-b is trusted ✓
   - VC not expired ✓
   - VC not revoked ✓
   ↓
6. Alice adds Bob to group using verified KeyPackage
```

**Implementation**:
```rust
async fn get_federated_mls_credential(
    local_mas: &MasDIDService,
    remote_mxid: &str,
) -> Result<MlsCredential> {
    // Parse domain from MXID
    let domain = extract_domain(remote_mxid)?;

    // Check trust registry
    if !local_mas.trust_registry.is_trusted(domain).await? {
        return Err(anyhow!("Server {} not trusted", domain));
    }

    // Discover remote MAS-DID endpoints
    let server_config = discover_server_mas_did(domain).await?;

    // Fetch credential
    let url = server_config.endpoints.mls_credential
        .replace("{mxid}", remote_mxid);

    let response = reqwest::get(&url).await?;
    let credential: MlsCredential = response.json().await?;

    // Verify credential
    verify_credential_from_trusted_server(
        &credential,
        &server_config.did,
    ).await?;

    Ok(credential)
}
```

### Cross-Domain VC Authorization

**Scenario**: Room on server-a requires EmployeeCredential. Bob@server-b wants to join.

**Flow**:
```
1. Bob presents EmployeeCredential issued by employer.example
   ↓
2. Server-a's MAS-DID verifies:
   - VC signature from employer.example ✓
   - employer.example in trusted issuers list ✓
   - VC meets room requirements ✓
   ↓
3. Server-a joins Bob to room
```

**Key Point**: VC issuers are **independent** of homeserver domains. A user from any server can present VCs from trusted issuers.

## MIMI Integration

### What is MIMI?

**More Instant Messaging Interoperability (MIMI)** is an IETF working group standardizing cross-platform messaging protocols.

**Goals**:
- Different messaging platforms can interoperate
- End-to-end encryption across platforms
- Federation of identity and groups

### MAS-DID as MIMI Identity Provider

MAS-DID's architecture naturally supports MIMI:

```
┌──────────────┐         ┌──────────────┐         ┌──────────────┐
│   Matrix     │         │   WhatsApp   │         │   Signal     │
│   Server     │         │   Server     │         │   Server     │
└──────┬───────┘         └──────┬───────┘         └──────┬───────┘
       │                        │                        │
       │ MLS + Identity         │ MLS + Identity         │ MLS + Identity
       ▼                        ▼                        ▼
┌──────────────┐         ┌──────────────┐         ┌──────────────┐
│  MAS-DID (A) │◀───────▶│  MAS-DID (B) │◀───────▶│  MAS-DID (C) │
│              │  Trust  │              │  Trust  │              │
│ Issues VCs   │         │ Issues VCs   │         │ Issues VCs   │
└──────────────┘         └──────────────┘         └──────────────┘
```

**How MAS-DID Enables MIMI**:

1. **Universal Identity**: DIDs work across platforms
2. **MLS Authentication**: MlsKeyPackageCredential VCs provide cross-platform key verification
3. **Federated Trust**: Trust registries enable cross-platform identity verification

### MIMI MLS Profile Compliance

**MIMI Requirements**:
- Use MLS for E2EE groups
- Support authentication of group members
- Handle cross-domain group membership

**MAS-DID Provides**:
- ✅ MLS AS for key authentication
- ✅ DID-based identity
- ✅ VC-based credentials for federated verification
- ✅ Trust registry for cross-domain scenarios

### Example: Matrix ↔ WhatsApp Group

```
Alice@matrix.org and Bob@whatsapp.example in same MLS group

1. Alice's Matrix client generates MLS KeyPackage
   ↓
2. Matrix MAS-DID issues MlsKeyPackageCredential
   {
     "issuer": "did:web:matrix.org",
     "credentialSubject": {
       "id": "did:key:Alice...",
       "mlsKeyPackage": "..."
     }
   }
   ↓
3. Bob's WhatsApp client fetches Alice's credential
   ↓
4. WhatsApp MAS-DID verifies credential
   - Trusts matrix.org MAS-DID ✓
   - Verifies signature ✓
   ↓
5. Bob's client adds Alice to group with confidence in identity
```

## Phased Implementation Roadmap

### Phase 1: MVP - Existing User Login (3 months)

**Goal**: Prove core OIDC-DID Bridge concept

**Features**:
- ✅ SIOPv2 authentication
- ✅ DID verification (did:key only)
- ✅ External ID mapping
- ✅ OIDC token issuance
- ✅ Basic session management

**Deliverables**:
1. Fork MAS codebase
2. Integrate `ssi`, `didkit`, `openid4vc` crates
3. Implement SIOPv2 RP endpoints
4. Implement DID verification logic
5. Implement Synapse Admin API integration for external_id lookup
6. Implement OIDC token issuance with DID-mapped MXIDs
7. Create QR code-based auth UI
8. Write integration tests

**Success Criteria**:
- User with existing DID can log into Element
- No password required
- OIDC tokens correctly issued to Synapse
- Sessions persist across page reloads

**Technical Risks**:
- `openid4vc` crate maturity
- DID resolution reliability
- Integration complexity with existing MAS

### Phase 2: Registration & Newnal Address (2 months)

**Goal**: Complete identity lifecycle with custom 3PID

**Features**:
- ✅ New user registration
- ✅ Opaque MXID generation (ULIDs)
- ✅ NewnalAddressVC verification
- ✅ 3PID management UI
- ✅ VC issuance for Newnal Addresses

**Deliverables**:
1. Registration flow with OID4VP
2. NewnalAddressVC schema and verification
3. Atomic user creation (MXID + external_id + 3PID)
4. Account management web UI
5. Add/remove Newnal Address endpoints
6. 3PID uniqueness enforcement
7. Trusted issuer configuration

**Success Criteria**:
- New users can register with DID + NewnalAddressVC
- Newnal Addresses appear in Matrix clients
- Users can manage addresses via account UI
- No duplicate Newnal Addresses allowed

**Technical Risks**:
- Synapse Admin API limitations
- VC verification performance
- Trust model for Newnal registrars

### Phase 3: MLS Authentication Service (4 months)

**Goal**: Enable secure group E2EE with automatic trust

**Features**:
- ✅ MLS KeyPackage credential issuance
- ✅ VC-based MLS identity verification
- ✅ `mls-rs` integration with custom validator
- ✅ Credential discovery endpoints
- ✅ Revocation support

**Deliverables**:
1. MlsKeyPackageCredential schema
2. Credential issuance flow
3. `DIDIdentityProvider` implementation for `mls-rs`
4. Credential verification logic
5. Discovery endpoint for user credentials
6. Status List 2021 revocation
7. Federation discovery (.well-known)
8. Cross-domain credential verification

**Success Criteria**:
- Users can generate MLS credentials
- Clients can add users to MLS groups with verified keys
- Revoked credentials rejected
- Cross-domain credentials verified

**Technical Risks**:
- MLS protocol complexity
- `mls-rs` custom validation hooks
- Performance of VC verification in MLS flows
- Matrix MLS adoption timeline

### Phase 4: VC-Based Authorization (3 months)

**Goal**: Dynamic room access control via VCs

**Features**:
- ✅ m.join.vc join rule
- ✅ Presentation definition evaluation
- ✅ Admin-forced join after VC verification
- ✅ Multiple VC types support
- ✅ Periodic re-verification (optional)

**Deliverables**:
1. Join rule specification (MSC)
2. Presentation evaluation engine
3. Room join with VC endpoint
4. Client integration guide
5. Admin tools for setting VC requirements
6. Template presentation definitions
7. Re-verification worker (optional)

**Success Criteria**:
- Rooms can require specific VCs
- Users can join by presenting required VCs
- Multiple credential types supported
- Privacy-preserving (selective disclosure)

**Technical Risks**:
- Matrix client adoption
- Presentation definition complexity
- Privacy implications of VC storage
- Performance of verification at scale

### Phase 5: Production Hardening (2 months)

**Goal**: Production-ready system

**Features**:
- ✅ Performance optimization
- ✅ Monitoring and observability
- ✅ Security audit
- ✅ Documentation
- ✅ Deployment tooling

**Deliverables**:
1. Caching layer (Redis)
2. Metrics and tracing (OpenTelemetry)
3. Security audit (internal + external)
4. Admin documentation
5. Deployment guides (Docker, K8s)
6. Backup/restore procedures
7. Incident response playbook

**Success Criteria**:
- <100ms p99 latency for auth flows
- Full observability with dashboards
- Security audit passed
- Production deployment successful
- 99.9% uptime SLA

## Total Timeline

**14 months** for complete implementation:
- Phase 1: Months 1-3
- Phase 2: Months 4-5
- Phase 3: Months 6-9
- Phase 4: Months 10-12
- Phase 5: Months 13-14

## Resource Requirements

### Development Team

**Core Team**:
- 2× Senior Backend Engineers (Rust)
- 1× Cryptography Engineer
- 1× Frontend Engineer (for account UI)
- 1× DevOps Engineer
- 0.5× Product Manager

**Advisors**:
- Matrix Protocol Expert
- DID/VC Standards Expert
- Security Consultant

### Infrastructure

**Development**:
- Development Matrix homeserver
- Test DID infrastructure
- CI/CD pipeline
- Staging environment

**Production**:
- PostgreSQL cluster
- Redis cluster
- Load balancers
- Monitoring stack (Prometheus, Grafana)

## Risk Mitigation

### Technical Risks

| Risk | Probability | Impact | Mitigation |
|------|------------|--------|------------|
| `openid4vc` crate immaturity | Medium | High | Contribute to crate, maintain fork if needed |
| Matrix MLS not ready | High | Medium | Phase 3 can be delayed, Phase 1-2 still valuable |
| Performance issues | Medium | Medium | Extensive benchmarking, caching strategy |
| Synapse Admin API limitations | Low | High | Work with Synapse team, propose enhancements |
| DID resolution failures | Medium | Medium | Cache, support did:key (no resolution needed) |

### Adoption Risks

| Risk | Probability | Impact | Mitigation |
|------|------------|--------|------------|
| Users don't have wallets | High | High | Provide reference wallet, partner with existing |
| Clients don't integrate | Medium | High | Reference implementation in Element, documentation |
| Complexity overwhelms users | Medium | Medium | Excellent UX design, onboarding flows |
| Deployment too complex | Low | Medium | Docker images, Helm charts, managed hosting |

### Standards Risks

| Risk | Probability | Impact | Mitigation |
|------|------------|--------|------------|
| OID4VP spec changes | Low | Medium | Track spec actively, modular implementation |
| MLS spec changes | Low | Medium | Use stable RFC 9420, track updates |
| W3C VC spec changes | Very Low | Low | Use stable VC 1.1, prepare for 2.0 |

## Success Metrics

### Phase 1 Success Metrics

- [ ] 100% of test users can log in with DID
- [ ] <500ms p99 authentication latency
- [ ] Zero security vulnerabilities in audit
- [ ] Integration tests >95% coverage

### Phase 2 Success Metrics

- [ ] 100% of new users successfully register
- [ ] Newnal Address verification >99% success rate
- [ ] Account UI satisfaction score >4/5
- [ ] Zero duplicate 3PIDs in production

### Phase 3 Success Metrics

- [ ] MLS credential issuance <1s p99
- [ ] Credential verification <100ms p99
- [ ] Zero false-positive revocations
- [ ] Federated verification working across 3+ servers

### Phase 4 Success Metrics

- [ ] VC room join success rate >99%
- [ ] Support 10+ VC types
- [ ] Presentation evaluation <200ms p99
- [ ] Privacy audit passed

### Phase 5 Success Metrics

- [ ] 99.9% uptime
- [ ] <1s p99 end-to-end auth latency
- [ ] Zero critical security issues
- [ ] 100% documentation coverage

## Future Enhancements

### Post-MVP Features

**Year 2**:
1. **Additional DID Methods**: did:web, did:ion, did:ethr
2. **Hardware Wallet Support**: YubiKey, Ledger integration
3. **Biometric Auth**: WebAuthn + DID binding
4. **Anonymous Credentials**: Zero-knowledge proofs for privacy
5. **Delegation**: Allow users to delegate auth to devices
6. **Recovery**: Social recovery for DIDs

**Year 3**:
7. **Verifiable Organizations**: Org DIDs for enterprises
8. **Credential Marketplace**: Discover/request VCs
9. **Audit Logs**: Immutable auth/authz logs
10. **AI-Assisted Policies**: Generate presentation definitions from natural language

## Community & Standardization

### Matrix Spec Proposals

**MSCs to Author**:
1. **MSC-XXXX**: External ID Authentication
2. **MSC-YYYY**: DID-Based Identity
3. **MSC-ZZZZ**: VC-Based Join Rules
4. **MSC-AAAA**: MLS Authentication Service

### Collaboration

**Engage with**:
- Matrix Foundation
- DIF (Decentralized Identity Foundation)
- IETF MIMI Working Group
- W3C Credentials Community Group

### Open Source

**Release Strategy**:
- Phase 1: Internal development
- Phase 2: Alpha release, invite testers
- Phase 3: Beta release, open source
- Phase 4: Public release, v1.0
- Phase 5: Production release, v1.0 stable

## Conclusion

MAS-DID represents a fundamental shift in how Matrix handles identity, moving from centralized accounts to user-controlled decentralized identifiers. The phased implementation approach balances ambition with pragmatism, delivering value at each milestone while building toward a complete system.

**Key Takeaways**:
1. **Proven Standards**: Built on W3C DIDs/VCs, OpenID, MLS
2. **Incremental Value**: Each phase delivers standalone benefits
3. **Federation-First**: Designed for multi-server, multi-platform future
4. **Privacy-Preserving**: User control, selective disclosure
5. **Production-Ready**: Performance, security, observability built-in

The roadmap provides a clear path from concept to production-ready decentralized identity system for Matrix.

## Additional Resources

- [W3C DID Specification](https://www.w3.org/TR/did-core/)
- [W3C VC Data Model](https://www.w3.org/TR/vc-data-model/)
- [OpenID for Verifiable Presentations](https://openid.net/specs/openid-4-verifiable-presentations-1_0.html)
- [RFC 9420: Messaging Layer Security](https://www.rfc-editor.org/rfc/rfc9420)
- [IETF MIMI Working Group](https://datatracker.ietf.org/wg/mimi/about/)
- [Matrix MSC3861: OIDC Delegated Auth](https://github.com/matrix-org/matrix-spec-proposals/pull/3861)
