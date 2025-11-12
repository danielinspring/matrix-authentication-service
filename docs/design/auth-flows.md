# Authentication Flows

## Overview

This document details the complete authentication flows for MAS-DID, including new user registration and existing user login. These flows integrate the OIDC-DID Bridge, identity mapping, and Newnal Address verification.

## Flow 1: New User Registration with Newnal Address

### Prerequisites

- User has DID and wallet app installed
- User has obtained `NewnalAddressVC` from Newnal registrar
- User has Matrix client (e.g., Element)

### Sequence Diagram

```
┌──────┐    ┌─────────┐    ┌─────────┐    ┌────────┐    ┌─────────┐
│Client│    │ Synapse │    │ MAS-DID │    │ Wallet │    │ Synapse │
│      │    │ (OIDC)  │    │         │    │        │    │  Admin  │
└──┬───┘    └────┬────┘    └────┬────┘    └───┬────┘    └────┬────┘
   │             │              │              │              │
   │ 1. Register │              │              │              │
   ├────────────>│              │              │              │
   │             │              │              │              │
   │             │ 2. Redirect  │              │              │
   │             │    to OIDC   │              │              │
   │             ├─────────────>│              │              │
   │             │              │              │              │
   │             │ 3. Display   │              │              │
   │<────────────┴──────────────┤              │              │
   │             Registration   │              │              │
   │             Page + QR Code │              │              │
   │                            │              │              │
   │                            │ 4. User      │              │
   │                            │    scans QR  │              │
   │                            │<─────────────┤              │
   │                            │              │              │
   │                            │ 5. Request   │              │
   │                            │    SIOPv2 +  │              │
   │                            │    Newnal VC │              │
   │                            ├─────────────>│              │
   │                            │              │              │
   │                            │              │ 6. User      │
   │                            │              │    approves  │
   │                            │              │              │
   │                            │ 7. ID Token  │              │
   │                            │    + VP with │              │
   │                            │    NewnalVC  │              │
   │                            │<─────────────┤              │
   │                            │              │              │
   │                            │ 8. Verify    │              │
   │                            │    DID &     │              │
   │                            │    VC        │              │
   │                            │              │              │
   │                            │ 9. Create    │              │
   │                            │    User      │              │
   │                            ├──────────────┼──────────────>│
   │                            │              │              │
   │                            │              │ 10. User     │
   │                            │              │     Created  │
   │                            │<─────────────┼───────────────┤
   │                            │              │              │
   │                            │ 11. OIDC     │              │
   │                            │     AuthCode │              │
   │             │<─────────────┤              │              │
   │             │              │              │              │
   │ 12. AuthCode│              │              │              │
   │<────────────┤              │              │              │
   │             │              │              │              │
   │ 13. Exchange│              │              │              │
   │     Code    │              │              │              │
   ├────────────>│              │              │              │
   │             │              │              │              │
   │             │ 14. Exchange │              │              │
   │             │     Code     │              │              │
   │             ├─────────────>│              │              │
   │             │              │              │              │
   │             │ 15. OIDC     │              │              │
   │             │     Tokens   │              │              │
   │             │<─────────────┤              │              │
   │             │              │              │              │
   │ 16. Tokens  │              │              │              │
   │<────────────┤              │              │              │
   │             │              │              │              │
   │ 17. Authenticated          │              │              │
```

### Detailed Steps

#### Step 1-2: Client Initiates Registration

**Client → Synapse**:
```http
GET https://synapse.example.com/_matrix/client/v3/register
```

**Synapse → MAS-DID**:
```http
HTTP/1.1 302 Found
Location: https://mas-did.example.com/authorize?
    response_type=code
    &client_id=synapse_client
    &redirect_uri=https://synapse.example.com/_synapse/client/oidc/callback
    &scope=openid%20urn:matrix:org.matrix.msc2967.client:api:*
    &state=abc123
    &nonce=xyz789
```

#### Step 3: MAS-DID Presents Registration Page

MAS-DID checks for existing session. If none, renders registration page.

