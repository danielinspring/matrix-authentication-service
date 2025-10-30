# Diagrams

This page provides high-level visualizations of the system using Mermaid. These complement the detailed API docs:

- Rust crate docs: [rustdoc](../rustdoc/)
- Admin API (OpenAPI): [Swagger UI](../api/index.html)
- Frontend components: [Storybook](../storybook/)
- Frontend TypeScript API: [TypeDoc](../tsdoc/)

## Backend crates and dependencies

```mermaid
graph LR
  subgraph Backend (Rust)
    A[crates/handlers] --> B[crates/http]
    A --> C[crates/data-model]
    A --> D[crates/storage]
    D --> E[crates/storage-pg]
    A --> F[crates/config]
    A --> G[crates/context]
    A --> H[crates/i18n]
    A --> I[crates/email]
    A --> J[crates/keystore]
    J --> K[crates/jose]
    A --> L[crates/oauth2-types]
    A --> M[crates/oidc-client]
    B --> N[crates/axum-utils]
    B --> O[crates/router]
    O --> A
    A --> P[crates/templates]
    A --> Q[crates/tasks]
    A --> R[crates/listener]
    A --> S[crates/tower]
    A --> T[crates/matrix]
    T --> U[crates/matrix-synapse]
    A --> V[crates/spa]
    A --> W[crates/policy]
  end
```

Notes:
- `crates/handlers` implements HTTP handlers and business logic, depending on configuration, storage, i18n, email, keystore, etc.
- `crates/http` wires Axum, routing, and middleware.
- `crates/storage` defines storage traits; `crates/storage-pg` provides the PostgreSQL implementation.

## Request flow (sequence)

```mermaid
sequenceDiagram
  participant C as Client
  participant RP as Router/Axum
  participant H as Handlers
  participant S as Storage (Postgres)
  participant O as OAuth2/JWT (jose)

  C->>RP: HTTP request
  RP->>H: Route match + middleware context
  H->>S: Query/update models
  S-->>H: Data/Result
  H->>O: Token/Signature operations
  O-->>H: Claims/JWT/JWS
  H-->>RP: Response payload
  RP-->>C: HTTP response
```

## Frontend routes (React)

```mermaid
graph TD
  R[__root.tsx]
  R --> A[_account.tsx]
  A --> A1[_account.index.tsx]
  A --> A2[_account.sessions.index.tsx]
  A --> A3[_account.sessions.browsers.tsx]
  A --> A4[_account.plan.index.tsx]
  R --> P[password.change.index.tsx]
  R --> P2[password.change.success.tsx]
  R --> PR[password.recovery.index.tsx]
  R --> RX[reset-cross-signing.tsx]
  RX --> RX1[reset-cross-signing.index.tsx]
  RX --> RX2[reset-cross-signing.success.tsx]
  RX --> RX3[reset-cross-signing.cancelled.tsx]
  R --> D[devices.$.tsx]
  R --> S[sessions.$id.tsx]
  R --> CL[clients.$id.tsx]
  R --> EM1[emails.$id.in-use.tsx]
  R --> EM2[emails.$id.verify.tsx]
```

## Admin API overview

```mermaid
graph LR
  subgraph Admin API (OpenAPI)
    U[Users]
    C[Clients]
    S[Sessions]
    E[Emails]
    D[Devices]
    A[Audit/Policy]
  end

  UI[Swagger UI] -->|OAuth2| Admin((Admin user))
  Admin -->|Calls| U
  Admin --> C
  Admin --> S
  Admin --> E
  Admin --> D
  Admin --> A
```

For full endpoint details and try-it-out, see the [Swagger UI](../api/index.html) and the raw [spec.json](../api/spec.json).
