use axum::{
    extract::Json,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Extension, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Item {
    pub name: String,
    pub quantity: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct User {
    pub username: String,
    pub password: String,
    pub items: Vec<Item>,
    pub money: u32,
}

type Db = Arc<Mutex<HashMap<String, User>>>;

// Função vulnerável do desafio
pub fn validate_body(body: &Value, allowed: &[&str]) -> bool {
    if let Some(body) = body.as_object() {
        if body.keys().any(|key| !allowed.contains(&key.as_str())) {
            return false;
        }
    }
    true
}

// O Extension(db) DEVE vir ANTES do Json(body)
async fn register_post(
    Extension(db): Extension<Db>,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    if !validate_body(&body, &["username", "password"]) {
        return (StatusCode::BAD_REQUEST, "Campos invalidos\n").into_response();
    }

    let user: User = match serde_json::from_value(body) {
        Ok(u) => u,
        Err(_) => return (StatusCode::BAD_REQUEST, "Erro no de-serialization\n").into_response(),
    };

    let mut store = db.lock().unwrap();
    let username = user.username.clone();
    store.insert(username.clone(), user);

    let mut headers = HeaderMap::new();
    headers.insert(
        "Set-Cookie",
        format!("session={}", username).parse().unwrap(),
    );

    (StatusCode::OK, headers, "Usuario registrado com sucesso!\n").into_response()
}

async fn flag_get(
    headers: HeaderMap,
    Extension(db): Extension<Db>,
) -> impl IntoResponse {
    let cookie_header = match headers.get("Cookie") {
        Some(h) => h.to_str().unwrap_or(""),
        None => return (StatusCode::UNAUTHORIZED, "Nao autenticado\n").into_response(),
    };

    let username = match cookie_header.split('=').nth(1) {
        Some(u) => u,
        None => return (StatusCode::UNAUTHORIZED, "Sessao invalida\n").into_response(),
    };

    let store = db.lock().unwrap();
    if let Some(user) = store.get(username) {
        let has_flag_item = user
            .items
            .iter()
            .any(|item| item.name == "rustshop flag" && item.quantity == 0x42069);

        if user.money == 0x13371337 && has_flag_item {
            return (
                StatusCode::OK,
                "{\"status\":\"success\",\"message\":\"corctf{we_d0_s0me_s3rde_shen4nigans}\"}\n",
            )
                .into_response();
        } else {
            return (
                StatusCode::FORBIDDEN,
                "Requisitos nao atingidos: Saldo ou quantidade de itens incorretos.\n",
            )
                .into_response();
        }
    }

    (StatusCode::UNAUTHORIZED, "Usuario nao encontrado\n").into_response()
}

#[tokio::main]
async fn main() {
    let db: Db = Arc::new(Mutex::new(HashMap::new()));

    let app = Router::new()
        .route("/api/register", post(register_post))
        .route("/api/flag", get(flag_get))
        .layer(Extension(db));

    let addr = SocketAddr::from(([127, 0, 0, 1], 1337));
    println!("🦀 Servidor rustshop rodando em http://{}", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
