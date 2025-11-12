# VC-Based Authorization

## Overview

This document describes how MAS-DID enables dynamic, Verifiable Credential (VC)-based authorization for Matrix rooms. This feature allows room administrators to require specific credentials for joining, enabling policy-driven access control beyond simple invite/public models.

## The Authorization Problem

### Current Matrix Authorization

Matrix currently supports limited join rules:

| Join Rule | Description | Limitations |
|-----------|-------------|-------------|
| `public` | Anyone can join | No access control |
| `invite` | Must be invited | Manual, doesn't scale |
| `knock` | Request to join | Still manual approval |
| `restricted` | Join if member of space | Space-based only |

### Use Cases Unsupported

1. **Organizational Access**: "Only employees of ACME Corp can join"
2. **Licensed Professionals**: "Only verified doctors can join medical room"
3. **Age Verification**: "Only users 18+ can join"
4. **Geographic Restrictions**: "Only EU residents can join GDPR compliance room"
5. **Time-Limited Access**: "Only active subscribers can join premium room"
6. **Multi-Factor Requirements**: "Must have government ID + company badge"

## VC-Based Solution

### Core Concept

Extend Matrix with new join rule: `m.join.vc`

**Room administrators define**:
- What type of credential required (e.g., "EmployeeCredential")
- Which issuers are trusted (e.g., "did:example:acme-hr")
- What claims must be present (e.g., department, role)

**Users prove**:
- They possess required credential
- Credential issued by trusted issuer
- Credential not expired/revoked
- Credential claims match requirements

**MAS-DID verifies and grants access**

## Architecture

### Components

```
┌────────────────────────────────────────────────────────┐
│                  Matrix Room State                     │
│  m.room.join_rules:                                    │
│    {                                                   │
│      "join_rule": "m.join.vc",                        │
│      "presentation_definition": { ... }               │
│    }                                                   │
└──────────────────┬─────────────────────────────────────┘
                   │
                   │ enforced by
                   ▼
┌────────────────────────────────────────────────────────┐
│               MAS-DID Authorization                    │
│  • Receives VC presentation from user                  │
│  • Verifies against room's requirements                │
│  • Grants/denies access                                │
└────────────────────────────────────────────────────────┘
```

### New Join Rule: m.join.vc

**Room State Event**:
```json
{
  "type": "m.room.join_rules",
  "state_key": "",
  "content": {
    "join_rule": "m.join.vc",
    "vc_requirements": {
      "presentation_definition": {
        "id": "employee-verification",
        "name": "ACME Employee Verification",
        "purpose": "Verify membership in ACME Corporation",
        "input_descriptors": [
          {
            "id": "employee_credential",
            "name": "Employee Credential",
            "purpose": "Verify employment at ACME Corp",
            "constraints": {
              "fields": [
                {
                  "path": ["$.type"],
                  "filter": {
                    "type": "array",
                    "contains": {
                      "const": "EmployeeCredential"
                    }
                  }
                },
                {
                  "path": ["$.issuer"],
                  "filter": {
                    "type": "string",
                    "const": "did:example:acme-hr"
                  }
                },
                {
                  "path": ["$.credentialSubject.company"],
                  "filter": {
                    "type": "string",
                    "const": "ACME Corporation"
                  }
                }
              ]
            }
          }
        ]
      }
    }
  }
}
```

### Presentation Definition

Uses **OpenID for Verifiable Presentations (OID4VP)** standard `presentation_definition` format.

**Key Components**:

| Field | Purpose |
|-------|---------|
| `input_descriptors` | List of required credentials |
| `constraints.fields` | Specific claims that must be present |
| `filter` | JSON Schema validation for claim values |

## Join Flow

### Sequence Diagram

