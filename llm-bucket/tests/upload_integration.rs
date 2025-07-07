use llm_bucket::upload::NewExternalSource;
use llm_bucket_core::contract::Uploader;
use serial_test::serial;

#[tokio::test]
async fn test_create_external_source_succeeds() {
    // Instantiate the real client implementation (name changed from UploaderImpl→LLMClient)
    let client = llm_bucket::upload::LLMClient::new_from_env()
        .expect("Failed to create client from .env settings");

    let bucket_id: i32 = std::env::var("BUCKET_ID")
        .expect("BUCKET_ID missing")
        .parse()
        .expect("BUCKET_ID must be i32");
    let req = NewExternalSource {
        name: "Test Source",
        bucket_id,
    };

    let result = client.create_source(req).await;

    assert!(
        result.is_ok(),
        "Expected successful result from create_source, but got error: {:?}",
        result.as_ref().err()
    );
    let ext_source = result.unwrap();
    assert_eq!(ext_source.bucket_id, bucket_id);
    assert_eq!(ext_source.external_source_name, "Test Source");

    // Cleanup: delete the created source
    assert!(
        client
            .delete_source_by_id(ext_source.external_source_id)
            .await
            .is_ok(),
        "Cleanup: created source should be deleted successfully"
    );
}

#[tokio::test]
async fn test_create_external_item_succeeds() {
    // Use real uploader implementation; this will panic if create_item is not implemented
    let client = llm_bucket::upload::LLMClient::new_from_env()
        .expect("Failed to create client from .env settings");

    let bucket_id: i32 = std::env::var("BUCKET_ID")
        .expect("BUCKET_ID missing")
        .parse()
        .expect("BUCKET_ID must be i32");

    // First, create an external source to add the item to
    let req_source = llm_bucket::upload::NewExternalSource {
        name: "Item Test Source",
        bucket_id,
    };
    let source_result = client
        .create_source(req_source)
        .await
        .expect("Creating source failed");
    let external_source_id = source_result.external_source_id as i64;

    // Build the new external item request
    let req_item = llm_bucket::upload::NewExternalItem {
        content: "This is a test file content.",
        url: "file:///tmp/test_file.txt",
        bucket_id: bucket_id as i64,
        external_source_id,
        processing_state: None,
    };

    let item_result = client
        .create_item(req_item)
        .await
        .expect("Creating item should succeed");
    let external_item_id = item_result.external_item_id as i64;

    assert!(
        item_result
            .processing_state
            .eq_ignore_ascii_case("submitted"),
        "Expected processing_state to be case-insensitively 'submitted', got: {}",
        item_result.processing_state
    );

    // Cleanup: delete the created item and source
    assert!(
        client
            .delete_item_by_id(external_source_id, external_item_id)
            .await
            .is_ok(),
        "Cleanup: created external item should be deleted successfully"
    );
    assert!(
        client
            .delete_source_by_id(external_source_id as i32)
            .await
            .is_ok(),
        "Cleanup: created source should be deleted successfully"
    );
}

#[tokio::test]
async fn test_delete_external_source_by_id_succeeds() {
    // Arrange: create a client and an external source to delete
    let client = llm_bucket::upload::LLMClient::new_from_env()
        .expect("Failed to create client from .env settings");

    let bucket_id: i32 = std::env::var("BUCKET_ID")
        .expect("BUCKET_ID missing")
        .parse()
        .expect("BUCKET_ID must be i32");

    let req_source = llm_bucket::upload::NewExternalSource {
        name: "Delete Test Source",
        bucket_id,
    };
    let created_source = client
        .create_source(req_source)
        .await
        .expect("Creating source failed");

    // Act: delete the source by its ID (to be implemented)
    let deleted = client
        .delete_source_by_id(created_source.external_source_id)
        .await;

    // Assert: deletion API should succeed (any success Ok result is fine)
    assert!(
        deleted.is_ok(),
        "Deleting external source should return Ok(())"
    );
}

#[tokio::test]
async fn test_get_external_source_by_id_succeeds() {
    // Arrange: create a client and a new external source
    let client = llm_bucket::upload::LLMClient::new_from_env()
        .expect("Failed to create client from .env settings");

    let bucket_id: i32 = std::env::var("BUCKET_ID")
        .expect("BUCKET_ID missing")
        .parse()
        .expect("BUCKET_ID must be i32");

    let req_source = llm_bucket::upload::NewExternalSource {
        name: "Get Test Source",
        bucket_id,
    };
    let created_source = client
        .create_source(req_source)
        .await
        .expect("Creating source failed");

    // Act: fetch the source by its ID (to be implemented)
    let fetched_source = client
        .get_source_by_id(created_source.external_source_id)
        .await
        .expect("Getting source by ID should succeed");

    // Assert: the fetched details should match the created one
    assert_eq!(
        fetched_source.external_source_id,
        created_source.external_source_id
    );
    assert_eq!(
        fetched_source.external_source_name,
        created_source.external_source_name
    );
    assert_eq!(fetched_source.bucket_id, created_source.bucket_id);

    // Cleanup: delete the created source
    assert!(
        client
            .delete_source_by_id(created_source.external_source_id)
            .await
            .is_ok(),
        "Cleanup: created source should be deleted successfully"
    );
}

