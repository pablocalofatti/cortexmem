use std::sync::Arc;

use cortexmem::db::Database;
use cortexmem::mcp::CortexMemServer;

fn test_server() -> Arc<CortexMemServer> {
    let db = Database::open_in_memory().unwrap();
    Arc::new(CortexMemServer::new(db, None))
}

async fn start_test_server() -> String {
    let server = test_server();
    let app = cortexmem::http::build_router(server);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    format!("http://{addr}")
}

#[tokio::test]
async fn should_return_health_check() {
    let base = start_test_server().await;
    let resp = reqwest::get(format!("{base}/health")).await.unwrap();
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "ok");
    assert!(body["version"].is_string());
}

#[tokio::test]
async fn should_create_and_get_observation() {
    let base = start_test_server().await;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{base}/observations"))
        .json(&serde_json::json!({
            "project": "testproj",
            "title": "Auth decision",
            "content": "Using JWT for stateless auth",
            "type": "decision",
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);
    let body: serde_json::Value = resp.json().await.unwrap();
    let id = body["id"].as_i64().unwrap();
    assert!(id > 0);
    assert_eq!(body["status"], "saved");

    let resp = client
        .get(format!("{base}/observations/{id}"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let obs: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(obs["title"], "Auth decision");
}

#[tokio::test]
async fn should_return_404_for_missing_observation() {
    let base = start_test_server().await;
    let resp = reqwest::get(format!("{base}/observations/99999"))
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);
}

#[tokio::test]
async fn should_soft_delete_observation() {
    let base = start_test_server().await;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{base}/observations"))
        .json(&serde_json::json!({
            "project": "p",
            "title": "to delete",
            "content": "ephemeral",
            "type": "discovery",
        }))
        .send()
        .await
        .unwrap();
    let id = resp.json::<serde_json::Value>().await.unwrap()["id"]
        .as_i64()
        .unwrap();

    let resp = client
        .delete(format!("{base}/observations/{id}"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["mode"], "soft");
}

#[tokio::test]
async fn should_hard_delete_observation() {
    let base = start_test_server().await;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{base}/observations"))
        .json(&serde_json::json!({
            "project": "p",
            "title": "hard delete me",
            "content": "gone",
            "type": "discovery",
        }))
        .send()
        .await
        .unwrap();
    let id = resp.json::<serde_json::Value>().await.unwrap()["id"]
        .as_i64()
        .unwrap();

    let resp = client
        .delete(format!("{base}/observations/{id}?hard=true"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["mode"], "hard");

    let resp = client
        .get(format!("{base}/observations/{id}"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);
}

#[tokio::test]
async fn should_search_observations() {
    let base = start_test_server().await;
    let client = reqwest::Client::new();

    client
        .post(format!("{base}/observations"))
        .json(&serde_json::json!({
            "project": "searchproj",
            "title": "Database indexing",
            "content": "Add index on user_id for faster queries",
            "type": "pattern",
        }))
        .send()
        .await
        .unwrap();

    let resp = client
        .get(format!("{base}/search?q=index&project=searchproj"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let results: serde_json::Value = resp.json().await.unwrap();
    assert!(results.as_array().unwrap().len() > 0);
}

#[tokio::test]
async fn should_create_session() {
    let base = start_test_server().await;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{base}/sessions"))
        .json(&serde_json::json!({
            "project": "myproj",
            "directory": "/tmp/myproj",
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["session_id"].as_i64().unwrap() > 0);
}

#[tokio::test]
async fn should_get_stats() {
    let base = start_test_server().await;
    let resp = reqwest::get(format!("{base}/stats")).await.unwrap();
    assert_eq!(resp.status(), 200);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["total"].is_number());
}

#[tokio::test]
async fn should_save_and_retrieve_prompt() {
    let base = start_test_server().await;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{base}/prompts"))
        .json(&serde_json::json!({
            "content": "Fix the login bug",
            "project": "myproj",
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(body["id"].as_i64().unwrap() > 0);

    let resp = client
        .get(format!("{base}/prompts/recent?project=myproj"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let prompts: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(prompts.as_array().unwrap().len(), 1);
    assert_eq!(prompts[0]["content"], "Fix the login bug");
}