**Handler**:
```rust
async fn handle_authorize(
    Query(params): Query<OidcParams>,
    session: Session,
) -> Result<Html<String>> {
    // Check for existing authenticated session
    if let Some(did) = session.get::<String>("authenticated_did")? {
        // User already authenticated, check if registered
        return handle_existing_session(did, params).await;
    }

    // No session - show registration/login choice
    render_auth_page(params).await
}

async fn render_auth_page(params: OidcParams) -> Result<Html<String>> {
    let template = r#"
        <h1>Welcome to MAS-DID</h1>
        <p>Sign in with your decentralized identity</p>

        <button onclick="register()">New User - Register</button>
        <button onclick="login()">Existing User - Login</button>
    "#;

    Ok(Html(template.to_string()))
}
```

#### Step 4-5: Registration Flow Initiation

User clicks "Register". MAS-DID generates SIOPv2 request with Newnal Address VC requirement.

**Request Generation**:
```rust
use openid4vc::siop::RelyingParty;
use openid4vc::oid4vp::PresentationDefinition;

async fn initiate_registration(session: Session) -> Result<Html<String>> {
    let rp = get_relying_party().await?;

    // Create presentation definition for Newnal Address
    let presentation_def = PresentationDefinition {
        id: "newnal-registration".to_string(),
        name: Some("Newnal Address Verification".to_string()),
        purpose: Some("Verify ownership of Newnal Mobile Address".to_string()),
        input_descriptors: vec![
            InputDescriptor {
                id: "newnal_address".to_string(),
                name: Some("Newnal Address Credential".to_string()),
                purpose: Some("Prove ownership of Newnal Mobile Address".to_string()),
                constraints: Constraints {
                    fields: vec![
                        Field {
                            path: vec!["$.type".to_string()],
                            filter: json!({
                                "type": "array",
                                "contains": { "const": "NewnalAddressCredential" }
                            }),
                        },
                        Field {
                            path: vec!["$.credentialSubject.newnalAddress".to_string()],
                            filter: json!({
                                "type": "string",
                                "pattern": "^\\+[0-9]+N[0-9]+$"
                            }),
                        }
                    ],
                },
            }
        ],
    };

    // Generate authorization request
    let nonce = generate_nonce();
    session.insert("siop_nonce", &nonce)?;
    session.insert("flow_type", "registration")?;

    let auth_req = rp.create_authorization_request(
        AuthorizationRequestParams {
            nonce,
            redirect_uri: "https://mas-did.example.com/auth/siop/callback".to_string(),
            presentation_definition: Some(presentation_def),
            state: generate_state(),
        }
    ).await?;

    // Store request for later retrieval
    let request_id = store_request(&auth_req).await?;

    // Generate QR code
    let request_uri = format!(
        "openid://?request_uri=https://mas-did.example.com/auth/siop/request/{}",
        request_id
    );

    let qr_code = generate_qr_code(&request_uri)?;

    // Render page
    let template = format!(r#"
        <h1>Register with Decentralized Identity</h1>
        <p>Scan this QR code with your wallet app</p>
        <img src="data:image/png;base64,{}" alt="QR Code" />
        <p>We'll request:</p>
        <ul>
            <li>Proof of your DID ownership (authentication)</li>
            <li>Your Newnal Mobile Address credential</li>
        </ul>
    "#, qr_code);

    Ok(Html(template))
}
```

#### Step 6-7: User Approves and Wallet Responds

**Wallet generates response**:
```json
{
  "id_token": "eyJhbGciOiJFZERTQSIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJkaWQ6a2V5Ono2TWtVc2VyLi4uIiwic3ViIjoiZGlkOmtleTp6Nk1rVXNlci4uLiIsImF1ZCI6ImRpZDprZXk6ejZNa01BU0RJRCIsIm5vbmNlIjoieHl6Nzg5IiwiaWF0IjoxNzAwMDAwMDAwLCJleHAiOjE3MDAwMDAwNjB9.signature",
  "vp_token": "eyJhbGciOiJFZERTQSIsInR5cCI6IkpXVCJ9.eyJob2xkZXIiOiJkaWQ6a2V5Ono2TWtVc2VyLi4uIiwidmVyaWZpYWJsZUNyZWRlbnRpYWwiOlt7Li4uTmV3bmFsQWRkcmVzc0NSZWR6bnRpYWwuLi59XX0.signature",
  "state": "abc123"
}
```

#### Step 8: MAS-DID Verifies

