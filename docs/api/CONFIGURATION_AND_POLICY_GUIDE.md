# Configuration and Policy Engine Guide

## Overview

This guide covers MAS configuration system and the policy engine, which provides fine-grained control over authorization decisions.

## Configuration System

### Configuration Architecture

```mermaid
graph TB
    subgraph "Configuration Sources"
        EnvVars[Environment Variables]
        ConfigFiles[YAML Config Files]
        Defaults[Built-in Defaults]
    end
    
    subgraph "Configuration Loading"
        Figment[Figment Framework]
        Validation[Schema Validation]
    end
    
    subgraph "Configuration Sections"
        Database[Database Config]
        HTTP[HTTP Config]
        Matrix[Matrix Config]
        Email[Email Config]
        Secrets[Secrets Config]
        Telemetry[Telemetry Config]
        UpstreamOAuth[Upstream OAuth2]
        Policy[Policy Config]
        Captcha[Captcha Config]
    end
    
    EnvVars --> Figment
    ConfigFiles --> Figment
    Defaults --> Figment
    
    Figment --> Validation
    Validation --> Database
    Validation --> HTTP
    Validation --> Matrix
    Validation --> Email
    Validation --> Secrets
    Validation --> Telemetry
    Validation --> UpstreamOAuth
    Validation --> Policy
    Validation --> Captcha
```

### Configuration API

#### Core Traits

```rust
/// Trait for configuration sections
pub trait ConfigurationSection: 
    serde::de::DeserializeOwned + 
    schemars::JsonSchema 
{
    /// Validate the configuration
    fn validate(&self) -> Result<(), ConfigError>;
    
    /// Get the configuration key prefix
    fn key() -> &'static str;
}

/// Extension trait for loading configuration
pub trait ConfigurationSectionExt: ConfigurationSection {
    /// Extract configuration from Figment
    fn extract(figment: &Figment) -> Result<Self, BoxedError>;
    
    /// Extract configuration with defaults
    fn extract_or_default(figment: &Figment) -> Result<Self, BoxedError>;
}
```

### Main Configuration Sections

#### 1. Root Configuration

```rust
pub struct RootConfig {
    /// Database configuration
    pub database: DatabaseConfig,
    
    /// HTTP server configuration
    pub http: HttpConfig,
    
    /// Matrix homeserver configuration
    pub matrix: MatrixConfig,
    
    /// Email configuration
    pub email: EmailConfig,
    
    /// Secrets configuration
    pub secrets: SecretsConfig,
    
    /// Telemetry configuration
    pub telemetry: TelemetryConfig,
    
    /// Upstream OAuth2 providers
    pub upstream_oauth2: UpstreamOAuth2Config,
    
    /// Policy engine configuration
    pub policy: PolicyConfig,
    
    /// Captcha configuration
    pub captcha: Option<CaptchaConfig>,
    
    /// Session configuration
    pub session: SessionConfig,
    
    /// Password configuration
    pub passwords: PasswordsConfig,
    
    /// Account management configuration
    pub account: AccountConfig,
}
```

**Example:**

```yaml
# config.yaml
database:
  uri: "postgresql://mas:password@localhost/mas"
  min_connections: 5
  max_connections: 10

http:
  listeners:
    - name: web
      address: "0.0.0.0:8080"
      proxy_protocol: false
    - name: internal
      address: "127.0.0.1:8081"
      proxy_protocol: false

matrix:
  homeserver: "https://matrix.example.com"
  endpoint: "http://localhost:8008"
  secret: "shared_secret_here"

email:
  from: "\"Auth Service\" <noreply@example.com>"
  reply_to: "support@example.com"
  transport: smtp
  hostname: "smtp.example.com"
  port: 587
  mode: tls

secrets:
  encryption: "base64encodedkey=="
  keys:
    - kid: "key-1"
      key: "symmetric:base64key"
    - kid: "key-2"
      key: "rsa:path/to/private.pem"
```

#### 2. Database Configuration