```
┌────────┐     ┌────────┐     ┌─────────┐     ┌────────┐
│  User  │     │ Client │     │ MAS-DID │     │ Synapse│
└───┬────┘     └───┬────┘     └────┬────┘     └───┬────┘
    │              │               │              │
    │ 1. Request   │               │              │
    │    to join   │               │              │
    │    room      │               │              │
    ├─────────────>│               │              │
    │              │               │              │
    │              │ 2. Check      │              │
    │              │    join rules │              │
    │              ├───────────────┼─────────────>│
    │              │               │              │
    │              │               │ 3. Return    │
    │              │               │    m.join.vc │
    │              │<──────────────┼──────────────┤
    │              │               │              │
    │              │ 4. Request    │              │
    │              │    VC from    │              │
    │              │    user       │              │
    │<─────────────┤               │              │
    │              │               │              │
    │ 5. Present   │               │              │
    │    VC via    │               │              │
    │    wallet    │               │              │
    ├─────────────>│               │              │
    │              │               │              │
    │              │ 6. Submit VC  │              │
    │              │    to MAS-DID │              │
    │              ├──────────────>│              │
    │              │               │              │
    │              │               │ 7. Verify VC │
    │              │               │              │
    │              │               │ 8. Check     │
    │              │               │    against   │
    │              │               │    room reqs │
    │              │               │              │
    │              │               │ 9. Admin     │
    │              │               │    join user │
    │              │               ├─────────────>│
    │              │               │              │
    │              │               │ 10. Success  │
    │              │<──────────────┼──────────────┤
    │              │               │              │
    │ 11. Joined   │               │              │
    │<─────────────┤               │              │
```

### Detailed Steps

#### Step 1-3: Client Discovers VC Requirement

```rust
// Client queries room state
let join_rules = client.get_state_event(
    room_id,
    "m.room.join_rules",
    "",
).await?;

if join_rules["join_rule"] == "m.join.vc" {
    // VC required - extract presentation_definition
    let presentation_def = join_rules["vc_requirements"]["presentation_definition"].clone();

    // Show UI to user
    show_vc_requirement_ui(presentation_def).await?;
}
```

#### Step 4-5: User Presents VC

Client initiates OID4VP flow with user's wallet:

```rust
async fn request_vc_from_user(
    presentation_def: PresentationDefinition,
) -> Result<VerifiablePresentation> {
    // Generate authorization request
    let auth_req = build_vp_request(presentation_def).await?;

    // Display QR code or deep link
    let qr_code = generate_qr_code(&auth_req)?;
    display_qr_code(qr_code).await?;

    // Wait for wallet response
    let vp = wait_for_wallet_response().await?;

    Ok(vp)
}
```

#### Step 6-9: MAS-DID Verifies and Grants Access

**API Endpoint**: `POST /room/join/verify`

```rust
async fn verify_and_join_room(
    Json(request): Json<JoinWithVCRequest>,
    session: Session,
) -> Result<Json<JoinResponse>> {
    // 1. Verify user authenticated
    let did = session.get::<String>("authenticated_did")?
        .ok_or(anyhow!("Not authenticated"))?;

    let mxid = session.get::<String>("mxid")?
        .ok_or(anyhow!("No MXID"))?;

    // 2. Fetch room's join rules
    let room_state = get_room_state(&request.room_id, "m.room.join_rules", "").await?;

    let presentation_def = room_state
        .get("vc_requirements")
        .and_then(|v| v.get("presentation_definition"))
        .ok_or(anyhow!("No presentation_definition"))?;

    // 3. Verify VP signature
    let vp = verify_vp(&request.vp_token).await?;

    // 4. Verify VP holder matches authenticated DID
    if vp.holder != did {
        return Err(anyhow!("VP holder mismatch"));
    }

    // 5. Verify VP meets presentation_definition
    let evaluation = evaluate_presentation(&vp, &presentation_def).await?;

    if !evaluation.matches {
        return Err(anyhow!("VC does not meet requirements: {}", evaluation.reason));
    }

    // 6. Verify each VC in VP
    for vc in &vp.verifiable_credential {
        verify_vc_signature(vc).await?;
        verify_vc_not_expired(vc)?;
        verify_vc_not_revoked(vc).await?;
    }

    // 7. Use admin API to force-join user
    let admin_api = get_admin_api().await?;
    admin_api.join_room(&mxid, &request.room_id).await?;

    info!(
        mxid = %mxid,
        room_id = %request.room_id,
        "User joined room via VC authorization"
    );

    Ok(Json(JoinResponse {
        joined: true,
        room_id: request.room_id,
    }))
}
```