```rust
async fn handle_siop_callback(
    Form(response): Form<SiopResponse>,
    session: Session,
) -> Result<Response> {
    let rp = get_relying_party().await?;

    // Verify ID token (DID authentication)
    let id_token = rp.verify_id_token(&response.id_token).await?;
    let user_did = id_token.issuer(); // User's DID

    // Verify VP token (Newnal Address VC)
    let vp = rp.verify_vp_token(&response.vp_token).await?;

    // Extract and verify Newnal Address VC
    let vc = vp.verifiable_credential
        .first()
        .ok_or(anyhow!("No VC in VP"))?;

    verify_vc_issuer_trusted(vc, &["did:example:newnal-registrar"]).await?;
    verify_vc_not_expired(vc)?;
    verify_vc_subject_matches(vc, user_did)?;

    let newnal_address = vc.credential_subject
        .get("newnalAddress")
        .ok_or(anyhow!("No newnalAddress in VC"))?
        .as_str()
        .ok_or(anyhow!("Invalid newnalAddress"))?;

    // Check address not already taken
    if !check_address_available(newnal_address).await? {
        return Err(anyhow!("Newnal Address already registered"));
    }

    // Store verified info in session
    session.insert("authenticated_did", user_did)?;
    session.insert("verified_newnal_address", newnal_address)?;
    session.insert("registration_verified", true)?;

    // Continue registration flow
    complete_registration(session).await
}
```

#### Step 9-10: Create User in Synapse

```rust
async fn complete_registration(session: Session) -> Result<Response> {
    let admin_api = get_admin_api().await?;

    let did = session.get::<String>("authenticated_did")?
        .ok_or(anyhow!("No DID in session"))?;
    let newnal_address = session.get::<String>("verified_newnal_address")?
        .ok_or(anyhow!("No Newnal Address in session"))?;

    // Generate opaque MXID
    let mxid = generate_opaque_mxid(&config().matrix_domain);

    // Create user with atomic external_id and 3PID
    let request = CreateUserRequest {
        external_ids: vec![
            ExternalId {
                auth_provider: "mas-did".to_string(),
                external_id: did.clone(),
            }
        ],
        threepids: vec![
            ThreePid {
                medium: "m.id.newnal".to_string(),
                address: newnal_address.clone(),
            }
        ],
        ..Default::default()
    };

    admin_api.create_user(&mxid, request).await.map_err(|e| {
        error!(did = %did, error = %e, "User creation failed");
        anyhow!("Failed to create user: {}", e)
    })?;

    info!(
        mxid = %mxid,
        did = %did,
        newnal_address = %newnal_address,
        "User registered successfully"
    );

    // Store MXID in session
    session.insert("mxid", &mxid)?;

    // Proceed to OIDC flow
    complete_oidc_flow(session).await
}
```

#### Step 11-16: Complete OIDC Flow

```rust
async fn complete_oidc_flow(session: Session) -> Result<Response> {
    let mxid = session.get::<String>("mxid")?
        .ok_or(anyhow!("No MXID in session"))?;

    // Retrieve original OIDC parameters
    let redirect_uri = session.get::<String>("oidc_redirect_uri")?
        .ok_or(anyhow!("No redirect URI"))?;
    let state = session.get::<String>("oidc_state")?;

    // Generate authorization code
    let auth_code = generate_authorization_code();
    store_authorization_code(&auth_code, &mxid).await?;

    // Redirect back to Synapse
    let redirect_url = format!(
        "{}?code={}&state={}",
        redirect_uri,
        auth_code,
        state.unwrap_or_default()
    );

    Ok(Redirect::to(&redirect_url).into_response())
}
```

When Synapse exchanges the code:

```rust
async fn handle_token_exchange(
    Form(request): Form<TokenRequest>,
) -> Result<Json<TokenResponse>> {
    // Verify authorization code
    let mxid = verify_and_consume_code(&request.code).await?;

    // Generate tokens
    let access_token = generate_access_token(&mxid).await?;
    let refresh_token = generate_refresh_token(&mxid).await?;
    let id_token = generate_id_token(&mxid).await?;

    Ok(Json(TokenResponse {
        access_token,
        token_type: "Bearer".to_string(),
        expires_in: 3600,
        refresh_token: Some(refresh_token),
        id_token: Some(id_token),
    }))
}

async fn generate_id_token(mxid: &str) -> Result<String> {
    let localpart = mxid.trim_start_matches('@')
        .split(':')
        .next()
        .ok_or(anyhow!("Invalid MXID"))?;

    let claims = json!({
        "iss": config().issuer,
        "sub": localpart,
        "aud": "synapse_client",
        "iat": current_timestamp(),
        "exp": current_timestamp() + 3600,
    });

    sign_jwt(&claims).await
}
```

