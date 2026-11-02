//! Migration gate test. Runs EVERY module's migrations
//! against a throwaway Postgres, twice, and checks
//! each table exists. Uses TEST_PG_URL when set,
//! else a testcontainers Postgres. Skips LOUDLY when
//! no database is reachable (never silent).

use hearth_core::{run_migrations, Module};
use sqlx::PgPool;

/// Tables each migration must create.
const TABLES: &[&str] = &[
    "hello_ping",
    "today_snapshot",
    "reminders",
    "tasks",
    "items",
    "rooms",
    "health_checkin",
    "business_cache",
    "_hearth_migrations",
];

/// Holds the container alive while the test runs.
/// Owns it only; never reads it.
#[allow(dead_code)]
enum Hold {
    Container(Box<testcontainers::ContainerAsync<testcontainers::GenericImage>>),
    None,
}

/// Connect to a throwaway DB. None means skip (loud).
async fn setup() -> Option<(Hold, PgPool)> {
    if let Ok(url) = std::env::var("TEST_PG_URL") {
        let pool = PgPool::connect(&url)
            .await
            .expect("TEST_PG_URL must connect");
        return Some((Hold::None, pool));
    }
    match start_container().await {
        Ok((hold, pool)) => Some((hold, pool)),
        Err(e) => {
            eprintln!("SKIP migrations_pg: no Postgres reachable ({e}).");
            eprintln!("SKIP migrations_pg: set TEST_PG_URL or start docker/podman.");
            None
        }
    }
}

/// Start a throwaway Postgres container.
async fn start_container() -> Result<(Hold, PgPool), String> {
    use testcontainers::{core::WaitFor, runners::AsyncRunner, GenericImage, ImageExt};
    let image = GenericImage::new("postgres", "16-alpine")
        .with_wait_for(WaitFor::message_on_stderr(
            "database system is ready to accept connections",
        ))
        .with_env_var("POSTGRES_PASSWORD", "hearth")
        .with_env_var("POSTGRES_DB", "hearth_test");
    let node = image.start().await.map_err(|e| e.to_string())?;
    let port = node
        .get_host_port_ipv4(5432)
        .await
        .map_err(|e| e.to_string())?;
    let url = format!("postgres://postgres:hearth@127.0.0.1:{port}/hearth_test");
    let pool = PgPool::connect(&url).await.map_err(|e| e.to_string())?;
    Ok((Hold::Container(Box::new(node)), pool))
}

/// All modules' migrations apply cleanly and rerun cleanly.
#[tokio::test]
async fn all_migrations_apply_and_rerun_cleanly() {
    let Some((_hold, pool)) = setup().await else {
        return;
    };
    let hello = hello::Hello;
    let today = today::Today;
    let reminders = reminders::Reminders;
    let tasks = tasks::Tasks;
    let inventory = inventory::Inventory;
    let health = health::HealthMod;
    let business = business::Business;
    let modules = [
        (hello.id(), hello.migrations()),
        (today.id(), today.migrations()),
        (reminders.id(), reminders.migrations()),
        (tasks.id(), tasks.migrations()),
        (inventory.id(), inventory.migrations()),
        (health.id(), health.migrations()),
        (business.id(), business.migrations()),
    ];
    run_migrations(&pool, &modules)
        .await
        .expect("first apply works");
    run_migrations(&pool, &modules).await.expect("rerun works");
    for table in TABLES {
        let found: (Option<String>,) = sqlx::query_as("SELECT to_regclass($1)::text")
            .bind(format!("public.{table}"))
            .fetch_one(&pool)
            .await
            .expect("to_regclass works");
        assert!(found.0.is_some(), "table {table} must exist");
    }
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM _hearth_migrations")
        .fetch_one(&pool)
        .await
        .expect("count works");
    assert_eq!(count.0, 8, "one version row per module");
}