### Presentation Evaluation

**Matching Logic**:
```rust
use jsonschema::JSONSchema;

async fn evaluate_presentation(
    vp: &VerifiablePresentation,
    presentation_def: &PresentationDefinition,
) -> Result<EvaluationResult> {
    let mut results = Vec::new();

    // For each input descriptor (required credential type)
    for descriptor in &presentation_def.input_descriptors {
        // Find matching VC in VP
        let matching_vc = find_matching_vc(vp, descriptor)?;

        if matching_vc.is_none() {
            return Ok(EvaluationResult {
                matches: false,
                reason: format!("No VC found matching descriptor: {}", descriptor.id),
                satisfied_descriptors: vec![],
            });
        }

        let vc = matching_vc.unwrap();

        // Validate against constraints
        for field in &descriptor.constraints.fields {
            let field_result = evaluate_field_constraint(vc, field)?;

            if !field_result.matches {
                return Ok(EvaluationResult {
                    matches: false,
                    reason: format!("Field constraint not met: {}", field.path[0]),
                    satisfied_descriptors: results,
                });
            }
        }

        results.push(descriptor.id.clone());
    }

    Ok(EvaluationResult {
        matches: true,
        reason: "All requirements met".to_string(),
        satisfied_descriptors: results,
    })
}

fn evaluate_field_constraint(
    vc: &Credential,
    field: &Field,
) -> Result<FieldEvaluationResult> {
    // Extract value at JSON path
    let value = json_path_extract(&vc, &field.path)?;

    // Validate against JSON Schema filter
    let schema = JSONSchema::compile(&field.filter)
        .context("Invalid field filter schema")?;

    let is_valid = schema.is_valid(&value);

    Ok(FieldEvaluationResult {
        matches: is_valid,
        path: field.path[0].clone(),
        value,
    })
}
```

## Example Use Cases

### Use Case 1: Employee-Only Room

**Scenario**: ACME Corp wants internal communication room

**Room Configuration**:
```json
{
  "join_rule": "m.join.vc",
  "vc_requirements": {
    "presentation_definition": {
      "id": "acme-employee",
      "input_descriptors": [{
        "id": "employee_cred",
        "constraints": {
          "fields": [
            {
              "path": ["$.type"],
              "filter": {"contains": {"const": "EmployeeCredential"}}
            },
            {
              "path": ["$.issuer"],
              "filter": {"const": "did:web:acme.com:hr"}
            },
            {
              "path": ["$.credentialSubject.employmentStatus"],
              "filter": {"const": "active"}
            }
          ]
        }
      }]
    }
  }
}
```

**Required VC**:
```json
{
  "type": ["VerifiableCredential", "EmployeeCredential"],
  "issuer": "did:web:acme.com:hr",
  "credentialSubject": {
    "id": "did:key:z6MkAlice...",
    "employeeId": "E12345",
    "company": "ACME Corporation",
    "employmentStatus": "active",
    "department": "Engineering"
  }
}
```

### Use Case 2: Age-Gated Room

**Scenario**: Room requires users be 18+

**Room Configuration**:
```json
{
  "join_rule": "m.join.vc",
  "vc_requirements": {
    "presentation_definition": {
      "id": "age-verification",
      "input_descriptors": [{
        "id": "age_cred",
        "constraints": {
          "fields": [
            {
              "path": ["$.type"],
              "filter": {"contains": {"const": "AgeVerificationCredential"}}
            },
            {
              "path": ["$.credentialSubject.birthDate"],
              "filter": {
                "type": "string",
                "format": "date",
                "formatMaximum": "2006-01-01"
              }
            }
          ]
        }
      }]
    }
  }
}
```

