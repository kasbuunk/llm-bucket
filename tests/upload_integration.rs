use llm_bucket::upload::{NewExternalSource, Uploader};

#[tokio::test]
async fn test_create_external_source_succeeds() {
    // Instantiate the real uploader implementation (for now: placeholder, will fail to compile)
    // You will need: use llm_bucket::upload::UploaderImpl; once implemented
    let uploader = llm_bucket::upload::UploaderImpl::new_from_env()
        .expect("Failed to create uploader from .env settings");

    let bucket_id: i32 = std::env::var("BUCKET_ID")
        .expect("BUCKET_ID missing")
        .parse()
        .expect("BUCKET_ID must be i32");
    let req = NewExternalSource {
        name: "Test Source",
        bucket_id,
    };

    let result = uploader.create_source(req).await;

    assert!(
        result.is_ok(),
        "Expected successful result from create_source, but got error: {:?}",
        result.as_ref().err()
    );
    let ext_source = result.unwrap();
    assert_eq!(ext_source.bucket_id, bucket_id);
    assert_eq!(ext_source.external_source_name, "Test Source");
}

#[tokio::test]
async fn test_create_external_item_succeeds() {
    // Use real uploader implementation; this will panic if create_item is not implemented
    let uploader = llm_bucket::upload::UploaderImpl::new_from_env()
        .expect("Failed to create uploader from .env settings");

    let bucket_id: i32 = std::env::var("BUCKET_ID")
        .expect("BUCKET_ID missing")
        .parse()
        .expect("BUCKET_ID must be i32");

    // First, create an external source to add the item to
    let req_source = llm_bucket::upload::NewExternalSource {
        name: "Item Test Source",
        bucket_id,
    };
    let source_result = uploader.create_source(req_source).await.expect("Creating source failed");
    let external_source_id = source_result.external_source_id as i64;

    // Build the new external item request
    let req_item = llm_bucket::upload::NewExternalItem {
        content: "This is a test file content.",
        url: "file:///tmp/test_file.txt",
        bucket_id: bucket_id as i64,
        external_source_id,
        processing_state: None,
    };

    let item_result = uploader.create_item(req_item).await;

    assert!(
        item_result.is_ok(),
        "Expected successful result from create_item, but got error: {:?}",
        item_result.as_ref().err()
    );

    // Optional: match returned item fields etc once implemented
}
