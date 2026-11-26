//! End-to-end tests against a real server and a real Postgres in Docker.
//!
//! Each test starts its own container, so tests are isolated and run in parallel.

mod common;

use common::{TestApp, assert_error};
use reqwest::{Method, StatusCode};
use rstest::rstest;
use serde_json::{Value, json};

#[tokio::test]
async fn todo_lifecycle() {
    let app = TestApp::spawn().await;

    // Create.
    let response = app
        .request(
            Method::POST,
            "/todos",
            Some(json!({ "title": "  Buy milk ", "description": "2L" })),
        )
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let location = response.headers()["location"].to_str().unwrap().to_owned();
    let created: Value = response.json().await.unwrap();
    let id = created["id"].as_i64().expect("id should be an integer");
    assert_eq!(location, format!("/todos/{id}"));
    assert_eq!(created["title"], "Buy milk", "title should be trimmed");
    assert_eq!(created["description"], "2L");
    assert_eq!(created["completed"], false);
    assert_eq!(created["created_at"], created["updated_at"]);

    // Read it back through the Location header.
    let response = app.request(Method::GET, &location, None).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.json::<Value>().await.unwrap(), created);

    // It shows up in the list.
    let list: Value = app
        .request(Method::GET, "/todos", None)
        .await
        .json()
        .await
        .unwrap();
    assert_eq!(list, json!([created]));

    // Complete it.
    let response = app
        .request(Method::PATCH, &location, Some(json!({ "completed": true })))
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let updated: Value = response.json().await.unwrap();
    assert_eq!(updated["completed"], true);
    assert_eq!(updated["title"], "Buy milk");
    assert_eq!(updated["description"], "2L");
    assert_eq!(updated["created_at"], created["created_at"]);
    assert_ne!(updated["updated_at"], created["updated_at"]);

    // The completed filter finds it.
    let done: Value = app
        .request(Method::GET, "/todos?completed=true", None)
        .await
        .json()
        .await
        .unwrap();
    assert_eq!(done, json!([updated]));

    // Delete it, and it's gone.
    let response = app.request(Method::DELETE, &location, None).await;
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    let response = app.request(Method::GET, &location, None).await;
    assert_error(response, StatusCode::NOT_FOUND).await;
}

#[rstest]
#[case::empty_title(
    "application/json",
    r#"{"title":""}"#,
    StatusCode::UNPROCESSABLE_ENTITY
)]
#[case::blank_title(
    "application/json",
    r#"{"title":"   "}"#,
    StatusCode::UNPROCESSABLE_ENTITY
)]
#[case::title_too_long(
    "application/json",
    &format!(r#"{{"title":"{}"}}"#, "a".repeat(201)),
    StatusCode::UNPROCESSABLE_ENTITY,
)]
#[case::missing_title(
    "application/json",
    r#"{"description":"x"}"#,
    StatusCode::UNPROCESSABLE_ENTITY
)]
#[case::unknown_field(
    "application/json",
    r#"{"title":"x","done":true}"#,
    StatusCode::UNPROCESSABLE_ENTITY
)]
#[case::malformed_json("application/json", r#"{"title":"#, StatusCode::BAD_REQUEST)]
#[case::not_json("text/plain", "Buy milk", StatusCode::UNSUPPORTED_MEDIA_TYPE)]
#[tokio::test]
async fn create_rejects_invalid_body(
    #[case] content_type: &str,
    #[case] body: &str,
    #[case] expected: StatusCode,
) {
    let app = TestApp::spawn().await;

    let response = app
        .request_raw(Method::POST, "/todos", content_type, body)
        .await;

    assert_error(response, expected).await;
    let list: Value = app
        .request(Method::GET, "/todos", None)
        .await
        .json()
        .await
        .unwrap();
    assert_eq!(list, json!([]), "nothing should have been stored");
}

#[rstest]
#[case::get_missing(Method::GET, "/todos/999", None, StatusCode::NOT_FOUND)]
#[case::patch_missing(Method::PATCH, "/todos/999", Some(json!({ "completed": true })), StatusCode::NOT_FOUND)]
#[case::delete_missing(Method::DELETE, "/todos/999", None, StatusCode::NOT_FOUND)]
#[case::get_non_integer_id(Method::GET, "/todos/abc", None, StatusCode::BAD_REQUEST)]
#[case::delete_non_integer_id(Method::DELETE, "/todos/abc", None, StatusCode::BAD_REQUEST)]
#[tokio::test]
async fn unknown_or_invalid_id(
    #[case] method: Method,
    #[case] path: &str,
    #[case] body: Option<Value>,
    #[case] expected: StatusCode,
) {
    let app = TestApp::spawn().await;

    let response = app.request(method, path, body).await;

    assert_error(response, expected).await;
}

