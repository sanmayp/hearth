//! Hello module. Proves the factory path end to end:
//! migration, route, tool, widget, health.

use async_trait::async_trait;
use axum::{routing::get, Json, Router};
use hearth_core::{AppState, Health, JobDef, Migration, Module, ToolDef, WidgetDef};
use serde_json::{json, Value};

/// The hello module.
pub struct Hello;

#[async_trait]
impl Module for Hello {
    fn id(&self) -> &'static str {
        "hello"
    }

    fn migrations(&self) -> Vec<Migration> {
        vec![Migration {
            version: 1,
            name: "create_hello_ping",
            sql: "CREATE TABLE IF NOT EXISTS hello_ping
                  (id BIGSERIAL PRIMARY KEY, at TIMESTAMPTZ NOT NULL DEFAULT now())",
        }]
    }

    fn routes(&self, _state: &AppState) -> Router<()> {
        Router::new().route("/v1/hello", get(say_hello))
    }

    fn jobs(&self) -> Vec<JobDef> {
        vec![]
    }

    fn tools(&self) -> Vec<ToolDef> {
        vec![ToolDef {
            name: "hello.say",
            about: "Say hello. Test tool only.",
            schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        }]
    }

    fn widgets(&self) -> Vec<WidgetDef> {
        vec![WidgetDef {
            id: "hello.card",
            title: "Hello",
            module: "hello",
        }]
    }

    async fn health(&self, state: &AppState) -> Health {
        match sqlx::query("SELECT 1").fetch_one(&state.pool).await {
            Ok(_) => Health::ok("hello answers"),
            Err(e) => Health::down(format!("db down: {e}")),
        }
    }
}

/// GET /v1/hello. Answers with a fixed greeting.
async fn say_hello() -> Json<Value> {
    Json(json!({"ok": true, "module": "hello", "hello": "Hearth answers."}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_is_hello() {
        assert_eq!(Hello.id(), "hello");
    }

    #[test]
    fn one_ordered_migration() {
        let ms = Hello.migrations();
        assert_eq!(ms.len(), 1);
        assert_eq!(ms[0].version, 1);
    }

    #[test]
    fn tool_schema_is_valid_json_object() {
        let tools = Hello.tools();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].schema["type"], json!("object"));
    }

    #[tokio::test]
    async fn hello_answers() {
        let Json(v) = say_hello().await;
        assert_eq!(v["ok"], json!(true));
        assert_eq!(v["module"], json!("hello"));
    }
}
