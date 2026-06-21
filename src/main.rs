use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use rand::RngExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use reqwest::Client;

#[derive(Clone)]
struct AppState {
    brevo_api_key: String,
    http_client: Client,
}

#[derive(Debug, Deserialize)]
struct FilloutPayload {
    fields: Vec<FilloutField>,
}

#[derive(Debug, Deserialize)]
struct FilloutField {
    name: String,
    value: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct BrevoRequest {
    email: String,
    attributes: HashMap<String, String>,
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    
    let brevo_api_key = std::env::var("BREVO_API_KEY").unwrap_or_else(|_| {
        println!("Warning: BREVO_API_KEY not set in environment");
        "".to_string()
    });
    
    let state = AppState {
        brevo_api_key,
        http_client: Client::new(),
    };

    let app = Router::new()
        .route("/webhook", post(handle_webhook))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Listening on port 3000");
    axum::serve(listener, app).await.unwrap();
}

#[axum::debug_handler]
async fn handle_webhook(
    State(state): State<AppState>,
    Json(payload): Json<FilloutPayload>,
) -> impl IntoResponse {
    let mut name = String::new();
    let mut email = String::new();
    let mut attributes = HashMap::new();

    for field in payload.fields {
        let field_val = match field.value {
            serde_json::Value::String(s) => s,
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::Bool(b) => b.to_string(),
            _ => continue, // Ignore complex objects for now
        };

        if field.name.eq_ignore_ascii_case("name") {
            name = field_val.clone();
        } else if field.name.eq_ignore_ascii_case("email") {
            email = field_val.clone();
        }

        // Brevo requires attribute names to be UPPERCASE
        if !field.name.eq_ignore_ascii_case("email") {
            attributes.insert(field.name.to_uppercase(), field_val);
        }
    }

    if email.is_empty() {
        return (StatusCode::BAD_REQUEST, "Email is required").into_response();
    }

    // Generate referral code: first 3 letters of Name + 4 random numbers
    let name_prefix = name.chars().take(3).collect::<String>().to_uppercase();
    
    // Support older and newer rand versions
    let random_numbers: u32 = {
        let mut rng = rand::rng();
        rng.random_range(1000..=9999)
    };
    
    let referral_code = format!("{}{}", name_prefix, random_numbers);

    println!("Generated Referral Code for {} ({}): {}", name, email, referral_code);

    attributes.insert("REFERRAL_CODE".to_string(), referral_code);

    let brevo_req = BrevoRequest {
        email,
        attributes,
    };

    let res = state.http_client.post("https://api.brevo.com/v3/contacts")
        .header("api-key", &state.brevo_api_key)
        .header("Content-Type", "application/json")
        .json(&brevo_req)
        .send()
        .await;

    match res {
        Ok(response) => {
            if response.status().is_success() {
                (StatusCode::OK, "Saved to Brevo").into_response()
            } else {
                let err_text = response.text().await.unwrap_or_default();
                eprintln!("Brevo error: {}", err_text);
                (StatusCode::INTERNAL_SERVER_ERROR, "Failed to save to Brevo").into_response()
            }
        }
        Err(e) => {
            eprintln!("Request error: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, "Request error").into_response()
        }
    }
}