```rust
pub struct DatabaseConfig {
    /// Database connection URI
    pub uri: String,
    
    /// Minimum connections in pool
    pub min_connections: Option<u32>,
    
    /// Maximum connections in pool
    pub max_connections: Option<u32>,
    
    /// Connection timeout
    pub connect_timeout: Option<Duration>,
    
    /// Idle timeout
    pub idle_timeout: Option<Duration>,
    
    /// Maximum lifetime of connections
    pub max_lifetime: Option<Duration>,
}
```

**Example:**

```yaml
database:
  uri: "postgresql://user:password@host:5432/database"
  min_connections: 5
  max_connections: 20
  connect_timeout: 30s
  idle_timeout: 10m
  max_lifetime: 30m
```

#### 3. HTTP Configuration

```rust
pub struct HttpConfig {
    /// List of HTTP listeners
    pub listeners: Vec<ListenerConfig>,
    
    /// Public base URL
    pub public_base: Option<Url>,
    
    /// ISSUER URL (defaults to public_base)
    pub issuer: Option<Url>,
    
    /// Trusted proxies
    pub trusted_proxies: Vec<IpNetwork>,
}

pub struct ListenerConfig {
    /// Listener name
    pub name: String,
    
    /// Bind address
    pub address: SocketAddr,
    
    /// Use PROXY protocol
    pub proxy_protocol: bool,
    
    /// TLS configuration
    pub tls: Option<TlsConfig>,
    
    /// Resources to serve
    pub resources: Vec<ResourceConfig>,
}

pub enum ResourceConfig {
    /// Discovery endpoints
    Discovery,
    
    /// Human-facing pages
    Human,
    
    /// OAuth2/OIDC API endpoints
    OAuth2,
    
    /// GraphQL API
    GraphQL { playground: bool },
    
    /// Compatibility endpoints
    Compat,
    
    /// Admin API
    Admin,
    
    /// Health check
    Health,
}
```

**Example:**

```yaml
http:
  public_base: "https://auth.example.com"
  issuer: "https://auth.example.com/"
  
  listeners:
    - name: web
      address: "0.0.0.0:8443"
      proxy_protocol: false
      tls:
        certificate: "/path/to/cert.pem"
        key: "/path/to/key.pem"
      resources:
        - discovery
        - human
        - oauth2
        - graphql:
            playground: true
        - compat
    
    - name: admin
      address: "127.0.0.1:8081"
      resources:
        - admin
        - health
  
  trusted_proxies:
    - "10.0.0.0/8"
    - "172.16.0.0/12"
```

#### 4. Matrix Configuration

```rust
pub struct MatrixConfig {
    /// Matrix homeserver URL (public)
    pub homeserver: Url,
    
    /// Admin API endpoint (internal)
    pub endpoint: Url,
    
    /// Shared secret for authentication
    pub secret: String,
}
```

**Example:**

```yaml
matrix:
  homeserver: "https://matrix.example.com"
  endpoint: "http://synapse:8008"
  secret: "shared_secret_with_synapse"
```

#### 5. Email Configuration

```rust
pub struct EmailConfig {
    /// From address
    pub from: String,
    
    /// Reply-to address
    pub reply_to: Option<String>,
    
    /// Email transport
    pub transport: EmailTransportConfig,
}

pub enum EmailTransportConfig {
    /// SMTP transport
    Smtp {
        hostname: String,
        port: u16,
        mode: SmtpMode,
        username: Option<String>,
        password: Option<String>,
    },
    
    /// Sendmail transport
    Sendmail {
        command: String,
    },
    
    /// Blackhole (for testing)
    Blackhole,
}

pub enum SmtpMode {
    Plain,
    StartTls,
    Tls,
}
```

**Example:**

```yaml
email:
  from: "\"Matrix Auth\" <noreply@example.com>"
  reply_to: "support@example.com"
  transport: smtp
  hostname: "smtp.gmail.com"
  port: 587
  mode: starttls
  username: "noreply@example.com"
  password: "${SMTP_PASSWORD}"
```

#### 6. Upstream OAuth2 Configuration

