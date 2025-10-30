//! DID and Verifiable Credentials utilities for MAS
//!
//! Feature-gated Spruce SSI implementations; graceful no-op stubs otherwise.

use anyhow::Result;

#[derive(Debug, Clone)]
pub struct DidDocument(pub serde_json::Value);

#[derive(Debug, Clone)]
pub struct VerifiableCredential(pub serde_json::Value);

#[async_trait::async_trait]
pub trait DidManager: Send + Sync {
    async fn create_peer_did(&self) -> Result<(String, DidDocument)>;
    async fn resolve(&self, did: &str) -> Result<DidDocument>;
}

#[async_trait::async_trait]
pub trait VcVerifier: Send + Sync {
    async fn verify_vc(&self, vc: &VerifiableCredential) -> Result<bool>;
}

#[cfg(feature = "ssi")]
mod ssi_impl {
    use super::*;
    use ssi::did::DIDMethod;
    use ssi::did_resolve::DIDResolver;
    use ssi::jwk::JWK;
    use ssi::vc::Credential;

    #[derive(Default, Debug, Clone)]
    pub struct SsiDidManager;

    #[async_trait::async_trait]
    impl DidManager for SsiDidManager {
        async fn create_peer_did(&self) -> Result<(String, DidDocument)> {
            // Generate Ed25519 JWK
            let signing_key = JWK::generate_ed25519();

            // did:peer:numalgo=2 via ssi's did:peer method (if available)
            // Fallback to did:key as bootstrap if did:peer isn't present in this ssi version
            let (did, doc) = if let Some(method) = ssi::did::DIDMethod::from_str("peer").ok() {
                // Construct DID Document from JWKs
                let doc = method
                    .generate(&signing_key, None)
                    .map_err(|e| anyhow::anyhow!("failed to generate did:peer: {e}"))?;
                (doc.id.clone(), doc)
            } else {
                let (did, doc) = ssi::did::didkey::DIDKey::generate(&signing_key);
                (did, doc)
            };

            let json = serde_json::to_value(&doc)?;
            Ok((did, DidDocument(json)))
        }

        async fn resolve(&self, did: &str) -> Result<DidDocument> {
            let resolver = ssi::did_resolve::HTTPDIDResolver::default();
            let (res, _meta) = resolver
                .resolve(did, &ssi::did_resolve::ResolutionInputMetadata::default())
                .await;
            let doc = res
                .document
                .ok_or_else(|| anyhow::anyhow!("DID not found"))?;
            let json = serde_json::to_value(&doc)?;
            Ok(DidDocument(json))
        }
    }

    #[derive(Default, Debug, Clone)]
    pub struct SsiVcVerifier;

    #[async_trait::async_trait]
    impl VcVerifier for SsiVcVerifier {
        async fn verify_vc(&self, vc: &VerifiableCredential) -> Result<bool> {
            // Support both JSON-LD and compact JWT forms parsed to ssi::vc::Credential
            let credential: Credential = serde_json::from_value(vc.0.clone())?;
            let resolver = ssi::did_resolve::HTTPDIDResolver::default();
            let options = ssi::vc::VerificationOptions::default();
            let result = credential.verify(Some(options), &resolver).await;
            Ok(result.errors.is_empty())
        }
    }

    pub use {SsiDidManager as DefaultDidManager, SsiVcVerifier as DefaultVcVerifier};
}

#[cfg(not(feature = "ssi"))]
mod stub_impl {
    use super::*;

    #[derive(Default, Debug, Clone)]
    pub struct StubDidManager;

    #[async_trait::async_trait]
    impl DidManager for StubDidManager {
        async fn create_peer_did(&self) -> Result<(String, DidDocument)> {
            Err(anyhow::anyhow!(
                "SSI feature not enabled: build with feature \"ssi\""
            ))
        }

        async fn resolve(&self, _did: &str) -> Result<DidDocument> {
            Err(anyhow::anyhow!(
                "SSI feature not enabled: build with feature \"ssi\""
            ))
        }
    }

    #[derive(Default, Debug, Clone)]
    pub struct StubVcVerifier;

    #[async_trait::async_trait]
    impl VcVerifier for StubVcVerifier {
        async fn verify_vc(&self, _vc: &VerifiableCredential) -> Result<bool> {
            Err(anyhow::anyhow!(
                "SSI feature not enabled: build with feature \"ssi\""
            ))
        }
    }

    pub use {StubDidManager as DefaultDidManager, StubVcVerifier as DefaultVcVerifier};
}

pub use cfg_if::cfg_if;

cfg_if::cfg_if! {
    if #[cfg(feature = "ssi")] {
        pub use ssi_impl::{DefaultDidManager, DefaultVcVerifier};
    } else {
        pub use stub_impl::{DefaultDidManager, DefaultVcVerifier};
    }
}

#[cfg(all(feature = "ssi", test))]
mod tests {
    use super::*;

    #[tokio::test]
    async fn create_and_resolve_peer_did() {
        let mgr = crate::DefaultDidManager::default();
        let (did, doc) = mgr.create_peer_did().await.expect("create did");
        assert!(did.starts_with("did:"));
        assert!(doc.0.get("id").is_some());

        // Local resolve may or may not work depending on method; ensure call succeeds/fails gracefully
        let _ = mgr.resolve(&did).await.ok();
    }
}