## Flow 2: Existing User Login

### Prerequisites

- User previously registered with MAS-DID
- User has same DID and wallet
- DID → MXID mapping exists in Synapse

### Sequence Diagram

```
┌──────┐    ┌─────────┐    ┌─────────┐    ┌────────┐    ┌─────────┐
│Client│    │ Synapse │    │ MAS-DID │    │ Wallet │    │ Synapse │
│      │    │ (OIDC)  │    │         │    │        │    │  Admin  │
└──┬───┘    └────┬────┘    └────┬────┘    └───┬────┘    └────┬────┘
   │             │              │              │              │
   │ 1. Login    │              │              │              │
   ├────────────>│              │              │              │
   │             │              │              │              │
   │             │ 2. Redirect  │              │              │
   │             │    to OIDC   │              │              │
   │             ├─────────────>│              │              │
   │             │              │              │              │
   │             │ 3. Display   │              │              │
   │<────────────┴──────────────┤              │              │
   │             Login Page +   │              │              │
   │             QR Code        │              │              │
   │                            │              │              │
   │                            │ 4. User      │              │
   │                            │    scans QR  │              │
   │                            │<─────────────┤              │
   │                            │              │              │
   │                            │ 5. Request   │              │
   │                            │    SIOPv2    │              │
   │                            ├─────────────>│              │
   │                            │              │              │
   │                            │ 6. ID Token  │              │
   │                            │<─────────────┤              │
   │                            │              │              │
   │                            │ 7. Verify    │              │
   │                            │    DID       │              │
   │                            │              │              │
   │                            │ 8. Lookup    │              │
   │                            │    MXID      │              │
   │                            ├──────────────┼──────────────>│
   │                            │              │              │
   │                            │              │ 9. Return    │
   │                            │              │    MXID      │
   │                            │<─────────────┼───────────────┤
   │                            │              │              │
   │                            │ 10. OIDC     │              │
   │                            │     AuthCode │              │
   │             │<─────────────┤              │              │
   │             │              │              │              │
   │ 11-16. Token Exchange (same as registration)             │
```

### Detailed Steps

#### Steps 1-3: Login Initiation

Similar to registration, but MAS-DID renders login page.

```rust
async fn initiate_login(session: Session) -> Result<Html<String>> {
    let rp = get_relying_party().await?;

    // Simple SIOPv2 request (no VC required for login)
    let nonce = generate_nonce();
    session.insert("siop_nonce", &nonce)?;
    session.insert("flow_type", "login")?;

    let auth_req = rp.create_authorization_request(
        AuthorizationRequestParams {
            nonce,
            redirect_uri: "https://mas-did.example.com/auth/siop/callback".to_string(),
            presentation_definition: None, // No VC required
            state: generate_state(),
        }
    ).await?;

    let request_id = store_request(&auth_req).await?;
    let request_uri = format!(
        "openid://?request_uri=https://mas-did.example.com/auth/siop/request/{}",
        request_id
    );

    let qr_code = generate_qr_code(&request_uri)?;

    let template = format!(r#"
        <h1>Sign In</h1>
        <p>Scan this QR code with your wallet app</p>
        <img src="data:image/png;base64,{}" alt="QR Code" />
    "#, qr_code);

    Ok(Html(template))
}
```

#### Steps 4-7: Authentication

Wallet provides only `id_token` (no `vp_token` needed).

```rust
async fn handle_siop_callback_login(
    Form(response): Form<SiopResponse>,
    session: Session,
) -> Result<Response> {
    let flow_type = session.get::<String>("flow_type")?
        .ok_or(anyhow!("No flow type"))?;

    if flow_type == "registration" {
        return handle_siop_callback_registration(response, session).await;
    }

    // LOGIN FLOW
    let rp = get_relying_party().await?;

    // Verify ID token only
    let id_token = rp.verify_id_token(&response.id_token).await?;
    let user_did = id_token.issuer();

    // Store authenticated DID
    session.insert("authenticated_did", user_did)?;

    // Lookup user
    lookup_and_complete(session).await
}
```