```rust
pub struct UpstreamOAuth2Config {
    /// List of upstream providers
    pub providers: Vec<UpstreamOAuth2ProviderConfig>,
}

pub struct UpstreamOAuth2ProviderConfig {
    /// Provider ID
    pub id: String,
    
    /// OIDC issuer URL
    pub issuer: Option<Url>,
    
    /// Authorization endpoint (if not using discovery)
    pub authorization_endpoint: Option<Url>,
    
    /// Token endpoint
    pub token_endpoint: Option<Url>,
    
    /// JWKS URI
    pub jwks_uri: Option<Url>,
    
    /// Client ID
    pub client_id: String,
    
    /// Client secret
    pub client_secret: Option<String>,
    
    /// Token authentication method
    pub token_endpoint_auth_method: Option<OAuthClientAuthenticationMethod>,
    
    /// Scopes to request
    pub scope: String,
    
    /// Claims to import
    pub claims_imports: ClaimsImports,
    
    /// Display name for the provider
    pub human_name: Option<String>,
    
    /// Brand identifier
    pub brand_name: Option<String>,
}

pub struct ClaimsImports {
    /// How to determine the localpart
    pub localpart: ClaimImport,
    
    /// How to import the display name
    pub displayname: Option<ClaimImport>,
    
    /// How to import email
    pub email: Option<ClaimImport>,
}

pub struct ClaimImport {
    /// Import action (force, suggest, require, ignore)
    pub action: ImportAction,
    
    /// Jinja2 template for extracting the value
    pub template: String,
}
```

**Example:**

```yaml
upstream_oauth2:
  providers:
    - id: "google"
      issuer: "https://accounts.google.com"
      client_id: "google-client-id"
      client_secret: "${GOOGLE_CLIENT_SECRET}"
      scope: "openid email profile"
      token_endpoint_auth_method: client_secret_post
      human_name: "Google"
      brand_name: "google"
      
      claims_imports:
        localpart:
          action: force
          template: "{{ user.email.split('@')[0] }}"
        
        email:
          action: suggest
          template: "{{ user.email }}"
        
        displayname:
          action: suggest
          template: "{{ user.name }}"
    
    - id: "github"
      issuer: "https://github.com/"
      client_id: "github-client-id"
      client_secret: "${GITHUB_CLIENT_SECRET}"
      scope: "read:user user:email"
      human_name: "GitHub"
      brand_name: "github"
      
      claims_imports:
        localpart:
          action: force
          template: "{{ user.login }}"
        
        email:
          action: require
          template: "{{ user.email }}"
        
        displayname:
          action: suggest
          template: "{{ user.name }}"
```

#### 7. Policy Configuration

```rust
pub struct PolicyConfig {
    /// Path to policy WASM modules
    pub wasm_modules: Vec<PathBuf>,
    
    /// Policy data
    pub data: Option<serde_json::Value>,
}
```

**Example:**

```yaml
policy:
  wasm_modules:
    - "/path/to/policies/authorization_grant.wasm"
    - "/path/to/policies/client_registration.wasm"
    - "/path/to/policies/email.wasm"
    - "/path/to/policies/register.wasm"
  
  data:
    allowed_domains:
      - "example.com"
      - "trusted.org"
    
    blocked_usernames:
      - "admin"
      - "root"
      - "system"
```

#### 8. Session Configuration

```rust
pub struct SessionConfig {
    /// TTL for inactive sessions
    pub ttl: Duration,
    
    /// Grace period after TTL
    pub grace_period: Duration,
    
    /// Maximum session lifetime
    pub max_lifetime: Option<Duration>,
}
```

**Example:**

```yaml
session:
  ttl: 7d           # Sessions expire after 7 days of inactivity
  grace_period: 1h  # 1 hour grace period
  max_lifetime: 90d # Absolute maximum lifetime
```

#### 9. Secrets Configuration

```rust
pub struct SecretsConfig {
    /// Encryption key for sensitive data
    pub encryption: String,
    
    /// Signing keys
    pub keys: Vec<KeyConfig>,
}

pub struct KeyConfig {
    /// Key ID
    pub kid: String,
    
    /// Key material
    pub key: KeyMaterial,
}

pub enum KeyMaterial {
    /// Symmetric key
    Symmetric { key: Vec<u8> },
    
    /// RSA private key
    Rsa { key: RsaPrivateKey },
    
    /// EC private key
    EllipticCurve { key: EcPrivateKey },
}
```

