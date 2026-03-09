use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use cortexmem::db::Database;
use cortexmem::mcp::CortexMemServer;
use http_body_util::BodyExt;
use tower::ServiceExt;

fn test_app() -> axum::Router {
    let db = Database::open_in_memory().unwrap();
    let server = Arc::new(CortexMemServer::new(db, None));
    cortexmem::http::build_router(server)
}

async fn body_json(resp: axum::response::Response) -> serde_json::Value {
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn should_return_health_check() {
    let app = test_app();
    let resp = app
        .oneshot(Request::get("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["status"], "ok");
    assert!(body["version"].is_string());
}

#[tokio::test]
async fn should_create_and_get_observation() {
    let app = test_app();

    let resp = app
        .clone()
        .oneshot(
            Request::post("/observations")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "project": "testproj",
                        "title": "Auth decision",
                        "content": "Using JWT for stateless auth",
                        "type": "decision",
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = body_json(resp).await;
    let id = body["id"].as_i64().unwrap();
    assert!(id > 0);
    assert_eq!(body["status"], "saved");

    let resp = app
        .oneshot(
            Request::get(format!("/observations/{id}").as_str())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let obs = body_json(resp).await;
    assert_eq!(obs["title"], "Auth decision");
}

#[tokio::test]
async fn should_return_404_for_missing_observation() {
    let app = test_app();
    let resp = app
        .oneshot(
            Request::get("/observations/99999")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn should_soft_delete_observation() {
    let app = test_app();

    let resp = app
        .clone()
        .oneshot(
            Request::post("/observations")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "project": "p",
                        "title": "to delete",
                        "content": "ephemeral",
                        "type": "discovery",
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let id = body_json(resp).await["id"].as_i64().unwrap();

    let resp = app
        .oneshot(
            Request::delete(format!("/observations/{id}").as_str())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["mode"], "soft");
}

#[tokio::test]
async fn should_hard_delete_observation() {
    let app = test_app();

    let resp = app
        .clone()
        .oneshot(
            Request::post("/observations")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "project": "p",
                        "title": "hard delete me",
                        "content": "gone",
                        "type": "discovery",
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let id = body_json(resp).await["id"].as_i64().unwrap();

    let resp = app
        .clone()
        .oneshot(
            Request::delete(format!("/observations/{id}?hard=true").as_str())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["mode"], "hard");

    let resp = app
        .oneshot(
            Request::get(format!("/observations/{id}").as_str())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn should_search_observations() {
    let app = test_app();

    app.clone()
        .oneshot(
            Request::post("/observations")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "project": "searchproj",
                        "title": "Database indexing",
                        "content": "Add index on user_id for faster queries",
                        "type": "pattern",
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let resp = app
        .oneshot(
            Request::get("/search?q=index&project=searchproj")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let results = body_json(resp).await;
    assert!(!results.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn should_create_session() {
    let app = test_app();

    let resp = app
        .oneshot(
            Request::post("/sessions")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "project": "myproj",
                        "directory": "/tmp/myproj",
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = body_json(resp).await;
    assert!(body["session_id"].as_i64().unwrap() > 0);
}

#[tokio::test]
async fn should_get_stats() {
    let app = test_app();
    let resp = app
        .oneshot(Request::get("/stats").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert!(body["total"].is_number());
}

#[tokio::test]
async fn should_save_and_retrieve_prompt() {
    let app = test_app();

    let resp = app
        .clone()
        .oneshot(
            Request::post("/prompts")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&serde_json::json!({
                        "content": "Fix the login bug",
                        "project": "myproj",
                    }))
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let body = body_json(resp).await;
    assert!(body["id"].as_i64().unwrap() > 0);

    let resp = app
        .oneshot(
            Request::get("/prompts/recent?project=myproj")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let prompts = body_json(resp).await;
    assert_eq!(prompts.as_array().unwrap().len(), 1);
    assert_eq!(prompts[0]["content"], "Fix the login bug");
}