#### Steps 8-9: Lookup MXID

```rust
async fn lookup_and_complete(session: Session) -> Result<Response> {
    let admin_api = get_admin_api().await?;
    let did = session.get::<String>("authenticated_did")?
        .ok_or(anyhow!("No DID"))?;

    // Query Synapse for MXID
    let mxid = match admin_api.get_user_by_external_id("mas-did", &did).await {
        Ok(user) => user.user_id,
        Err(e) if e.status_code() == 404 => {
            // User not registered
            warn!(did = %did, "Login attempt with unregistered DID");
            return Err(anyhow!("DID not registered. Please register first."));
        }
        Err(e) => return Err(e.into()),
    };

    info!(did = %did, mxid = %mxid, "User logged in");

    session.insert("mxid", &mxid)?;

    // Complete OIDC flow (same as registration)
    complete_oidc_flow(session).await
}
```

## Flow 3: Session Reuse

If user already has valid MAS-DID session, skip SIOPv2 flow.

```rust
async fn handle_authorize(
    Query(params): Query<OidcParams>,
    session: Session,
) -> Result<Response> {
    // Check for existing valid session
    if let Some(mxid) = session.get::<String>("mxid")? {
        if session_valid(&session)? {
            info!(mxid = %mxid, "Reusing existing session");

            // Store OIDC params and complete flow
            session.insert("oidc_redirect_uri", &params.redirect_uri)?;
            session.insert("oidc_state", &params.state)?;

            return complete_oidc_flow(session).await;
        }
    }

    // No valid session - show login/register choice
    render_auth_page(params).await
}

fn session_valid(session: &Session) -> Result<bool> {
    if let Some(created_at) = session.get::<i64>("created_at")? {
        let age = current_timestamp() - created_at;
        Ok(age < SESSION_MAX_AGE_SECONDS)
    } else {
        Ok(false)
    }
}
```

## Flow 4: Device-to-Device Flow

For mobile apps with wallet installed locally, use deep links instead of QR codes.

```rust
async fn initiate_login_mobile(session: Session) -> Result<Response> {
    let rp = get_relying_party().await?;

    let auth_req = rp.create_authorization_request(/* ... */).await?;
    let request_id = store_request(&auth_req).await?;

    // Generate deep link instead of QR
    let deep_link = format!(
        "wallet-app://siop?request_uri=https://mas-did.example.com/auth/siop/request/{}",
        request_id
    );

    // Redirect to deep link
    Ok(Redirect::to(&deep_link).into_response())
}
```

## Error Handling

### Registration Errors

**DID Already Registered**:
```rust
if user_exists(&did).await? {
    return Err(UserError::AlreadyRegistered {
        message: "This DID is already registered. Please log in instead.".to_string(),
        did: did.to_string(),
    });
}
```

**Newnal Address Already Taken**:
```rust
if !check_address_available(&newnal_address).await? {
    return Err(UserError::AddressInUse {
        message: "This Newnal Address is already registered to another user.".to_string(),
        address: newnal_address.to_string(),
    });
}
```

**VC Verification Failed**:
```rust
match verify_vc(vc).await {
    Err(VCError::SignatureInvalid) => {
        return Err(UserError::InvalidCredential {
            message: "Your Newnal Address credential signature is invalid.".to_string(),
        });
    }
    Err(VCError::Expired) => {
        return Err(UserError::CredentialExpired {
            message: "Your Newnal Address credential has expired. Please obtain a new one.".to_string(),
        });
    }
    Err(VCError::UntrustedIssuer) => {
        return Err(UserError::UntrustedIssuer {
            message: "Your credential was not issued by a trusted registrar.".to_string(),
        });
    }
    Ok(()) => {}
}
```

### Login Errors

**Unregistered DID**:
```rust
if user_mxid.is_none() {
    return render_error_page(
        "Not Registered",
        "Your DID is not registered. Please register first.",
        Some("/register")
    );
}
```

