# Matrix Authentication Service - Comprehensive API Documentation

## Table of Contents

1. [Overview](#overview)
2. [System Architecture](#system-architecture)
3. [Core Crates](#core-crates)
4. [REST API Endpoints](#rest-api-endpoints)
5. [GraphQL API](#graphql-api)
6. [Frontend Architecture](#frontend-architecture)
7. [Authentication Flows](#authentication-flows)
8. [Usage Examples](#usage-examples)
9. [Integration Guide](#integration-guide)

---

## Overview

Matrix Authentication Service (MAS) is an OAuth 2.0 and OpenID Connect (OIDC) provider specifically designed for Matrix homeservers. It implements next-generation authentication for Matrix as specified in [MSC3861](https://github.com/matrix-org/matrix-doc/pull/3861).

### Key Features

- **OAuth 2.0 & OIDC Provider**: Full implementation of OAuth 2.0 and OpenID Connect standards
- **Matrix Integration**: Native integration with Matrix homeservers (Synapse)
- **Multi-factor Authentication**: Support for multiple authentication methods
- **Upstream OAuth2 Providers**: Integration with external identity providers
- **Policy Engine**: OPA (Open Policy Agent) based policy enforcement
- **GraphQL API**: Modern GraphQL API for management operations
- **REST API**: Standards-compliant REST endpoints for OAuth2/OIDC

---

## System Architecture

### High-Level Architecture

```mermaid
graph TB
    subgraph "Client Layer"
        WebClient[Web Client]
        MobileClient[Mobile Client]
        ThirdPartyApp[Third-party App]
    end
    
    subgraph "MAS - Matrix Authentication Service"
        Router[Router/HTTP Server]
        Handlers[Request Handlers]
        
        subgraph "Core Services"
            OAuth2[OAuth2 Service]
            OIDC[OIDC Service]
            GraphQL[GraphQL API]
            PolicyEngine[Policy Engine]
        end
        
        subgraph "Storage Layer"
            StorageAPI[Storage API]
            PostgreSQL[(PostgreSQL)]
        end
        
        subgraph "Integration Layer"
            MatrixConnector[Matrix Connector]
            EmailService[Email Service]
            UpstreamOAuth[Upstream OAuth2]
        end
    end
    
    subgraph "External Systems"
        Homeserver[Matrix Homeserver]
        SMTP[SMTP Server]
        UpstreamIDP[Upstream Identity Providers]
    end
    
    WebClient --> Router
    MobileClient --> Router
    ThirdPartyApp --> Router
    
    Router --> Handlers
    Handlers --> OAuth2
    Handlers --> OIDC
    Handlers --> GraphQL
    Handlers --> PolicyEngine
    
    OAuth2 --> StorageAPI
    OIDC --> StorageAPI
    GraphQL --> StorageAPI
    StorageAPI --> PostgreSQL
    
    Handlers --> MatrixConnector
    Handlers --> EmailService
    Handlers --> UpstreamOAuth
    
    MatrixConnector --> Homeserver
    EmailService --> SMTP
    UpstreamOAuth --> UpstreamIDP
```

### Component Interaction Flow

```mermaid
sequenceDiagram
    participant Client
    participant Router
    participant Handler
    participant Storage
    participant Policy
    participant Matrix
    
    Client->>Router: HTTP Request
    Router->>Handler: Route to Handler
    Handler->>Storage: Load Data
    Storage-->>Handler: Return Data
    Handler->>Policy: Check Policy
    Policy-->>Handler: Policy Decision
    Handler->>Matrix: Verify with Homeserver
    Matrix-->>Handler: Verification Result
    Handler->>Storage: Save Changes
    Handler-->>Router: Response
    Router-->>Client: HTTP Response
```

---

## Core Crates

MAS is organized into multiple crates, each serving a specific purpose:

### 1. `mas-cli` - Command Line Interface

The main entry point for the application.

**Public API:**
- Main executable with subcommands for server management, configuration, and migrations

**Example Usage:**
```bash
# Start the server
mas-cli server

# Run database migrations
mas-cli database migrate

# Manage configuration
mas-cli config check
```

### 2. `mas-router` - URL Routing

Defines all routes and URL building utilities.

**Public Types:**
```rust
pub trait Route {
    fn route() -> &'static str;
    fn path_and_query(&self) -> Cow<'_, str>;
    fn absolute_url(&self, base: &Url) -> Url;
}

pub struct UrlBuilder {
    // URL building utilities
}
```

**Key Routes:**
- `OidcConfiguration` - `/.well-known/openid-configuration`
- `OAuth2TokenEndpoint` - `/oauth2/token`
- `OAuth2AuthorizationEndpoint` - `/authorize`
- `OidcUserinfo` - `/oauth2/userinfo`
- `OAuth2Introspection` - `/oauth2/introspect`
- `OAuth2Revocation` - `/oauth2/revoke`
- `OAuth2Keys` - `/oauth2/keys.json`

### 3. `mas-handlers` - Request Handlers

Implements HTTP request handlers for all endpoints.

**Public Functions:**

```rust
// Router constructors
pub fn healthcheck_router<S>() -> Router<S>;
pub fn graphql_router<S>(playground: bool, undocumented_oauth2_access: bool) -> Router<S>;
pub fn discovery_router<S>() -> Router<S>;
pub fn api_router<S>() -> Router<S>;
pub fn compat_router<S>(templates: Templates) -> Router<S>;
pub fn human_router<S>(templates: Templates) -> Router<S>;
pub fn admin_api_router<S>() -> Router<S>;

// GraphQL schema
pub fn graphql_schema() -> GraphQLSchema;
pub fn graphql_schema_builder() -> SchemaBuilder<Query, Mutation, EmptySubscription>;

// Support types
pub struct ActivityTracker;
pub struct BoundActivityTracker;
pub struct PreferredLanguage;
pub struct Limiter; // Rate limiting
pub struct RequesterFingerprint;
pub struct MetadataCache; // Upstream OAuth2 metadata cache
```

**Handler Categories:**
1. **OAuth2 Handlers** (`oauth2/*`)
   - Authorization endpoint
   - Token endpoint
   - Introspection
   - Revocation
   - Device authorization
   - Registration

2. **Admin Handlers** (`admin/*`)
   - Admin API endpoints (documented separately)

3. **GraphQL Handlers** (`graphql/*`)
   - Query and mutation handlers

4. **Compatibility Handlers** (`compat/*`)
   - Matrix Synapse compatibility endpoints

5. **View Handlers** (`views/*`)
   - HTML page rendering

### 4. `mas-data-model` - Data Models

Core domain models representing all entities.

**Public Types:**

```rust
// User Management
pub struct User;
pub struct UserEmail;
pub struct BrowserSession;
pub struct Password;
pub struct Authentication;
pub struct UserRegistration;
pub struct UserRecoverySession;

// OAuth2/OIDC
pub struct Client;
pub struct Session; // OAuth2 session
pub struct AccessToken;
pub struct RefreshToken;
pub struct AuthorizationGrant;
pub struct AuthorizationCode;
pub struct DeviceCodeGrant;

// Compatibility Layer
pub struct CompatSession; // Matrix Synapse compat
pub struct CompatAccessToken;
pub struct CompatRefreshToken;
pub struct Device;

// Upstream OAuth2
pub struct UpstreamOAuthProvider;
pub struct UpstreamOAuthLink;
pub struct UpstreamOAuthAuthorizationSession;

// Configuration
pub struct SiteConfig;
pub struct CaptchaConfig;

// Utilities
pub trait Clock;
pub struct SystemClock;
pub type BoxClock = Box<dyn Clock + Send + Sync>;
pub type BoxRng = Box<dyn RngCore + Send + Sync>;
```

**State Management:**

Many entities have state enums for tracking lifecycle:

```rust
pub enum SessionState {
    Active,
    Finished,
}

pub enum AccessTokenState {
    Valid,
    Revoked,
}

pub enum DeviceCodeGrantState {
    Pending,
    Fulfilled,
    Rejected,
    Exchanged,
}
```

### 5. `mas-storage` - Storage Abstraction

Defines repository traits for data access.

**Public Traits:**

```rust
/// Main repository access trait
pub trait RepositoryAccess: Send {
    type Error: std::error::Error + Send + Sync + 'static;
    
    fn user<'c>(&'c mut self) -> Box<dyn UserRepository<Error = Self::Error> + 'c>;
    fn oauth2_client<'c>(&'c mut self) -> Box<dyn OAuth2ClientRepository<Error = Self::Error> + 'c>;
    fn oauth2_session<'c>(&'c mut self) -> Box<dyn OAuth2SessionRepository<Error = Self::Error> + 'c>;
    fn oauth2_authorization_grant<'c>(&'c mut self) -> Box<dyn OAuth2AuthorizationGrantRepository<Error = Self::Error> + 'c>;
    fn oauth2_access_token<'c>(&'c mut self) -> Box<dyn OAuth2AccessTokenRepository<Error = Self::Error> + 'c>;
    // ... many more repository methods
}

/// Repository factory for creating repository instances
pub trait RepositoryFactory: Send + Sync {
    type Repository: RepositoryAccess;
    
    async fn repository(&self) -> Result<Self::Repository, Self::Error>;
}

/// Type-erased repository
pub type BoxRepository = Box<dyn RepositoryAccess<Error = BoxError>>;
pub type BoxRepositoryFactory = Box<dyn RepositoryFactory<Repository = BoxRepository, Error = BoxError>>;

/// Pagination support
pub struct Pagination {
    pub first: Option<usize>,
    pub after: Option<String>,
    pub last: Option<usize>,
    pub before: Option<String>,
}

pub struct Page<T> {
    pub edges: Vec<Edge<T>>,
    pub page_info: PageInfo,
}
```

### 6. `mas-config` - Configuration

Configuration loading and validation.

**Public API:**

```rust
pub trait ConfigurationSection: Deserialize + JSONSchema {
    fn validate(&self) -> Result<(), ConfigError>;
}

pub trait ConfigurationSectionExt: ConfigurationSection {
    fn extract(figment: &Figment) -> Result<Self, BoxedError>;
    fn extract_or_default(figment: &Figment) -> Result<Self, BoxedError>;
}

// Configuration sections
pub struct RootConfig;
pub struct DatabaseConfig;
pub struct MatrixConfig;
pub struct EmailConfig;
pub struct TelemetryConfig;
pub struct SecretsConfig;
// ... more config sections
```

### 7. `mas-keystore` - Key Management

Cryptographic key management.

**Public API:**

```rust
pub struct Keystore {
    // Key storage and management
}

impl Keystore {
    pub fn new() -> Self;
    pub async fn load_keys(&mut self) -> Result<(), Error>;
    pub fn signer(&self, algorithm: &Algorithm) -> Option<&Signer>;
    pub fn verifier(&self, kid: &str) -> Option<&Verifier>;
}

pub trait Encrypter: Send + Sync {
    fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>, Error>;
    fn decrypt(&self, ciphertext: &[u8]) -> Result<Vec<u8>, Error>;
}
```

### 8. `mas-policy` - Policy Engine

OPA (Open Policy Agent) integration for policy enforcement.

**Public API:**

```rust
pub struct Policy {
    // OPA policy engine
}

impl Policy {
    pub async fn evaluate_authorization_grant(&self, data: &AuthorizationGrantData) -> Result<PolicyDecision, Error>;
    pub async fn evaluate_client_registration(&self, data: &ClientRegistrationData) -> Result<PolicyDecision, Error>;
    pub async fn evaluate_email(&self, data: &EmailData) -> Result<PolicyDecision, Error>;
    pub async fn evaluate_register(&self, data: &RegisterData) -> Result<PolicyDecision, Error>;
}

pub struct PolicyDecision {
    pub allowed: bool,
    pub violations: Vec<String>,
}
```

### 9. `mas-matrix` - Matrix Integration

Integration with Matrix homeservers.

**Public Trait:**

```rust
#[async_trait]
pub trait HomeserverConnection: Send + Sync {
    async fn query_user(&self, localpart: &str) -> Result<bool, Error>;
    async fn provision_user(&self, user: &User) -> Result<(), Error>;
    async fn provision_device(&self, user: &User, device: &Device) -> Result<(), Error>;
    async fn delete_device(&self, user: &User, device_id: &str) -> Result<(), Error>;
    async fn sync_device_displayname(&self, user: &User, device: &Device) -> Result<(), Error>;
    async fn delete_user(&self, localpart: &str) -> Result<(), Error>;
    async fn set_displayname(&self, localpart: &str, displayname: Option<&str>) -> Result<(), Error>;
    async fn unset_displayname(&self, localpart: &str) -> Result<(), Error>;
    async fn allow_cross_signing_reset(&self, localpart: &str) -> Result<(), Error>;
}
```

### 10. `mas-templates` - Template Rendering

HTML template rendering using Minijinja.

**Public API:**

```rust
pub struct Templates {
    // Template engine
}

impl Templates {
    pub fn new() -> Self;
    pub fn render_error(&self, ctx: &ErrorContext) -> Result<String, Error>;
    pub fn render_not_found(&self, ctx: &NotFoundContext) -> Result<String, Error>;
    // ... template rendering methods for various pages
}

pub trait TemplateContext {
    fn with_language(self, locale: LanguageIdentifier) -> Self;
    fn with_csrf(self, csrf_token: CsrfToken) -> Self;
    fn with_session(self, session: &BrowserSession) -> Self;
}
```

### 11. `oauth2-types` - OAuth2 Type Definitions

Standard OAuth2 and OIDC types.

**Public Types:**

```rust
// OAuth2 Request/Response types
pub struct AuthorizationRequest;
pub struct TokenRequest;
pub struct TokenResponse;
pub struct IntrospectionRequest;
pub struct IntrospectionResponse;
pub struct RevocationRequest;
pub struct DeviceAuthorizationRequest;
pub struct DeviceAuthorizationResponse;

// OIDC types
pub struct IDToken;
pub struct UserInfoResponse;
pub struct ClientMetadata;

// Core types
pub enum ResponseType;
pub enum GrantType;
pub struct Scope;
pub struct ClientId;
```

---

## REST API Endpoints

### OpenID Connect Discovery

#### Get OpenID Configuration

```http
GET /.well-known/openid-configuration
```

**Response:**
```json
{
  "issuer": "https://auth.example.com",
  "authorization_endpoint": "https://auth.example.com/authorize",
  "token_endpoint": "https://auth.example.com/oauth2/token",
  "userinfo_endpoint": "https://auth.example.com/oauth2/userinfo",
  "jwks_uri": "https://auth.example.com/oauth2/keys.json",
  "registration_endpoint": "https://auth.example.com/oauth2/registration",
  "scopes_supported": ["openid", "profile", "email"],
  "response_types_supported": ["code"],
  "grant_types_supported": ["authorization_code", "refresh_token", "device_code"],
  "subject_types_supported": ["public"],
  "id_token_signing_alg_values_supported": ["RS256", "ES256"],
  "token_endpoint_auth_methods_supported": ["client_secret_basic", "client_secret_post", "private_key_jwt"]
}
```

### OAuth2 Authorization Flow

```mermaid
sequenceDiagram
    participant Client
    participant MAS
    participant User
    participant Homeserver
    
    Client->>MAS: GET /authorize?client_id=...&redirect_uri=...&scope=...
    MAS->>User: Show Login Page
    User->>MAS: POST /login (credentials)
    MAS->>Homeserver: Verify user
    Homeserver-->>MAS: User verified
    MAS->>User: Show Consent Page
    User->>MAS: POST /consent (approval)
    MAS->>Client: Redirect with authorization code
    Client->>MAS: POST /oauth2/token (exchange code)
    MAS-->>Client: Access token + Refresh token + ID token
    Client->>MAS: GET /oauth2/userinfo (with access token)
    MAS-->>Client: User information
```

#### Authorization Endpoint

```http
GET /authorize
```

**Query Parameters:**
- `client_id` (required): Client identifier
- `redirect_uri` (required): Redirect URI
- `response_type` (required): Must be `code`
- `scope` (required): Space-separated scopes (e.g., `openid profile email`)
- `state` (recommended): CSRF protection token
- `nonce` (optional): For ID token replay protection
- `code_challenge` (optional): PKCE code challenge
- `code_challenge_method` (optional): PKCE method (`S256` or `plain`)
- `prompt` (optional): `none`, `login`, or `consent`

**Example:**
```http
GET /authorize?client_id=abc123&redirect_uri=https%3A%2F%2Fclient.example.com%2Fcallback&response_type=code&scope=openid%20profile&state=xyz&code_challenge=E9Melhoa...&code_challenge_method=S256
```

**Response:**
Redirects to `redirect_uri` with:
- `code`: Authorization code
- `state`: Same state value from request

#### Token Endpoint

```http
POST /oauth2/token
Content-Type: application/x-www-form-urlencoded
```

**Parameters:**

For authorization code grant:
```
grant_type=authorization_code
code=<authorization_code>
redirect_uri=<redirect_uri>
client_id=<client_id>
code_verifier=<pkce_verifier> (if PKCE was used)
```

For refresh token grant:
```
grant_type=refresh_token
refresh_token=<refresh_token>
client_id=<client_id>
```

For device code grant:
```
grant_type=urn:ietf:params:oauth:grant-type:device_code
device_code=<device_code>
client_id=<client_id>
```

**Response:**
```json
{
  "access_token": "mat_...",
  "token_type": "Bearer",
  "expires_in": 3600,
  "refresh_token": "mar_...",
  "scope": "openid profile email",
  "id_token": "eyJhbGciOiJS..."
}
```

#### UserInfo Endpoint

```http
GET /oauth2/userinfo
Authorization: Bearer <access_token>
```

**Response:**
```json
{
  "sub": "01H2ABCDEFGHIJKLMNOPQRSTUV",
  "preferred_username": "alice",
  "email": "alice@example.com",
  "email_verified": true,
  "profile": "https://matrix.example.com/@alice:example.com",
  "picture": "https://matrix.example.com/_matrix/media/r0/download/..."
}
```

#### Introspection Endpoint

```http
POST /oauth2/introspect
Content-Type: application/x-www-form-urlencoded
Authorization: Basic <client_credentials>
```

**Parameters:**
```
token=<access_token>
```

**Response:**
```json
{
  "active": true,
  "scope": "openid profile email",
  "client_id": "abc123",
  "username": "alice",
  "token_type": "Bearer",
  "exp": 1234567890,
  "iat": 1234564290,
  "sub": "01H2ABCDEFGHIJKLMNOPQRSTUV"
}
```

#### Revocation Endpoint

```http
POST /oauth2/revoke
Content-Type: application/x-www-form-urlencoded
```

**Parameters:**
```
token=<access_or_refresh_token>
token_type_hint=access_token (or refresh_token)
```

**Response:**
```http
HTTP/1.1 200 OK
```

#### Device Authorization Endpoint

```http
POST /oauth2/device/authorize
Content-Type: application/x-www-form-urlencoded
```

**Parameters:**
```
client_id=<client_id>
scope=openid profile
```

**Response:**
```json
{
  "device_code": "GmRhmhcxhwAzkoEqiMEg_DnyEysNkuNhszIySk9eS",
  "user_code": "WDJB-MJHT",
  "verification_uri": "https://auth.example.com/device",
  "verification_uri_complete": "https://auth.example.com/device?code=WDJB-MJHT",
  "expires_in": 900,
  "interval": 5
}
```

#### Dynamic Client Registration

```http
POST /oauth2/registration
Content-Type: application/json
```

**Request:**
```json
{
  "client_name": "My Application",
  "client_uri": "https://myapp.example.com",
  "redirect_uris": [
    "https://myapp.example.com/callback"
  ],
  "grant_types": [
    "authorization_code",
    "refresh_token"
  ],
  "response_types": ["code"],
  "token_endpoint_auth_method": "client_secret_basic",
  "contacts": ["admin@myapp.example.com"],
  "logo_uri": "https://myapp.example.com/logo.png"
}
```

**Response:**
```json
{
  "client_id": "01H2CLIENTABCDEFGHIJKLMNO",
  "client_secret": "secret_value_here",
  "client_id_issued_at": 1234567890,
  "client_secret_expires_at": 0,
  "client_name": "My Application",
  "client_uri": "https://myapp.example.com",
  "redirect_uris": [
    "https://myapp.example.com/callback"
  ],
  "grant_types": [
    "authorization_code",
    "refresh_token"
  ],
  "response_types": ["code"],
  "token_endpoint_auth_method": "client_secret_basic"
}
```

### JWKS Endpoint

```http
GET /oauth2/keys.json
```

**Response:**
```json
{
  "keys": [
    {
      "kty": "RSA",
      "use": "sig",
      "kid": "key-id-1",
      "alg": "RS256",
      "n": "0vx7agoebGcQSuuPiLJXZptN9nndrQmbXEps2aiAFbWhM78LhWx...",
      "e": "AQAB"
    }
  ]
}
```

### Health Check

```http
GET /health
```

**Response:**
```json
{
  "status": "healthy",
  "database": "connected"
}
```

---

## GraphQL API

The GraphQL API provides rich querying and mutation capabilities for managing users, sessions, and OAuth2 clients.

### GraphQL Endpoint

```http
POST /graphql
Content-Type: application/json
```

### Authentication

GraphQL requests must include a valid access token:

```http
Authorization: Bearer <access_token>
```

### Schema Overview

```mermaid
graph LR
    Query --> ViewerQuery[Viewer Info]
    Query --> UserQuery[User Management]
    Query --> SessionQuery[Session Management]
    Query --> ClientQuery[OAuth2 Clients]
    
    Mutation --> UserMutation[User Operations]
    Mutation --> SessionMutation[Session Operations]
    Mutation --> EmailMutation[Email Operations]
    Mutation --> PasswordMutation[Password Operations]
```

### Key Queries

#### Get Current User (Viewer)

```graphql
query {
  viewer {
    __typename
    ... on User {
      id
      username
      createdAt
      emails {
        edges {
          node {
            id
            email
            confirmedAt
          }
        }
      }
    }
  }
}
```

#### List User Sessions

```graphql
query {
  viewer {
    ... on User {
      browserSessions(first: 10) {
        edges {
          node {
            id
            createdAt
            lastActiveAt
            userAgent {
              name
              os
              deviceType
            }
          }
        }
        pageInfo {
          hasNextPage
          endCursor
        }
      }
    }
  }
}
```

#### List OAuth2 Sessions

```graphql
query {
  viewer {
    ... on User {
      oauth2Sessions(first: 10, state: ACTIVE) {
        edges {
          node {
            id
            scope
            createdAt
            lastActiveAt
            client {
              id
              clientName
              clientUri
              logoUri
            }
          }
        }
      }
    }
  }
}
```

#### Get OAuth2 Client Details

```graphql
query GetClient($id: ID!) {
  oauth2Client(id: $id) {
    id
    clientId
    clientName
    clientUri
    logoUri
    redirectUris
  }
}
```

### Key Mutations

#### Add Email Address

```graphql
mutation AddEmail($userId: ID!, $email: String!) {
  addEmail(input: {
    userId: $userId
    email: $email
  }) {
    status
    email {
      id
      email
      confirmedAt
    }
  }
}
```

**Status values:**
- `ADDED` - Email successfully added
- `EXISTS` - Email already exists
- `INVALID` - Email format invalid
- `DENIED` - Blocked by policy

#### Remove Email Address

```graphql
mutation RemoveEmail($emailId: ID!) {
  removeEmail(input: {
    userEmailId: $emailId
  }) {
    status
    user {
      id
      emails {
        edges {
          node {
            id
            email
          }
        }
      }
    }
  }
}
```

#### Set Primary Email

```graphql
mutation SetPrimaryEmail($emailId: ID!) {
  setPrimaryEmail(input: {
    userEmailId: $emailId
  }) {
    status
    user {
      id
      primaryEmail {
        id
        email
      }
    }
  }
}
```

#### End Browser Session

```graphql
mutation EndSession($sessionId: ID!) {
  endBrowserSession(input: {
    browserSessionId: $sessionId
  }) {
    status
    browserSession {
      id
      state
    }
  }
}
```

#### End OAuth2 Session

```graphql
mutation EndOAuth2Session($sessionId: ID!) {
  endOauth2Session(input: {
    oauth2SessionId: $sessionId
  }) {
    status
    oauth2Session {
      id
      state
    }
  }
}
```

#### Set Password

```graphql
mutation SetPassword($userId: ID!, $password: String!) {
  setPassword(input: {
    userId: $userId
    password: $password
  }) {
    status
  }
}
```

#### Lock/Unlock User

```graphql
mutation LockUser($userId: ID!) {
  lockUser(input: {
    userId: $userId
  }) {
    status
    user {
      id
      lockedAt
    }
  }
}

mutation UnlockUser($userId: ID!) {
  unlockUser(input: {
    userId: $userId
  }) {
    status
    user {
      id
      lockedAt
    }
  }
}
```

#### Add User (Admin only)

```graphql
mutation AddUser($username: String!) {
  addUser(input: {
    username: $username
    skipHomeserverCheck: false
  }) {
    status
    user {
      id
      username
    }
  }
}
```

### Pagination

GraphQL queries use cursor-based pagination:

```graphql
query {
  viewer {
    ... on User {
      browserSessions(first: 10, after: "cursor_value") {
        edges {
          cursor
          node {
            id
          }
        }
        pageInfo {
          hasNextPage
          hasPreviousPage
          startCursor
          endCursor
        }
      }
    }
  }
}
```

### GraphQL Playground

If enabled, interactive GraphQL Playground is available at:

```
GET /graphql/playground
```

---

## Frontend Architecture

The frontend is a React application using:
- **React Router** for routing
- **TanStack Query** for data fetching
- **GraphQL Code Generator** for type-safe GraphQL operations
- **Tailwind CSS** for styling

### Frontend Architecture Diagram

```mermaid
graph TB
    subgraph "Frontend Application"
        Router[React Router]
        
        subgraph "Routes"
            AccountRoute[Account Management]
            SessionsRoute[Sessions]
            EmailsRoute[Email Management]
            PasswordRoute[Password Management]
            DevicesRoute[Devices]
            ClientsRoute[OAuth2 Clients]
        end
        
        subgraph "Components"
            UserProfile[User Profile]
            SessionCard[Session Card]
            EmailList[Email List]
            ClientDetail[Client Detail]
            Forms[Form Components]
        end
        
        subgraph "Services"
            GraphQLClient[GraphQL Client]
            AuthService[Auth Service]
            I18n[Internationalization]
        end
    end
    
    Router --> AccountRoute
    Router --> SessionsRoute
    Router --> EmailsRoute
    
    AccountRoute --> UserProfile
    SessionsRoute --> SessionCard
    EmailsRoute --> EmailList
    ClientsRoute --> ClientDetail
    
    UserProfile --> GraphQLClient
    SessionCard --> GraphQLClient
    EmailList --> GraphQLClient
    ClientDetail --> GraphQLClient
    
    GraphQLClient --> MAS[MAS GraphQL API]
```

### Key Frontend Components

#### 1. User Profile Management

**Location:** `frontend/src/components/UserProfile/`

Components for managing user profile information:
- `UserEmailList.tsx` - Display and manage email addresses
- `AddEmailForm.tsx` - Form to add new email addresses

#### 2. Session Management

**Location:** `frontend/src/components/Session/`

Components for viewing and managing user sessions:
- `BrowserSession.tsx` - Browser session display
- `OAuth2Session.tsx` - OAuth2 session display
- `CompatSession.tsx` - Compatibility session display
- `EndSessionButton.tsx` - Session termination controls
- `DeviceTypeIcon.tsx` - Device type visualization
- `ClientAvatar.tsx` - Client application avatars
- `LastActive.tsx` - Last activity timestamp

#### 3. Session Detail Views

**Location:** `frontend/src/components/SessionDetail/`

Detailed views for different session types:
- `BrowserSessionDetail.tsx` - Full browser session information
- `OAuth2SessionDetail.tsx` - Full OAuth2 session information
- `CompatSessionDetail.tsx` - Full compat session information
- `SessionHeader.tsx` - Session header component
- `SessionInfo.tsx` - Session information display
- `EditSessionName.tsx` - Edit session display name

#### 4. OAuth2 Client Management

**Location:** `frontend/src/components/Client/`

Components for OAuth2 client management:
- `OAuth2ClientDetail.tsx` - Display client details and permissions

#### 5. Account Management

**Location:** `frontend/src/routes/_account*.tsx`

Routes for account management:
- `_account.index.tsx` - Account overview
- `_account.sessions.index.tsx` - Session listing
- `_account.sessions.browsers.tsx` - Browser sessions
- `_account.plan.index.tsx` - Subscription/plan management

#### 6. Email Verification

**Location:** `frontend/src/routes/emails.*.tsx`

Email verification flows:
- `emails.$id.verify.tsx` - Email verification page
- `emails.$id.in-use.tsx` - Email already in use page

### Frontend Routes

```typescript
// Main routes structure
{
  path: "/",
  component: Root,
  children: [
    // Account management (requires auth)
    {
      path: "account",
      component: AccountLayout,
      children: [
        { index: true, component: AccountIndex },
        { path: "sessions", component: SessionsIndex },
        { path: "sessions/browsers", component: BrowserSessions },
      ]
    },
    // Password management
    { path: "password/change", component: PasswordChange },
    { path: "password/recovery", component: PasswordRecovery },
    // Session details
    { path: "sessions/:id", component: SessionDetail },
    // Device details
    { path: "devices/:id", component: DeviceDetail },
    // Client details
    { path: "clients/:id", component: ClientDetail },
  ]
}
```

---

## Authentication Flows

### 1. Authorization Code Flow with PKCE

```mermaid
sequenceDiagram
    participant App as Client App
    participant Browser as User Browser
    participant MAS as MAS Server
    participant HS as Matrix Homeserver
    
    Note over App: Generate code_verifier & code_challenge
    App->>Browser: Redirect to /authorize
    Browser->>MAS: GET /authorize?client_id=...&code_challenge=...
    
    alt User not authenticated
        MAS->>Browser: Show login page
        Browser->>MAS: POST /login (credentials)
        MAS->>HS: Verify credentials
        HS-->>MAS: Valid
        MAS->>Browser: Set session cookie
    end
    
    MAS->>Browser: Show consent page
    Browser->>MAS: POST /consent (approve)
    MAS->>Browser: Redirect to callback with code
    Browser->>App: Navigate to redirect_uri?code=...
    
    App->>MAS: POST /oauth2/token (code + code_verifier)
    MAS->>MAS: Verify PKCE challenge
    MAS-->>App: access_token + refresh_token + id_token
    
    App->>MAS: GET /oauth2/userinfo (access_token)
    MAS-->>App: User profile
```

### 2. Device Code Flow

```mermaid
sequenceDiagram
    participant Device as Limited Device
    participant User as User (Browser)
    participant MAS as MAS Server
    
    Device->>MAS: POST /oauth2/device/authorize
    MAS-->>Device: device_code + user_code + verification_uri
    
    Device->>Device: Display user_code
    Note over Device: Show: "Go to https://auth.example.com/device<br/>and enter code: ABCD-EFGH"
    
    User->>MAS: Navigate to verification_uri
    User->>MAS: Enter user_code
    MAS->>User: Show login page
    User->>MAS: POST /login (authenticate)
    MAS->>User: Show consent page
    User->>MAS: POST /consent (approve)
    MAS->>MAS: Mark device_code as approved
    
    loop Polling
        Device->>MAS: POST /oauth2/token (device_code)
        alt Not yet approved
            MAS-->>Device: authorization_pending
        else Approved
            MAS-->>Device: access_token + refresh_token + id_token
        end
    end
```

### 3. Upstream OAuth2 Login Flow

```mermaid
sequenceDiagram
    participant User as User
    participant MAS as MAS Server
    participant IDP as Upstream IDP<br/>(Google, GitHub, etc.)
    participant HS as Matrix Homeserver
    
    User->>MAS: Click "Sign in with Provider"
    MAS->>MAS: Create authorization session
    MAS->>User: Redirect to IDP
    User->>IDP: Authenticate
    IDP->>User: Redirect to MAS callback
    User->>MAS: GET /upstream/callback?code=...
    MAS->>IDP: Exchange code for tokens
    IDP-->>MAS: access_token + id_token
    MAS->>IDP: GET /userinfo
    IDP-->>MAS: User profile
    
    alt User exists
        MAS->>MAS: Link upstream account
    else User doesn't exist
        MAS->>HS: Check username availability
        HS-->>MAS: Available
        MAS->>MAS: Create user
        MAS->>HS: Provision user
    end
    
    MAS->>User: Set session cookie
    MAS->>User: Redirect to application
```

### 4. Session Refresh Flow

```mermaid
sequenceDiagram
    participant App as Client App
    participant MAS as MAS Server
    
    Note over App: Access token expired
    App->>MAS: POST /oauth2/token<br/>grant_type=refresh_token
    MAS->>MAS: Verify refresh token
    
    alt Refresh token valid
        MAS->>MAS: Rotate refresh token (optional)
        MAS-->>App: New access_token + new refresh_token
    else Refresh token invalid/expired
        MAS-->>App: 401 Unauthorized
        Note over App: User must re-authenticate
    end
```

---

## Usage Examples

### Example 1: Server Setup and Configuration

```bash
# Create configuration file
cat > config.yaml <<EOF
database:
  uri: "postgresql://mas:password@localhost/mas"

http:
  listeners:
    - name: web
      address: "0.0.0.0:8080"

matrix:
  homeserver: "https://matrix.example.com"
  endpoint: "http://localhost:8008"
  secret: "shared_secret_with_synapse"

secrets:
  encryption: "base64_encoded_secret_key"

upstream_oauth2:
  providers:
    - id: "github"
      issuer: "https://github.com/"
      client_id: "github_client_id"
      client_secret: "github_client_secret"
      scope: "read:user user:email"
      claims_imports:
        localpart:
          action: require
          template: "{{ user.login }}"
        email:
          action: require
          template: "{{ user.email }}"
EOF

# Run migrations
mas-cli database migrate

# Start server
mas-cli server
```

### Example 2: OAuth2 Client Application (Python)

```python
import requests
from urllib.parse import urlencode
import secrets
import hashlib
import base64

class MASClient:
    def __init__(self, issuer_url, client_id, client_secret, redirect_uri):
        self.issuer_url = issuer_url
        self.client_id = client_id
        self.client_secret = client_secret
        self.redirect_uri = redirect_uri
        
        # Discover endpoints
        config = requests.get(f"{issuer_url}/.well-known/openid-configuration").json()
        self.authorization_endpoint = config["authorization_endpoint"]
        self.token_endpoint = config["token_endpoint"]
        self.userinfo_endpoint = config["userinfo_endpoint"]
    
    def generate_pkce(self):
        """Generate PKCE code verifier and challenge"""
        code_verifier = base64.urlsafe_b64encode(secrets.token_bytes(32)).decode('utf-8').rstrip('=')
        code_challenge = base64.urlsafe_b64encode(
            hashlib.sha256(code_verifier.encode('utf-8')).digest()
        ).decode('utf-8').rstrip('=')
        return code_verifier, code_challenge
    
    def get_authorization_url(self, scope="openid profile email"):
        """Generate authorization URL"""
        code_verifier, code_challenge = self.generate_pkce()
        state = secrets.token_urlsafe(32)
        
        params = {
            "client_id": self.client_id,
            "redirect_uri": self.redirect_uri,
            "response_type": "code",
            "scope": scope,
            "state": state,
            "code_challenge": code_challenge,
            "code_challenge_method": "S256"
        }
        
        url = f"{self.authorization_endpoint}?{urlencode(params)}"
        return url, state, code_verifier
    
    def exchange_code(self, code, code_verifier):
        """Exchange authorization code for tokens"""
        data = {
            "grant_type": "authorization_code",
            "code": code,
            "redirect_uri": self.redirect_uri,
            "client_id": self.client_id,
            "client_secret": self.client_secret,
            "code_verifier": code_verifier
        }
        
        response = requests.post(self.token_endpoint, data=data)
        response.raise_for_status()
        return response.json()
    
    def refresh_token(self, refresh_token):
        """Refresh access token"""
        data = {
            "grant_type": "refresh_token",
            "refresh_token": refresh_token,
            "client_id": self.client_id,
            "client_secret": self.client_secret
        }
        
        response = requests.post(self.token_endpoint, data=data)
        response.raise_for_status()
        return response.json()
    
    def get_userinfo(self, access_token):
        """Get user information"""
        headers = {"Authorization": f"Bearer {access_token}"}
        response = requests.get(self.userinfo_endpoint, headers=headers)
        response.raise_for_status()
        return response.json()

# Usage
client = MASClient(
    issuer_url="https://auth.example.com",
    client_id="your_client_id",
    client_secret="your_client_secret",
    redirect_uri="https://yourapp.example.com/callback"
)

# Step 1: Get authorization URL
auth_url, state, code_verifier = client.get_authorization_url()
print(f"Navigate to: {auth_url}")

# Step 2: After user authorizes and returns with code
# code = request.args.get('code')
# tokens = client.exchange_code(code, code_verifier)

# Step 3: Get user info
# userinfo = client.get_userinfo(tokens['access_token'])
```

### Example 3: GraphQL Client (JavaScript/TypeScript)

```typescript
import { GraphQLClient, gql } from 'graphql-request';

const endpoint = 'https://auth.example.com/graphql';
const accessToken = 'your_access_token';

const client = new GraphQLClient(endpoint, {
  headers: {
    authorization: `Bearer ${accessToken}`,
  },
});

// Query current user
const VIEWER_QUERY = gql`
  query {
    viewer {
      __typename
      ... on User {
        id
        username
        createdAt
        emails {
          edges {
            node {
              id
              email
              confirmedAt
            }
          }
        }
        browserSessions(first: 10, state: ACTIVE) {
          edges {
            node {
              id
              createdAt
              lastActiveAt
              userAgent {
                name
                os
                deviceType
              }
            }
          }
        }
      }
    }
  }
`;

const data = await client.request(VIEWER_QUERY);
console.log('Current user:', data.viewer);

// Add email address
const ADD_EMAIL_MUTATION = gql`
  mutation AddEmail($userId: ID!, $email: String!) {
    addEmail(input: { userId: $userId, email: $email }) {
      status
      email {
        id
        email
        confirmedAt
      }
    }
  }
`;

const result = await client.request(ADD_EMAIL_MUTATION, {
  userId: data.viewer.id,
  email: 'newemail@example.com',
});

console.log('Add email result:', result.addEmail);

// End a session
const END_SESSION_MUTATION = gql`
  mutation EndSession($sessionId: ID!) {
    endBrowserSession(input: { browserSessionId: $sessionId }) {
      status
      browserSession {
        id
        state
      }
    }
  }
`;

await client.request(END_SESSION_MUTATION, {
  sessionId: 'session_id_here',
});
```

### Example 4: Device Code Flow (CLI Application)

```python
import requests
import time
import sys

def device_code_flow(client_id, issuer_url):
    # Get device code
    device_auth_response = requests.post(
        f"{issuer_url}/oauth2/device/authorize",
        data={
            "client_id": client_id,
            "scope": "openid profile email"
        }
    )
    device_auth = device_auth_response.json()
    
    # Display instructions to user
    print(f"\nPlease visit: {device_auth['verification_uri']}")
    print(f"And enter code: {device_auth['user_code']}")
    print("\nWaiting for authorization...")
    
    # Poll token endpoint
    interval = device_auth.get('interval', 5)
    expires_in = device_auth['expires_in']
    start_time = time.time()
    
    while time.time() - start_time < expires_in:
        time.sleep(interval)
        
        token_response = requests.post(
            f"{issuer_url}/oauth2/token",
            data={
                "grant_type": "urn:ietf:params:oauth:grant-type:device_code",
                "device_code": device_auth['device_code'],
                "client_id": client_id
            }
        )
        
        if token_response.status_code == 200:
            tokens = token_response.json()
            print("\n✓ Authorization successful!")
            return tokens
        
        error = token_response.json().get('error')
        if error == 'authorization_pending':
            print(".", end="", flush=True)
            continue
        elif error == 'slow_down':
            interval += 5
            continue
        else:
            print(f"\n✗ Error: {error}")
            return None
    
    print("\n✗ Authorization timed out")
    return None

# Usage
tokens = device_code_flow(
    client_id="your_client_id",
    issuer_url="https://auth.example.com"
)

if tokens:
    print(f"Access token: {tokens['access_token']}")
    # Use the access token for API calls
```

### Example 5: Matrix Homeserver Integration

Configure Synapse to use MAS:

```yaml
# homeserver.yaml
experimental_features:
  msc3861:
    enabled: true
    issuer: https://auth.example.com/
    client_id: synapse_client_id
    client_auth_method: client_secret_basic
    client_secret: synapse_client_secret
    
    # Admin token for MAS to call Synapse admin API
    admin_token: synapse_admin_token
    
    # Account management URL
    account_management_url: https://auth.example.com/account/
```

---

## Integration Guide

### Step 1: Register Your Application

Use dynamic client registration or register via admin API:

```http
POST /oauth2/registration HTTP/1.1
Host: auth.example.com
Content-Type: application/json

{
  "client_name": "My App",
  "client_uri": "https://myapp.example.com",
  "redirect_uris": ["https://myapp.example.com/callback"],
  "grant_types": ["authorization_code", "refresh_token"],
  "response_types": ["code"],
  "token_endpoint_auth_method": "client_secret_basic"
}
```

Save the returned `client_id` and `client_secret`.

### Step 2: Implement Authorization Code Flow

1. **Generate PKCE parameters** (recommended for security)
2. **Redirect user** to `/authorize` endpoint
3. **Handle callback** with authorization code
4. **Exchange code** for tokens at `/oauth2/token`
5. **Store tokens** securely
6. **Refresh tokens** before they expire

### Step 3: Use Access Tokens

Include access token in API requests:

```http
GET /oauth2/userinfo HTTP/1.1
Host: auth.example.com
Authorization: Bearer <access_token>
```

Or for GraphQL:

```http
POST /graphql HTTP/1.1
Host: auth.example.com
Authorization: Bearer <access_token>
Content-Type: application/json

{
  "query": "{ viewer { __typename } }"
}
```

### Step 4: Handle Token Expiration

When an access token expires:

1. Use refresh token to get new access token
2. If refresh token is expired, redirect user to login again

```http
POST /oauth2/token HTTP/1.1
Host: auth.example.com
Content-Type: application/x-www-form-urlencoded

grant_type=refresh_token&refresh_token=<refresh_token>&client_id=<client_id>
```

### Step 5: Implement Logout

To logout:

1. Revoke access and refresh tokens
2. Clear local session
3. Optionally redirect to MAS logout endpoint

```http
POST /oauth2/revoke HTTP/1.1
Host: auth.example.com
Content-Type: application/x-www-form-urlencoded

token=<access_or_refresh_token>&token_type_hint=access_token
```

### Security Best Practices

1. **Always use HTTPS** in production
2. **Use PKCE** for authorization code flow
3. **Validate state parameter** to prevent CSRF
4. **Store tokens securely** (never in localStorage for web apps)
5. **Implement token rotation** for refresh tokens
6. **Validate ID tokens** if using OIDC
7. **Use appropriate scopes** - request minimum necessary
8. **Implement proper error handling**
9. **Monitor for token leakage**
10. **Keep client secrets secure** (never expose in client-side code)

---

## Additional Resources

### Configuration Reference

See [Configuration Documentation](../reference/configuration.md) for detailed configuration options.

### Admin API

The Admin API provides additional management capabilities. See the [Admin API documentation](./index.html).

### Policy Engine

For custom authorization rules, see the [Policy Engine documentation](../topics/policy.md).

### Scopes Reference

For available OAuth2 scopes, see the [Scopes Reference](../reference/scopes.md).

### CLI Reference

For command-line operations, see the [CLI Reference](../reference/cli/).

### Development Guide

To contribute to MAS, see the [Contributing Guide](../development/contributing.md).

---

## Support

- **Community Room**: [#matrix-auth:matrix.org](https://matrix.to/#/#matrix-auth:matrix.org)
- **Issues**: [GitHub Issues](https://github.com/element-hq/matrix-authentication-service/issues)
- **Documentation**: [https://element-hq.github.io/matrix-authentication-service/](https://element-hq.github.io/matrix-authentication-service/)

---

*Last Updated: 2025-10-30*
