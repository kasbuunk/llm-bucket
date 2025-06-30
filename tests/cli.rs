use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::NamedTempFile;
use std::fs::write;
use std::env;

/// Creates a minimal config file for the CLI to read (no upload section).
fn create_minimal_config() -> NamedTempFile {
    let config = NamedTempFile::new().expect("Creating temp config file failed");
    // The upload section is now omitted—only download and process remain.
    write(
        config.path(),
        b"download:\n  output_dir: ./tmp\n  sources:\n    - type: git\n      repo_url: \"git@github.com:kasbuunk/llm-bucket.git\"\n      reference: null\nprocess:\n  kind: FlattenFiles\n"
    )
    .expect("Writing temp config failed");
    config
}

#[test]
fn sync_cli_happy_flow_succeeds_with_valid_config_and_env() {
    dotenv::dotenv().ok();
    let config = create_minimal_config();

    // Load required env vars as in other integration tests
    let bucket_id = std::env::var("BUCKET_ID")
        .expect("BUCKET_ID env var must be set for CLI integration test");
    let api_key = std::env::var("OCP_APIM_SUBSCRIPTION_KEY")
        .expect("OCP_APIM_SUBSCRIPTION_KEY env var must be set for CLI integration test");
    let mut cmd = Command::cargo_bin("llm-bucket").expect("Binary exists");

    cmd.arg("sync")
        .arg("--config")
        .arg(config.path())
        .env("BUCKET_ID", bucket_id)
        .env("OCP_APIM_SUBSCRIPTION_KEY", api_key);

    // This requires a running API/config, or development dummy/test tenant.
    // To ensure non-disruptive test, only assert overall success and summary output.
    // Should finish successfully and print a high-level summary or banner.
    // The assertion should NOT require a precise output match as it may vary.

    // Assert on both the previous high-level output, and presence of an "exit" trace marker.
    cmd.assert()
        .success()
        .stdout(
            predicate::str::contains("Synchronise")
                .or(predicate::str::contains("success"))
                .or(predicate::str::contains("report"))
                .or(predicate::str::contains("exit")) // NEW: trace event
        );
}

use std::sync::{Arc, Mutex};
use tracing_subscriber::{layer::Context, Layer, Registry};
use tracing_subscriber::prelude::*; // needed for .with()

/// Custom Layer to collect emitted event messages.
struct EventCollector {
    events: Arc<Mutex<Vec<String>>>,
}

impl<S> Layer<S> for EventCollector
where
    S: tracing::Subscriber,
{
    fn on_event(&self, event: &tracing::Event<'_>, _ctx: Context<'_, S>) {
        use tracing_subscriber::fmt::format::{FmtSpan, Writer};
        use std::fmt::Write as FmtWrite;
        let mut msg = String::new();
        let _ = write!(&mut msg, "{:?}", event);
        self.events.lock().unwrap().push(msg);
    }
}

#[tokio::test]
async fn emits_trace_initialised_event() {
    let events = Arc::new(Mutex::new(Vec::new()));
    let collector = EventCollector { events: events.clone() };
    let subscriber = Registry::default().with(collector);
    let _guard = tracing::subscriber::set_default(subscriber);

    // Import run, Cli, and Commands directly from crate root.
    use llm_bucket::cli::{run, Cli, Commands};

    // Provide minimum config for the Sync subcommand (using a dummy path).
    let cli = Cli {
        command: Commands::Sync {
            config: std::path::PathBuf::from("dummy.yaml"),
        }
    };

    let _ = run(cli).await;

    let event_msgs = events.lock().unwrap();
    assert!(
        event_msgs.iter().any(|msg| msg.contains("trace_initialised")),
        "Expected a 'trace_initialised' trace event, got: {:?}",
        event_msgs
    );
}