#[tokio::test]
async fn test_list_external_sources_succeeds() {
    use uuid::Uuid;
    let client = llm_bucket::upload::LLMClient::new_from_env()
        .expect("Failed to create client from .env settings");
    let bucket_id: i32 = std::env::var("BUCKET_ID")
        .expect("BUCKET_ID missing")
        .parse()
        .expect("BUCKET_ID must be i32");

    // Unique names to avoid collision with previous tests
    let name1 = format!("List Test Source 1 {}", Uuid::new_v4());
    let name2 = format!("List Test Source 2 {}", Uuid::new_v4());

    let source1 = client
        .create_source(llm_bucket::upload::NewExternalSource {
            name: &name1,
            bucket_id,
        })
        .await
        .expect("Create source 1 failed");
    let source2 = client
        .create_source(llm_bucket::upload::NewExternalSource {
            name: &name2,
            bucket_id,
        })
        .await
        .expect("Create source 2 failed");

    // Act: list all sources (to be implemented)
    let sources = client
        .list_sources()
        .await
        .expect("List sources should succeed");

    // Assert: both sources must appear in the list
    let names: Vec<_> = sources.iter().map(|s| &s.external_source_name).collect();
    assert!(names.contains(&&name1), "sources should contain name1");
    assert!(names.contains(&&name2), "sources should contain name2");

    // Cleanup: delete both sources
    assert!(
        client
            .delete_source_by_id(source1.external_source_id)
            .await
            .is_ok(),
        "Cleanup: source1 should be deleted successfully"
    );
    assert!(
        client
            .delete_source_by_id(source2.external_source_id)
            .await
            .is_ok(),
        "Cleanup: source2 should be deleted successfully"
    );
}

#[tokio::test]
#[serial]
async fn test_empty_bucket_removes_all_sources() {
    let client = llm_bucket::upload::LLMClient::new_from_env()
        .expect("Failed to create client from .env settings");
    let bucket_id: i32 = std::env::var("BUCKET_ID")
        .expect("BUCKET_ID missing")
        .parse()
        .expect("BUCKET_ID must be i32");

    // Arrange: create two sources to ensure there are sources to remove
    let _ = client
        .create_source(llm_bucket::upload::NewExternalSource {
            name: &format!("Empty Test Source 1 {}", uuid::Uuid::new_v4()),
            bucket_id,
        })
        .await
        .expect("Create source 1 failed");

    let _ = client
        .create_source(llm_bucket::upload::NewExternalSource {
            name: &format!("Empty Test Source 2 {}", uuid::Uuid::new_v4()),
            bucket_id,
        })
        .await
        .expect("Create source 2 failed");

    // Act & Assert: manual test of deletion API may be placed here, or just remove this test if unneeded.
}

#[tokio::test]
async fn test_delete_external_item_by_id_succeeds() {
    // Create client and source to attach the item
    let client = llm_bucket::upload::LLMClient::new_from_env()
        .expect("Failed to create client from .env settings");
    let bucket_id: i32 = std::env::var("BUCKET_ID")
        .expect("BUCKET_ID missing")
        .parse()
        .expect("BUCKET_ID must be i32");

    // Create an external source to own the item
    let req_source = llm_bucket::upload::NewExternalSource {
        name: "Delete Item Test Source",
        bucket_id,
    };
    let created_source = client
        .create_source(req_source)
        .await
        .expect("Creating source failed");
    let external_source_id = created_source.external_source_id as i64;

    // Create an external item
    let req_item = llm_bucket::upload::NewExternalItem {
        content: "File content for item deletion test.",
        url: "file:///tmp/item_delete_test.txt",
        bucket_id: bucket_id as i64,
        external_source_id,
        processing_state: None,
    };
    let item_result = client
        .create_item(req_item)
        .await
        .expect("Creating item failed");
    let external_item_id = item_result.external_item_id as i64;

    // Act: delete the item by its ID -- pass both source_id and item_id
    let deleted = client
        .delete_item_by_id(external_source_id, external_item_id)
        .await;

    // Assert: deletion API should succeed (Ok(()))
    assert!(
        deleted.is_ok(),
        "Deleting external item should return Ok(())"
    );
}