**Example:**

```yaml
secrets:
  # Base64-encoded encryption key for database encryption
  encryption: "YWVhZC1rZXktZm9yLWRhdGFiYXNlLWVuY3J5cHRpb24="
  
  keys:
    # Symmetric key for HMAC signing
    - kid: "hmac-2024"
      key: "symmetric:c2VjcmV0LWtleS1mb3ItaG1hYy1zaWduaW5n"
    
    # RSA key for RS256 signing
    - kid: "rsa-2024"
      key: "rsa:/path/to/rsa-private.pem"
    
    # EC key for ES256 signing
    - kid: "ec-2024"
      key: "ecdsa:/path/to/ec-private.pem"
```

## Policy Engine

The policy engine uses Open Policy Agent (OPA) compiled to WebAssembly to enforce fine-grained authorization policies.

### Policy Architecture

```mermaid
graph TB
    subgraph "Request Flow"
        Request[Incoming Request]
        Handler[Request Handler]
        Decision[Policy Decision]
    end
    
    subgraph "Policy Engine"
        Engine[OPA Engine]
        
        subgraph "Policy Modules"
            AuthzGrant[Authorization Grant Policy]
            ClientReg[Client Registration Policy]
            EmailPolicy[Email Policy]
            RegisterPolicy[Register Policy]
        end
        
        PolicyData[Policy Data]
    end
    
    Request --> Handler
    Handler --> Engine
    
    Engine --> AuthzGrant
    Engine --> ClientReg
    Engine --> EmailPolicy
    Engine --> RegisterPolicy
    
    PolicyData --> Engine
    
    AuthzGrant --> Decision
    ClientReg --> Decision
    EmailPolicy --> Decision
    RegisterPolicy --> Decision
    
    Decision --> Handler
```

### Policy API

```rust
/// Policy engine
pub struct Policy {
    // Internal OPA engine
}

impl Policy {
    /// Create a new policy engine
    pub fn new(config: &PolicyConfig) -> Result<Self, Error>;
    
    /// Evaluate authorization grant policy
    pub async fn evaluate_authorization_grant(
        &self,
        input: &AuthorizationGrantInput,
    ) -> Result<PolicyDecision, Error>;
    
    /// Evaluate client registration policy
    pub async fn evaluate_client_registration(
        &self,
        input: &ClientRegistrationInput,
    ) -> Result<PolicyDecision, Error>;
    
    /// Evaluate email policy
    pub async fn evaluate_email(
        &self,
        input: &EmailInput,
    ) -> Result<PolicyDecision, Error>;
    
    /// Evaluate registration policy
    pub async fn evaluate_register(
        &self,
        input: &RegisterInput,
    ) -> Result<PolicyDecision, Error>;
}

/// Policy decision result
pub struct PolicyDecision {
    /// Whether the action is allowed
    pub allowed: bool,
    
    /// List of policy violations (if denied)
    pub violations: Vec<String>,
}
```

### Policy Input Types

#### Authorization Grant Policy

```rust
pub struct AuthorizationGrantInput {
    /// The user requesting authorization
    pub user: User,
    
    /// The client requesting authorization
    pub client: Client,
    
    /// The requested scope
    pub scope: Scope,
    
    /// Whether this is a client credentials grant
    pub is_client_credentials: bool,
}
```

**Example Policy (Rego):**

```rego
package authorization_grant

import future.keywords.if
import future.keywords.in

default allow := true

# Deny if user is locked
deny["User account is locked"] if {
    input.user.locked_at != null
}

# Deny if client is not allowed for this user
deny["Client not authorized"] if {
    input.client.client_id in data.restricted_clients
    not input.user.username in data.admin_users
}

# Deny certain scopes for non-admin users
deny["Insufficient privileges for requested scope"] if {
    some scope in input.scope
    scope in data.admin_only_scopes
    not input.user.can_request_admin
}

# Final decision
allow if {
    count(deny) == 0
}

violations := deny
```

#### Client Registration Policy

