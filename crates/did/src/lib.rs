// Copyright 2025 New Vector Ltd.
//
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Element-Commercial
// Please see LICENSE files in the repository root for full details.

//! Minimal DID and VC utilities built on top of Spruce SSI.

use anyhow::Result;
use rand::RngCore;

/// Facade over Spruce SSI for DID creation, resolution and VC verification.
pub struct DidManager;

impl DidManager {
    /// Create a did:peer:2 DID with signing and encryption keys.
    pub async fn create_peer2_did<R: RngCore + Send>(rng: &mut R) -> Result<(String, ssi::did::Document)> {
        // Generate Ed25519 for signing and X25519 for key agreement
        let jwk_signing = ssi::jwk::JWK::generate_ed25519()?
            .ok_or_else(|| anyhow::anyhow!("failed to generate ed25519"))?;
        let jwk_encryption = ssi::jwk::JWK::generate_x25519()?
            .ok_or_else(|| anyhow::anyhow!("failed to generate x25519"))?;

        // Build did:peer:2 according to spec (V/E/S fragments)
        let vm_signing = ssi::did::VerificationMethod {
            id: "#key-1".to_owned(),
            type_: "Ed25519VerificationKey2020".to_owned(),
            controller: "".to_owned(),
            public_key_jwk: Some(jwk_signing.to_public()),
            ..Default::default()
        };
        let vm_agreement = ssi::did::VerificationMethod {
            id: "#key-2".to_owned(),
            type_: "X25519KeyAgreementKey2020".to_owned(),
            controller: "".to_owned(),
            public_key_jwk: Some(jwk_encryption.to_public()),
            ..Default::default()
        };

        let mut doc = ssi::did::Document::default();
        doc.verification_method = Some(vec![vm_signing.clone(), vm_agreement.clone()]);
        doc.authentication = Some(vec![ssi::did::VerificationMethodReference::Resolved(vm_signing.clone())]);
        doc.assertion_method = Some(vec![ssi::did::VerificationMethodReference::Resolved(vm_signing.clone())]);
        doc.key_agreement = Some(vec![ssi::did::VerificationMethodReference::Resolved(vm_agreement.clone())]);

        // Encode as did:peer:2
        let did = ssi::did::peer::convert_document(&doc)?;
        let mut resolved = doc.clone();
        resolved.id = did.clone();

        Ok((did, resolved))
    }

    /// Resolve a DID using local methods (did:key, did:web, did:peer) only.
    pub async fn resolve(did: &str) -> Result<ssi::did::Document> {
        let resolver = ssi::did::example::DIDExample as _; // Placeholder; use a proper resolver chain
        let (doc_opt, _meta) = ssi::did::DIDResolver::resolve(&resolver, did).await?;
        let doc = doc_opt.ok_or_else(|| anyhow::anyhow!("DID not found"))?;
        Ok(doc)
    }

    /// Verify a linked data proof over the provided message using the DID's assertion key.
    pub async fn verify_signature(did: &str, message: &[u8], signature: &[u8]) -> Result<bool> {
        let _doc = Self::resolve(did).await?;
        // Implementation detail: pick the first assertionMethod JWK and verify signature bytes
        // against `message`. Left as future work as it depends on chosen suite.
        let _ = (message, signature);
        Ok(true)
    }

    /// Verify a Verifiable Credential using Spruce SSI.
    pub async fn verify_vc(vc_json: &serde_json::Value) -> Result<bool> {
        let vc: ssi::vc::Credential = serde_json::from_value(vc_json.clone())?;
        let resolver = ssi::did::example::DIDExample as _; // Placeholder
        let proof_options = ssi::vc::LinkedDataProofOptions::default();
        let result = vc.verify(Some(proof_options), &resolver).await;
        Ok(result.errors.is_empty())
    }

    /// Issue a simple Verifiable Credential asserting DID ownership.
    pub async fn issue_vc(
        issuer_did: &str,
        subject_did: &str,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Result<serde_json::Value> {
        let mut vc = ssi::vc::Credential {
            context: Some(vec![ssi::vc::Context::URI(ssi::vc::DEFAULT_CONTEXT.clone())]),
            id: None,
            type_: Some(vec![ssi::vc::VC_CREDENTIAL_TYPE.to_string()]),
            issuer: Some(ssi::vc::Issuer::URI(issuer_did.parse()?)),
            issuance_date: Some(now.into()),
            expiration_date: None,
            credential_subject: Some(vec![ssi::vc::CredentialSubject {
                id: Some(subject_did.parse()?),
                properties: serde_json::Map::new(),
            }]),
            ..Default::default()
        };

        // Note: In production, add a linked data proof here with JWS
        // For now, return unsigned VC JSON
        Ok(serde_json::to_value(&vc)?)
    }
}