#[rstest]
#[case::empty_patch_changes_nothing(json!({}), "Buy milk", Some("2L"), false)]
#[case::null_clears_description(json!({ "description": null }), "Buy milk", None, false)]
#[case::sets_description(json!({ "description": "1L" }), "Buy milk", Some("1L"), false)]
#[case::sets_title_keeps_description(json!({ "title": " Buy oat milk " }), "Buy oat milk", Some("2L"), false)]
#[case::sets_completed(json!({ "completed": true }), "Buy milk", Some("2L"), true)]
#[case::sets_everything(
    json!({ "title": "Buy bread", "description": "Sourdough", "completed": true }),
    "Buy bread",
    Some("Sourdough"),
    true,
)]
#[tokio::test]
async fn patch_updates_only_given_fields(
    #[case] patch: Value,
    #[case] title: &str,
    #[case] description: Option<&str>,
    #[case] completed: bool,
) {
    let app = TestApp::spawn().await;
    let created = app.create_todo("Buy milk", Some("2L")).await;
    let path = format!("/todos/{}", created["id"]);

    let response = app.request(Method::PATCH, &path, Some(patch)).await;

    assert_eq!(response.status(), StatusCode::OK);
    let expected = json!({ "title": title, "description": description, "completed": completed });
    let updated: Value = response.json().await.unwrap();
    for field in ["title", "description", "completed"] {
        assert_eq!(updated[field], expected[field], "field {field}");
    }
    // The change is persisted, not just echoed.
    let stored: Value = app
        .request(Method::GET, &path, None)
        .await
        .json()
        .await
        .unwrap();
    assert_eq!(stored, updated);
}

#[rstest]
#[case::empty_title(json!({ "title": "" }))]
#[case::title_too_long(json!({ "title": "a".repeat(201) }))]
#[case::unknown_field(json!({ "done": true }))]
#[case::completed_wrong_type(json!({ "completed": "yes" }))]
#[tokio::test]
async fn patch_rejects_invalid_body(#[case] patch: Value) {
    let app = TestApp::spawn().await;
    let created = app.create_todo("Buy milk", None).await;
    let path = format!("/todos/{}", created["id"]);

    let response = app.request(Method::PATCH, &path, Some(patch)).await;

    assert_error(response, StatusCode::UNPROCESSABLE_ENTITY).await;
    let stored: Value = app
        .request(Method::GET, &path, None)
        .await
        .json()
        .await
        .unwrap();
    assert_eq!(stored, created, "a rejected patch must not change the todo");
}

#[rstest]
#[case::all("/todos", &["Walk dog", "Pay rent", "Buy milk"])]
#[case::completed("/todos?completed=true", &["Pay rent"])]
#[case::not_completed("/todos?completed=false", &["Walk dog", "Buy milk"])]
#[tokio::test]
async fn list_filters_and_orders_newest_first(
    #[case] path: &str,
    #[case] expected_titles: &[&str],
) {
    let app = TestApp::spawn().await;
    app.create_todo("Buy milk", None).await;
    let rent = app.create_todo("Pay rent", None).await;
    app.create_todo("Walk dog", None).await;
    let response = app
        .request(
            Method::PATCH,
            &format!("/todos/{}", rent["id"]),
            Some(json!({ "completed": true })),
        )
        .await;
    assert_eq!(response.status(), StatusCode::OK);

    let response = app.request(Method::GET, path, None).await;

    assert_eq!(response.status(), StatusCode::OK);
    let todos: Vec<Value> = response.json().await.unwrap();
    let titles: Vec<&str> = todos.iter().map(|t| t["title"].as_str().unwrap()).collect();
    assert_eq!(titles, expected_titles);
}

#[tokio::test]
async fn list_rejects_invalid_filter() {
    let app = TestApp::spawn().await;

    let response = app
        .request(Method::GET, "/todos?completed=maybe", None)
        .await;

    assert_error(response, StatusCode::BAD_REQUEST).await;
}

#[tokio::test]
async fn health_reports_ok() {
    let app = TestApp::spawn().await;

    let response = app.request(Method::GET, "/health", None).await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.json::<Value>().await.unwrap(),
        json!({ "status": "ok" })
    );
}

#[tokio::test]
async fn docs_are_served() {
    let app = TestApp::spawn().await;

    let response = app
        .request(Method::GET, "/api-docs/openapi.json", None)
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    let spec: Value = response.json().await.unwrap();
    assert_eq!(spec["info"]["title"], "Todo API");
    for path in ["/health", "/todos", "/todos/{id}"] {
        assert!(
            spec["paths"][path].is_object(),
            "spec should document {path}"
        );
    }

    let response = app.request(Method::GET, "/swagger-ui/", None).await;
    assert_eq!(response.status(), StatusCode::OK);
    let html = response.text().await.unwrap();
    assert!(
        html.contains("swagger-ui"),
        "should serve the Swagger UI page"
    );
}
