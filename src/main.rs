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

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3050").await.unwrap();
    println!("Listening on port 3050");
    axum::serve(listener, app).await.unwrap();
}

#[axum::debug_handler]
async fn handle_webhook(
    State(state): State<AppState>,
    Json(raw_payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    println!("Received webhook payload:\n{}", serde_json::to_string_pretty(&raw_payload).unwrap_or_default());

    // Try to find the fields/questions array, even if it's nested under "data"
    let fields_array = raw_payload.get("data").and_then(|d| d.get("fields").or(d.get("questions")))
        .or_else(|| raw_payload.get("fields").or(raw_payload.get("questions")))
        .and_then(|v| v.as_array());

    let Some(fields) = fields_array else {
        eprintln!("Could not find 'fields' or 'questions' array in payload");
        return (StatusCode::OK, "Payload ignored: no fields array").into_response(); // Return 200 so Fillout doesn't retry infinitely on weird events
    };

    let mut name = String::new();
    let mut email = String::new();
    let mut attributes = HashMap::new();

    for field in fields {
        let field_name = field.get("name").and_then(|n| n.as_str()).unwrap_or("");
        let field_value = field.get("value").unwrap_or(&serde_json::Value::Null);

        let field_val_str = match field_value {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::Bool(b) => b.to_string(),
            _ => continue, // Ignore complex objects for now
        };

        if field_name.eq_ignore_ascii_case("name") {
            name = field_val_str.clone();
        } else if field_name.eq_ignore_ascii_case("email") {
            email = field_val_str.clone();
        }

        // Brevo requires attribute names to be UPPERCASE
        if !field_name.eq_ignore_ascii_case("email") && !field_name.is_empty() {
            attributes.insert(field_name.to_uppercase(), field_val_str);
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
