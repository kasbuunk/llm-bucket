use llm_bucket::upload::{Uploader, NewExternalSource};

#[tokio::test]
async fn test_create_external_source_succeeds() {
    // Instantiate the real uploader implementation (for now: placeholder, will fail to compile)
    // You will need: use llm_bucket::upload::UploaderImpl; once implemented
    let uploader = llm_bucket::upload::UploaderImpl::new_from_env()
        .expect("Failed to create uploader from .env settings");

    let bucket_id: i32 = std::env::var("BUCKET_ID").expect("BUCKET_ID missing").parse().expect("BUCKET_ID must be i32");
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
    assert_eq!(ext_source.bucket_id, 1);
    assert_eq!(ext_source.external_source_name, "Test Source");
}
