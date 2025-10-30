// Copyright 2024, 2025 New Vector Ltd.
//
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Element-Commercial
// Please see LICENSE files in the repository root for full details.

//! Basic usage examples for mas-did
//!
//! This file demonstrates common DID operations.
//! Run with: cargo run --example basic_usage

use mas_did::{
    Credential, CredentialSubject, DIDManager, Issuer, Result, VerifiableCredential, DID,
};

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== DID and Verifiable Credentials Examples ===\n");

    // Example 1: Create and use a DID
    example_create_did().await?;

    // Example 2: Sign and verify data
    example_sign_verify().await?;

    // Example 3: Issue and verify credentials
    example_credentials().await?;

    // Example 4: DID resolution
    example_resolution().await?;

    println!("\n=== All examples completed successfully! ===");
    Ok(())
}

async fn example_create_did() -> Result<()> {
    println!("Example 1: Creating a DID\n");

    // Initialize DID manager
    let manager = DIDManager::new("example-app");

    // Create a did:peer:2 DID
    let (did, doc) = manager.create_peer_did().await?;

    println!("✓ Created DID: {}", did.id);
    println!("  Method: {}", did.method);
    println!("  Method-specific ID: {}", did.method_specific_id);

    // Print verification methods
    if let Some(methods) = &doc.verification_method {
        println!("\n  Verification Methods:");
        for method in methods {
            println!("    - {} ({})", method.id, method.method_type);
        }
    }

    // Cleanup
    manager.delete_did(&did)?;
    println!("\n✓ DID deleted from storage\n");

    Ok(())
}

async fn example_sign_verify() -> Result<()> {
    println!("Example 2: Signing and Verifying Data\n");

    let manager = DIDManager::new("example-app");
    let (did, _) = manager.create_peer_did().await?;

    // Data to sign
    let message = b"Hello, decentralized world!";
    println!("Message: {}", String::from_utf8_lossy(message));

    // Sign the message
    let signature = manager.sign(&did, message)?;
    println!("✓ Signature created ({} bytes)", signature.len());

    // Verify the signature
    let valid = manager.verify(&did, message, &signature)?;
    println!("✓ Signature verification: {}", if valid { "VALID" } else { "INVALID" });

    // Try verifying with wrong message
    let wrong_message = b"Different message";
    let invalid = manager.verify(&did, wrong_message, &signature)?;
    println!("✓ Wrong message verification: {}", if invalid { "VALID" } else { "INVALID" });

    // Cleanup
    manager.delete_did(&did)?;
    println!();

    Ok(())
}

async fn example_credentials() -> Result<()> {
    println!("Example 3: Verifiable Credentials\n");

    let manager = DIDManager::new("example-app");

    // Create issuer and subject DIDs
    let (issuer_did, _) = manager.create_peer_did().await?;
    let (subject_did, _) = manager.create_peer_did().await?;

    println!("✓ Issuer DID: {}", issuer_did.id);
    println!("✓ Subject DID: {}", subject_did.id);

    // Create a Matrix user credential
    let vc = Credential::matrix_user(
        issuer_did.id.clone(),
        subject_did.id.clone(),
        "@alice:example.com".to_string(),
        Some("Alice".to_string()),
    )
    .with_expiration(chrono::Utc::now() + chrono::Duration::days(365));

    println!("\n✓ Credential created");
    println!("  Type: {:?}", vc.credential_type);
    println!("  Subject: {}", vc.credential_subject.id);

    // Sign the credential
    let signed_vc = vc.sign(&manager, &issuer_did).await?;
    println!("\n✓ Credential signed");

    // Verify the credential
    let valid = signed_vc.verify(&manager).await?;
    println!("✓ Credential verification: {}", if valid { "VALID" } else { "INVALID" });

    // Check expiration
    let expired = signed_vc.is_expired();
    println!("✓ Credential expired: {}", if expired { "YES" } else { "NO" });

    // Create a custom credential
    let custom_claims = serde_json::json!({
        "role": "admin",
        "permissions": ["read", "write", "delete"],
        "level": 5
    });

    let custom_subject = CredentialSubject::new(subject_did.id.clone(), custom_claims);

    let custom_vc = VerifiableCredential::new(
        Issuer::DID(issuer_did.id.clone()),
        custom_subject,
        vec!["AdminCredential".to_string()],
    );

    let signed_custom = custom_vc.sign(&manager, &issuer_did).await?;
    println!("\n✓ Custom credential created and signed");

    // Access claims
    let role = signed_custom.credential_subject.claims["role"]
        .as_str()
        .unwrap_or("unknown");
    println!("  Role: {}", role);

    // Cleanup
    manager.delete_did(&issuer_did)?;
    manager.delete_did(&subject_did)?;
    println!();

    Ok(())
}

async fn example_resolution() -> Result<()> {
    println!("Example 4: DID Resolution\n");

    let manager = DIDManager::new("example-app");

    // Create a DID
    let (did, original_doc) = manager.create_peer_did().await?;
    println!("✓ Created DID: {}", did.id);

    // Resolve it back
    let resolved_doc = manager.resolve(&did).await?;
    println!("✓ Resolved DID document");

    // Compare
    println!("\n  Original methods: {}", 
        original_doc.verification_method.as_ref().map_or(0, |v| v.len()));
    println!("  Resolved methods: {}", 
        resolved_doc.verification_method.as_ref().map_or(0, |v| v.len()));

    // Parse a DID string
    let did_string = "did:peer:2.Ez6LSbysY2xFMRpGMhb7tFTLMpeuPRaqaWM1yECx2AtzE3KCc";
    let parsed = DID::parse(did_string)?;
    println!("\n✓ Parsed DID: {}", did_string);
    println!("  Method: {}", parsed.method);
    println!("  Method-specific ID: {}", parsed.method_specific_id);

    // Cleanup
    manager.delete_did(&did)?;

    Ok(())
}
