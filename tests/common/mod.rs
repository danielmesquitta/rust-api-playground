//! Shared harness: each test gets its own Postgres container and server instance.

use reqwest::{Client, Method, Response};
use serde_json::{Value, json};
use testcontainers_modules::postgres::Postgres;
use testcontainers_modules::testcontainers::runners::AsyncRunner as _;
use testcontainers_modules::testcontainers::{ContainerAsync, ImageExt as _};
use tokio::net::TcpListener;

/// Same major version as `docker-compose.yml`.
const POSTGRES_TAG: &str = "18-alpine";

/// A running server backed by a fresh database. Dropping it stops the container.
pub(crate) struct TestApp {
    client: Client,
    base_url: String,
    _db: ContainerAsync<Postgres>,
}

impl TestApp {
    /// Starts Postgres, applies migrations, and serves the app on an ephemeral port.
    pub(crate) async fn spawn() -> Self {
        let db = Postgres::default()
            .with_tag(POSTGRES_TAG)
            .start()
            .await
            .expect("Postgres container should start (is Docker running?)");
        let host = db.get_host().await.expect("container host");
        let port = db.get_host_port_ipv4(5432).await.expect("mapped port");
        let database_url = format!("postgres://postgres:postgres@{host}:{port}/postgres");

        let pool = rust_api_playground::connect(&database_url)
            .await
            .expect("connect and migrate");
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("local addr");
        tokio::spawn(async move {
            axum::serve(listener, rust_api_playground::app(pool))
                .await
                .expect("server should run");
        });

        Self {
            client: Client::new(),
            base_url: format!("http://{addr}"),
            _db: db,
        }
    }

    pub(crate) fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base_url)
    }

    /// Sends a request with an optional JSON body.
    pub(crate) async fn request(
        &self,
        method: Method,
        path: &str,
        body: Option<Value>,
    ) -> Response {
        let mut request = self.client.request(method, self.url(path));
        if let Some(body) = body {
            request = request.json(&body);
        }
        request.send().await.expect("request should be sent")
    }

    /// Sends a raw body with the given content type.
    pub(crate) async fn request_raw(
        &self,
        method: Method,
        path: &str,
        content_type: &str,
        body: &str,
    ) -> Response {
        self.client
            .request(method, self.url(path))
            .header(reqwest::header::CONTENT_TYPE, content_type)
            .body(body.to_owned())
            .send()
            .await
            .expect("request should be sent")
    }

    /// Creates a todo and returns its JSON, asserting the request succeeded.
    pub(crate) async fn create_todo(&self, title: &str, description: Option<&str>) -> Value {
        let response = self
            .request(
                Method::POST,
                "/todos",
                Some(json!({ "title": title, "description": description })),
            )
            .await;
        assert_eq!(response.status(), reqwest::StatusCode::CREATED);
        response.json().await.expect("todo JSON")
    }
}

/// Asserts the response has `status` and a JSON `{"error": "..."}` body, and returns the
/// message.
pub(crate) async fn assert_error(response: Response, status: reqwest::StatusCode) -> String {
    assert_eq!(response.status(), status);
    let body: Value = response.json().await.expect("error body should be JSON");
    body["error"]
        .as_str()
        .unwrap_or_else(|| panic!("error body should have an `error` string: {body}"))
        .to_owned()
}
