use super::jwt::{Claims, JwtMiddleware};
use actix_web::test::{self, TestRequest};
use actix_web::{web, App, HttpResponse};
use jsonwebtoken::{encode, EncodingKey, Header};
use serial_test::serial;
use std::time::{SystemTime, UNIX_EPOCH};

async fn test_handler() -> HttpResponse {
    HttpResponse::Ok().finish()
}

#[actix_web::test]
async fn test_valid_token() {
    let test_secret = "test_secret";

    let claims = Claims {
        exp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize
            + 3600,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(test_secret.as_bytes()),
    )
    .unwrap();

    let app = test::init_service(
        App::new()
            .wrap(JwtMiddleware::new(test_secret.to_string()))
            .route("/", web::get().to(test_handler)),
    )
    .await;

    let req = TestRequest::get()
        .uri("/")
        .insert_header(("Authorization", format!("Bearer {}", token)))
        .to_request();

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
}

#[actix_web::test]
async fn test_invalid_token() {
    let app = test::init_service(
        App::new()
            .wrap(JwtMiddleware::new("test_secret".to_string()))
            .route("/", web::get().to(test_handler)),
    )
    .await;

    let req = TestRequest::get()
        .uri("/")
        .insert_header(("Authorization", "Bearer invalid_token"))
        .to_request();

    let err = test::try_call_service(&app, req).await.unwrap_err();
    let resp = err.error_response();
    assert_eq!(resp.status().as_u16(), 401);
}

#[actix_web::test]
async fn test_missing_auth_header() {
    let app = test::init_service(
        App::new()
            .wrap(JwtMiddleware::new("test_secret".to_string()))
            .route("/", web::get().to(test_handler)),
    )
    .await;

    let req = TestRequest::get().uri("/").to_request();
    let err = test::try_call_service(&app, req).await.unwrap_err();
    let resp = err.error_response();
    assert_eq!(resp.status().as_u16(), 401);
}

#[test]
#[serial]
fn test_from_env_missing_secret() {
    // Ensure JWT_SECRET is not set
    std::env::remove_var("JWT_SECRET");

    // Should fail when JWT_SECRET is missing
    let result = JwtMiddleware::from_env();
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err(),
        "JWT_SECRET environment variable not set"
    );
}

#[test]
#[serial]
fn test_from_env_with_secret() {
    // Set JWT_SECRET
    std::env::set_var("JWT_SECRET", "test_value");

    // Should succeed when JWT_SECRET is set
    let result = JwtMiddleware::from_env();
    assert!(result.is_ok());

    // Clean up
    std::env::remove_var("JWT_SECRET");
}

#[actix_web::test]
async fn test_empty_bearer_token() {
    let app = test::init_service(
        App::new()
            .wrap(JwtMiddleware::new("test_secret".to_string()))
            .route("/", web::get().to(test_handler)),
    )
    .await;

    let req = TestRequest::get()
        .uri("/")
        .insert_header(("Authorization", "Bearer "))
        .to_request();

    let err = test::try_call_service(&app, req).await.unwrap_err();
    let resp = err.error_response();
    assert_eq!(resp.status().as_u16(), 401);
}