**DID Verification Failed**:
```rust
match verify_did_signature(&did, &message, &signature).await {
    Err(e) => {
        warn!(did = %did, error = %e, "DID verification failed");
        return render_error_page(
            "Authentication Failed",
            "Unable to verify your DID. Please try again.",
            Some("/login")
        );
    }
    Ok(false) => {
        error!(did = %did, "Signature verification failed");
        return render_error_page(
            "Authentication Failed",
            "Signature verification failed. This may indicate a security issue.",
            None
        );
    }
    Ok(true) => {}
}
```

## Security Considerations

### Nonce Validation

```rust
async fn validate_nonce(
    session: &Session,
    id_token: &IdToken,
) -> Result<()> {
    let expected_nonce = session.get::<String>("siop_nonce")?
        .ok_or(anyhow!("No nonce in session"))?;

    if id_token.nonce != expected_nonce {
        error!(
            expected = %expected_nonce,
            received = %id_token.nonce,
            "Nonce mismatch"
        );
        return Err(anyhow!("Invalid nonce"));
    }

    // Clear nonce (single use)
    session.remove("siop_nonce")?;

    Ok(())
}
```

### Session Fixation Prevention

```rust
async fn handle_successful_auth(
    mut session: Session,
    did: &str,
    mxid: &str,
) -> Result<()> {
    // Regenerate session ID to prevent fixation
    session.regenerate().await?;

    // Store auth info
    session.insert("authenticated_did", did)?;
    session.insert("mxid", mxid)?;
    session.insert("created_at", current_timestamp())?;

    Ok(())
}
```

### CSRF Protection

```rust
async fn handle_authorize(
    Query(params): Query<OidcParams>,
    session: Session,
) -> Result<Response> {
    // Generate and store state for CSRF protection
    let state = generate_state();
    session.insert("oauth_state", &state)?;

    // Use state in OIDC flow
    // ...
}

async fn handle_callback(
    Query(params): Query<CallbackParams>,
    session: Session,
) -> Result<Response> {
    // Verify state
    let expected_state = session.get::<String>("oauth_state")?
        .ok_or(anyhow!("No state in session"))?;

    if params.state != expected_state {
        return Err(anyhow!("CSRF validation failed"));
    }

    // Clear state (single use)
    session.remove("oauth_state")?;

    // Continue...
}
```

## Performance Optimization

### Request Caching

Store SIOPv2 requests in Redis with short TTL:

```rust
async fn store_request(
    redis: &RedisPool,
    request: &AuthorizationRequest,
) -> Result<String> {
    let request_id = generate_request_id();
    let key = format!("siop_request:{}", request_id);

    redis.set_ex(
        &key,
        serde_json::to_string(request)?,
        600, // 10 minute TTL
    ).await?;

    Ok(request_id)
}
```

### Parallel Verification

When verifying multiple VCs or checking multiple conditions:

```rust
async fn verify_registration_requirements(
    did: &str,
    vp: &VerifiablePresentation,
) -> Result<RegistrationData> {
    let (vc_result, did_result, address_result) = tokio::join!(
        verify_vc(&vp.verifiable_credential[0]),
        check_did_not_registered(did),
        check_address_available(&extract_address(&vp))
    );

    Ok(RegistrationData {
        vc: vc_result?,
        did_available: did_result?,
        address_available: address_result?,
    })
}
```

## Testing Considerations

### Integration Tests

```rust
#[tokio::test]
async fn test_registration_flow() {
    let app = test_app().await;

    // 1. Initiate registration
    let response = app.get("/authorize?...").await;
    assert_eq!(response.status(), StatusCode::OK);

    // 2. Extract QR code / request URI
    let request_id = extract_request_id(&response).await;

    // 3. Simulate wallet response
    let id_token = create_test_id_token("did:key:test...");
    let vp_token = create_test_vp_with_newnal_vc("did:key:test...", "+82N1012345678");

    let response = app.post("/auth/siop/callback")
        .form(&[("id_token", &id_token), ("vp_token", &vp_token)])
        .await;

    assert_eq!(response.status(), StatusCode::FOUND);

    // 4. Verify user created in Synapse
    let user = admin_api.get_user_by_external_id("mas-did", "did:key:test...").await.unwrap();
    assert!(user.user_id.starts_with("@01"));
}
```

## Next Steps

- [MLS Authentication Service](./mls-as.md): Binding DIDs to encryption keys
- [VC-Based Authorization](./vc-authorization.md): Room access control