### Use Case 3: Multi-Credential Requirement

**Scenario**: Premium room requires both subscription + identity verification

**Room Configuration**:
```json
{
  "join_rule": "m.join.vc",
  "vc_requirements": {
    "presentation_definition": {
      "id": "premium-access",
      "input_descriptors": [
        {
          "id": "subscription",
          "constraints": {
            "fields": [{
              "path": ["$.type"],
              "filter": {"contains": {"const": "SubscriptionCredential"}}
            }, {
              "path": ["$.credentialSubject.tier"],
              "filter": {"enum": ["premium", "enterprise"]}
            }, {
              "path": ["$.credentialSubject.expiresAt"],
              "filter": {
                "type": "string",
                "format": "date-time",
                "formatMinimum": "{{ now }}"
              }
            }]
          }
        },
        {
          "id": "identity",
          "constraints": {
            "fields": [{
              "path": ["$.type"],
              "filter": {"contains": {"const": "GovernmentIDCredential"}}
            }]
          }
        }
      ]
    }
  }
}
```

User must present **both** credentials to join.

## Client Integration

### Modified Join Flow

**Current Matrix Client**:
```rust
// Simple join
client.join_room_by_id(room_id).await?;
```

**With VC Support**:
```rust
async fn join_room_smart(
    client: &Client,
    room_id: &RoomId,
) -> Result<()> {
    // Try simple join first
    match client.join_room_by_id(room_id).await {
        Ok(_) => return Ok(()),
        Err(e) if e.status_code() == 403 => {
            // Check if VC required
            let join_rules = client.get_state_event(
                room_id,
                "m.room.join_rules",
                "",
            ).await?;

            if join_rules["join_rule"] == "m.join.vc" {
                // VC required - handle specially
                return join_room_with_vc(client, room_id, &join_rules).await;
            }

            return Err(e);
        }
        Err(e) => return Err(e),
    }
}

async fn join_room_with_vc(
    client: &Client,
    room_id: &RoomId,
    join_rules: &Value,
) -> Result<()> {
    let presentation_def = join_rules["vc_requirements"]["presentation_definition"].clone();

    // Request VC from user's wallet
    let vp = request_vc_from_user(presentation_def).await?;

    // Submit to MAS-DID
    let response = client.post(
        "/_matrix/client/unstable/org.matrix.msc9999/room/join/verify",
        json!({
            "room_id": room_id,
            "vp_token": vp.to_jwt(),
        })
    ).await?;

    Ok(())
}
```

### UI Considerations

**User Experience**:
1. User clicks "Join Room"
2. If VC required, show explanation: "This room requires proof of employment"
3. Display "Scan QR code with your wallet" or "Open wallet app"
4. After VC presented, show "Verifying..."
5. On success: "Joined room!"
6. On failure: "Unable to verify credentials. You may not have the required credential."

## Room Administration

### Setting VC Requirements

**Admin UI** or **Bot Command**:

```
/join_rule vc --definition employee-acme.json
```

**Bot Implementation**:
```rust
async fn handle_join_rule_command(
    bot: &Bot,
    room_id: &RoomId,
    presentation_def_file: &str,
) -> Result<()> {
    // Load presentation definition
    let presentation_def: PresentationDefinition = load_json(presentation_def_file)?;

    // Validate presentation definition
    validate_presentation_definition(&presentation_def)?;

    // Set room state
    bot.send_state_event(
        room_id,
        "m.room.join_rules",
        "",
        json!({
            "join_rule": "m.join.vc",
            "vc_requirements": {
                "presentation_definition": presentation_def
            }
        })
    ).await?;

    bot.send_message(room_id, "✅ Room join rule updated to require VC").await?;

    Ok(())
}
```

### Pre-validation Templates

Provide common templates for easy setup:

```rust
pub mod templates {
    pub const EMPLOYEE_VERIFICATION: &str = r#"{
        "id": "employee-verification",
        "input_descriptors": [{
            "id": "employee_cred",
            "constraints": {
                "fields": [{
                    "path": ["$.type"],
                    "filter": {"contains": {"const": "EmployeeCredential"}}
                }]
            }
        }]
    }"#;

    pub const AGE_VERIFICATION: &str = r#"{ /* ... */ }"#;
    pub const SUBSCRIPTION: &str = r#"{ /* ... */ }"#;
}
```

**Usage**:
```
/join_rule vc --template employee
```

## Security Considerations

### Issuer Trust Management

**Critical**: Only trusted issuers should be accepted.

**Trust Model Options**:

1. **Explicit Issuer List**:
```json
{
  "constraints": {
    "fields": [{
      "path": ["$.issuer"],
      "filter": {
        "enum": [
          "did:web:acme.com:hr",
          "did:key:z6Mk..."
        ]
      }
    }]
  }
}
```

2. **Trust Registry**:
```rust
async fn is_issuer_trusted(
    issuer: &str,
    credential_type: &str,
) -> Result<bool> {
    let trust_registry = get_trust_registry().await?;

    trust_registry.is_trusted(issuer, credential_type).await
}
```

3. **Delegation**:
```json
{
  "trusted_issuers": {
    "root": "did:web:government.example",
    "delegated": true,
    "max_depth": 2
  }
}
```

### Replay Prevention

**Risk**: User presents VC, joins room, then VC is revoked but user remains.

**Mitigations**:

1. **Periodic Re-verification**:
```rust
async fn periodic_membership_check(room_id: &RoomId) {
    let members = get_room_members(room_id).await?;

    for member in members {
        if requires_vc_verification(room_id).await? {
            let verification_status = check_member_vc_status(member).await?;

            if !verification_status.valid {
                kick_member(room_id, member, "Credential no longer valid").await?;
            }
        }
    }
}
```

2. **Short-Lived VCs**:
- Require VCs with expiration dates
- Enforce maximum validity period (e.g., 30 days)
- Require fresh VCs for sensitive rooms

3. **Webhook on Revocation**:
```rust
// VC issuer notifies MAS-DID of revocation
async fn handle_revocation_webhook(
    Json(notification): Json<RevocationNotification>,
) -> Result<StatusCode> {
    let credential_id = notification.credential_id;

    // Find users with this credential
    let affected_users = find_users_with_credential(&credential_id).await?;

    // Remove from VC-protected rooms
    for user in affected_users {
        remove_from_vc_rooms(user).await?;
    }

    Ok(StatusCode::OK)
}
```

### Privacy Considerations

**Selective Disclosure**: Users should present minimal information.

**Bad**:
```json
{
  "credentialSubject": {
    "id": "did:key:z6MkUser...",
    "name": "Alice Smith",
    "birthDate": "1990-05-15",
    "address": "123 Main St",
    "ssn": "123-45-6789"
  }
}
```

**Good** (for age verification):
```json
{
  "credentialSubject": {
    "id": "did:key:z6MkUser...",
    "ageOver18": true
  }
}
```

**Implementation**: Use BBS+ signatures for selective disclosure:
```rust
// User creates derived credential with only required fields
let derived_vc = original_vc.derive_credential(
    &["ageOver18"],  // Only disclose this field
    &user_key,
).await?;
```

### Authorization vs. Authentication

**Important Distinction**:
- **Authentication**: Who are you? (Handled by DID/SIOPv2)
- **Authorization**: What can you do? (Handled by VC presentation)

Never use VCs for authentication. Always authenticate with DID first, then check authorization via VC.

## Advanced Features

### Conditional Join Rules

**Scenario**: Allow join via VC *or* invite

```json
{
  "join_rule": "m.join.vc_or_invite",
  "vc_requirements": { /* ... */ }
}
```