```rust
pub struct ClientRegistrationInput {
    /// The client metadata being registered
    pub client_metadata: ClientMetadata,
    
    /// Whether this is from a privileged context
    pub is_privileged: bool,
}
```

**Example Policy:**

```rego
package client_registration

import future.keywords.if
import future.keywords.in

default allow := true

# Deny if redirect_uri is not in allowed list
deny["Redirect URI not allowed"] if {
    some redirect_uri in input.client_metadata.redirect_uris
    not any_allowed_domain(redirect_uri)
}

any_allowed_domain(uri) if {
    some domain in data.allowed_redirect_domains
    startswith(uri, domain)
}

# Require approval for certain grant types
deny["Grant type requires approval"] if {
    some grant_type in input.client_metadata.grant_types
    grant_type in data.restricted_grant_types
    not input.is_privileged
}

allow if {
    count(deny) == 0
}

violations := deny
```

#### Email Policy

```rust
pub struct EmailInput {
    /// The user adding the email
    pub user: User,
    
    /// The email address being added
    pub email: String,
}
```

**Example Policy:**

```rego
package email

import future.keywords.if
import future.keywords.in

default allow := true

# Deny emails from blocked domains
deny["Email domain not allowed"] if {
    domain := split(input.email, "@")[1]
    domain in data.blocked_domains
}

# Only allow emails from specific domains
deny["Only corporate emails allowed"] if {
    data.require_corporate_email
    domain := split(input.email, "@")[1]
    not domain in data.allowed_domains
}

# Limit number of emails per user
deny["Maximum number of emails reached"] if {
    count(input.user.emails) >= data.max_emails_per_user
}

allow if {
    count(deny) == 0
}

violations := deny
```

#### Registration Policy

```rust
pub struct RegisterInput {
    /// The username being registered
    pub username: String,
    
    /// The email being registered (if any)
    pub email: Option<String>,
}
```

**Example Policy:**

```rego
package register

import future.keywords.if
import future.keywords.in

default allow := true

# Deny reserved usernames
deny["Username is reserved"] if {
    input.username in data.reserved_usernames
}

# Deny usernames with inappropriate words
deny["Username contains inappropriate content"] if {
    some word in data.blocked_words
    contains(lower(input.username), word)
}

# Require email from specific domain
deny["Email required from corporate domain"] if {
    data.require_email
    input.email == null
}

deny["Corporate email required"] if {
    data.require_corporate_email
    input.email != null
    domain := split(input.email, "@")[1]
    not domain in data.corporate_domains
}

# Username length requirements
deny["Username too short"] if {
    count(input.username) < data.min_username_length
}

deny["Username too long"] if {
    count(input.username) > data.max_username_length
}

allow if {
    count(deny) == 0
}

violations := deny
```

### Policy Data Configuration

Policy data is configured in the main configuration file:

```yaml
policy:
  wasm_modules:
    - "./policies/authorization_grant.wasm"
    - "./policies/client_registration.wasm"
    - "./policies/email.wasm"
    - "./policies/register.wasm"
  
  data:
    # Authorization grant policy data
    restricted_clients: []
    admin_users: ["admin"]
    admin_only_scopes: ["urn:matrix:org.matrix.msc2967.client:api:*"]
    
    # Client registration policy data
    allowed_redirect_domains:
      - "https://app.example.com"
      - "https://trusted.example.com"
    restricted_grant_types:
      - "client_credentials"
    
    # Email policy data
    blocked_domains:
      - "tempmail.com"
      - "throwaway.email"
    allowed_domains:
      - "example.com"
      - "company.com"
    require_corporate_email: false
    max_emails_per_user: 5
    
    # Registration policy data
    reserved_usernames:
      - "admin"
      - "root"
      - "system"
      - "support"
    blocked_words:
      - "spam"
      - "abuse"
    require_email: false
    require_corporate_email: false
    corporate_domains:
      - "example.com"
    min_username_length: 3
    max_username_length: 32
```

## Environment Variable Substitution

Configuration values can reference environment variables:

```yaml
database:
  uri: "${DATABASE_URL}"

email:
  transport: smtp
  hostname: "${SMTP_HOST}"
  username: "${SMTP_USER}"
  password: "${SMTP_PASSWORD}"

upstream_oauth2:
  providers:
    - id: "google"
      client_id: "${GOOGLE_CLIENT_ID}"
      client_secret: "${GOOGLE_CLIENT_SECRET}"
```

## Configuration Validation

The configuration system validates all settings on startup:

```rust
use mas_config::{RootConfig, ConfigurationSectionExt};
use figment::{Figment, providers::{Format, Yaml, Env}};

// Load configuration
let figment = Figment::new()
    .merge(Yaml::file("config.yaml"))
    .merge(Env::prefixed("MAS_"));

// Extract and validate
let config = RootConfig::extract(&figment)?;

// All configuration is now validated and ready to use
```

## Complete Configuration Example

```yaml
# MAS Configuration
# https://element-hq.github.io/matrix-authentication-service/

database:
  uri: "postgresql://mas:${DB_PASSWORD}@db.example.com/mas"
  max_connections: 20
  connect_timeout: 30s

http:
  public_base: "https://auth.example.com"
  issuer: "https://auth.example.com/"
  
  listeners:
    - name: web
      address: "0.0.0.0:8443"
      tls:
        certificate: "/etc/mas/cert.pem"
        key: "/etc/mas/key.pem"
      resources:
        - discovery
        - human
        - oauth2
        - graphql:
            playground: false
        - compat
    
    - name: internal
      address: "127.0.0.1:8081"
      resources:
        - admin
        - health

matrix:
  homeserver: "https://matrix.example.com"
  endpoint: "http://synapse:8008"
  secret: "${MATRIX_SHARED_SECRET}"

email:
  from: "\"Auth Service\" <noreply@example.com>"
  reply_to: "support@example.com"
  transport: smtp
  hostname: "smtp.gmail.com"
  port: 587
  mode: starttls
  username: "${SMTP_USER}"
  password: "${SMTP_PASSWORD}"

secrets:
  encryption: "${ENCRYPTION_KEY}"
  keys:
    - kid: "rsa-2024-01"
      key: "rsa:/etc/mas/keys/rsa-2024-01.pem"
    - kid: "ec-2024-01"
      key: "ecdsa:/etc/mas/keys/ec-2024-01.pem"

telemetry:
  tracing:
    exporter: otlp
    endpoint: "http://collector:4317"
  metrics:
    exporter: prometheus
    listen_address: "0.0.0.0:9090"
  sentry:
    dsn: "${SENTRY_DSN}"
    traces_sample_rate: 0.1

upstream_oauth2:
  providers:
    - id: "google"
      issuer: "https://accounts.google.com"
      client_id: "${GOOGLE_CLIENT_ID}"
      client_secret: "${GOOGLE_CLIENT_SECRET}"
      scope: "openid email profile"
      human_name: "Google"
      brand_name: "google"
      claims_imports:
        localpart:
          action: force
          template: "{{ user.email.split('@')[0] | lower | replace('.', '_') }}"
        email:
          action: force
          template: "{{ user.email }}"
        displayname:
          action: suggest
          template: "{{ user.name }}"

policy:
  wasm_modules:
    - "/etc/mas/policies/authorization_grant.wasm"
    - "/etc/mas/policies/client_registration.wasm"
    - "/etc/mas/policies/email.wasm"
    - "/etc/mas/policies/register.wasm"
  data:
    allowed_redirect_domains:
      - "https://app.example.com"
      - "https://element.example.com"
    allowed_domains:
      - "example.com"
    reserved_usernames:
      - "admin"
      - "root"
    max_emails_per_user: 3

session:
  ttl: 14d
  grace_period: 1h
  max_lifetime: 90d

passwords:
  enabled: true
  min_length: 12
  require_uppercase: true
  require_lowercase: true
  require_number: true
  require_special: false

captcha:
  service: recaptcha
  site_key: "${RECAPTCHA_SITE_KEY}"
  secret_key: "${RECAPTCHA_SECRET_KEY}"

account:
  email_change_allowed: true
  displayname_change_allowed: true
  password_change_allowed: true
  recovery_allowed: true
```

---

*Last Updated: 2025-10-30*
