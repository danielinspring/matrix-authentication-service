// Copyright 2025 New Vector Ltd.
//
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Element-Commercial
// Please see LICENSE files in the repository root for full details.

use mas_data_model::{Clock, clock::MockClock};
use mas_storage::{Pagination, RepositoryAccess, did::UserDidLinkFilter};
use rand::SeedableRng;
use rand_chacha::ChaChaRng;
use sqlx::PgPool;

use crate::PgRepository;

#[sqlx::test(migrator = "crate::MIGRATOR")]
async fn test_user_did_repo(pool: PgPool) {
    let mut repo = PgRepository::from_pool(&pool).await.unwrap().boxed();
    let mut rng = ChaChaRng::seed_from_u64(42);
    let clock = MockClock::default();

    let user = repo
        .user()
        .add(&mut rng, &clock, "alice".to_owned())
        .await
        .unwrap();

    // Add a DID link without association
    let did = "did:example:alice".to_owned();
    let link = repo
        .user_did_link()
        .add(&mut rng, &clock, did.clone(), Some("example".to_owned()))
        .await
        .unwrap();

    assert_eq!(link.did, did);
    assert!(link.user_id.is_none());

    // Associate to user
    repo.user_did_link()
        .associate_to_user(&link, &user)
        .await
        .unwrap();

    // Lookup by did
    let found = repo.user_did_link().find_by_did(&did).await.unwrap().unwrap();
    assert_eq!(found.did, did);
    assert_eq!(found.user_id, Some(user.id));

    // List by user
    let list = repo
        .user_did_link()
        .list(UserDidLinkFilter::new().for_user(&user), Pagination::first(10))
        .await
        .unwrap();
    assert_eq!(list.edges.len(), 1);
    assert_eq!(list.edges[0].node.did, did);

    // Remove
    repo.user_did_link().remove(found).await.unwrap();

    // Should be gone
    assert!(repo.user_did_link().find_by_did(&did).await.unwrap().is_none());
}