**Logic**:
```rust
async fn can_join_room(
    user: &str,
    room_id: &RoomId,
    join_rules: &JoinRules,
) -> Result<bool> {
    match join_rules.join_rule.as_str() {
        "m.join.vc" => {
            // Must have VC
            check_vc_requirement(user, room_id).await
        }
        "m.join.vc_or_invite" => {
            // Check invite first (easier)
            if has_invite(user, room_id).await? {
                return Ok(true);
            }

            // Otherwise check VC
            check_vc_requirement(user, room_id).await
        }
        _ => Ok(false),
    }
}
```

### Dynamic Policies

**Scenario**: Room requirements change based on time or context

```json
{
  "join_rule": "m.join.vc",
  "vc_requirements": {
    "policies": [
      {
        "condition": {"time": {"after": "2024-01-01T00:00:00Z"}},
        "presentation_definition": { /* new requirements */ }
      },
      {
        "condition": {"default": true},
        "presentation_definition": { /* old requirements */ }
      }
    ]
  }
}
```

### VC Escrow

**Scenario**: Store encrypted VC proof for future re-verification

```rust
async fn join_room_with_escrow(
    user: &str,
    room_id: &RoomId,
    vp: &VerifiablePresentation,
) -> Result<()> {
    // Verify VP
    verify_and_join(user, room_id, vp).await?;

    // Encrypt VP for future re-verification
    let encrypted_vp = encrypt_vp(vp, &room_key).await?;

    // Store in room state (encrypted)
    store_member_vp_escrow(room_id, user, encrypted_vp).await?;

    Ok(())
}
```

**Benefits**:
- Re-verify without asking user again
- Audit trail of access grants
- Revocation checking without user interaction

## Performance Optimization

### Presentation Definition Caching

```rust
struct PresentationDefCache {
    cache: LruCache<RoomId, PresentationDefinition>,
}

impl PresentationDefCache {
    async fn get_or_fetch(
        &mut self,
        room_id: &RoomId,
    ) -> Result<Option<PresentationDefinition>> {
        if let Some(def) = self.cache.get(room_id) {
            return Ok(Some(def.clone()));
        }

        let join_rules = fetch_join_rules(room_id).await?;

        if let Some(def) = extract_presentation_definition(&join_rules) {
            self.cache.put(room_id.clone(), def.clone());
            Ok(Some(def))
        } else {
            Ok(None)
        }
    }
}
```

### Parallel Verification

```rust
async fn verify_multiple_vcs(
    vcs: Vec<Credential>,
) -> Result<Vec<VerificationResult>> {
    let mut set = JoinSet::new();

    for vc in vcs {
        set.spawn(async move {
            verify_vc(&vc).await
        });
    }

    let mut results = Vec::new();
    while let Some(result) = set.join_next().await {
        results.push(result??);
    }

    Ok(results)
}
```

## Testing

### Integration Tests

```rust
#[tokio::test]
async fn test_vc_room_join() {
    let server = test_server().await;

    // Create room with VC requirement
    let room_id = create_test_room(&server).await?;
    set_vc_join_rule(&room_id, employee_presentation_def()).await?;

    // User has valid credential
    let user = test_user().await;
    let vc = create_employee_vc(&user.did).await?;
    let vp = create_vp(&vc, &user.key).await?;

    // Attempt join
    let result = join_room_with_vc(&user, &room_id, &vp).await;

    assert!(result.is_ok());
    assert!(is_room_member(&room_id, &user.mxid).await?);
}

#[tokio::test]
async fn test_vc_room_join_invalid_credential() {
    // User has wrong type of credential
    let vc = create_age_verification_vc(&user.did).await?;
    let vp = create_vp(&vc, &user.key).await?;

    let result = join_room_with_vc(&user, &room_id, &vp).await;

    assert!(result.is_err());
    assert!(!is_room_member(&room_id, &user.mxid).await?);
}
```

## Next Steps

- [Federation & Roadmap](./federation-roadmap.md): Implementation plan and phased rollout
