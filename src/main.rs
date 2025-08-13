use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{delete, get, post, put},
    Router,
};
use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{
    trace::{self},
    Resource,
};
use serde::{Deserialize, Serialize};
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;
use tracing::{info, instrument};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

mod db;
mod handlers;
mod models;
mod schema;

use db::DbPool;
use handlers::UserRepository;
use models::{NewUser, UpdateUser, User};

#[derive(Serialize)]
struct HealthResponse {
    status: String,
    timestamp: u64,
}

#[derive(Deserialize, Debug)]
struct QueryParams {
    name: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct GreetingRequest {
    name: String,
}

#[derive(Serialize)]
struct GreetingResponse {
    message: String,
}

/// Initialize OpenTelemetry tracing
fn init_tracing() -> Result<(), Box<dyn std::error::Error>> {
    // First, set up basic tracing subscriber
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        "wanderer_connector=debug,tower_http=debug,axum::rejection=trace".into()
    });

    // Try to set up OpenTelemetry OTLP exporter
    match opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint("http://localhost:4317"), // Default OTEL collector endpoint
        )
        .with_trace_config(trace::config().with_resource(Resource::new(vec![
            KeyValue::new("service.name", "wanderer-connector"),
            KeyValue::new("service.version", "0.1.0"),
        ])))
        .install_batch(opentelemetry_sdk::runtime::Tokio)
    {
        Ok(tracer) => {
            println!("✅ OpenTelemetry initialized successfully, sending traces to http://localhost:4317");
            // Set up tracing subscriber with OpenTelemetry layer
            tracing_subscriber::registry()
                .with(env_filter)
                .with(tracing_subscriber::fmt::layer())
                .with(tracing_opentelemetry::layer().with_tracer(tracer))
                .init();
        }
        Err(e) => {
            println!("⚠️  Failed to initialize OpenTelemetry: {}", e);
            println!("📝 Falling back to console-only logging");
            // Fall back to console-only logging
            tracing_subscriber::registry()
                .with(env_filter)
                .with(tracing_subscriber::fmt::layer())
                .init();
        }
    }

    Ok(())
}

/// Health check endpoint
#[instrument]
async fn health() -> Json<HealthResponse> {
    info!("Health check requested");
    Json(HealthResponse {
        status: "healthy".to_string(),
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    })
}

/// Simple greeting endpoint with query parameters
#[instrument]
async fn hello(Query(params): Query<QueryParams>) -> Json<GreetingResponse> {
    let name = params.name.unwrap_or_else(|| "World".to_string());
    info!("Greeting requested for: {}", name);

    Json(GreetingResponse {
        message: format!("Hello, {}!", name),
    })
}

/// Greeting endpoint with JSON body
#[instrument]
async fn greet_json(Json(payload): Json<GreetingRequest>) -> Json<GreetingResponse> {
    info!("JSON greeting requested for: {}", payload.name);

    Json(GreetingResponse {
        message: format!("Hello, {}! (from JSON)", payload.name),
    })
}

// Database endpoints

/// Create a new user
#[instrument(skip(pool))]
async fn create_user(
    State(pool): State<DbPool>,
    Json(new_user): Json<NewUser>,
) -> Result<Json<User>, StatusCode> {
    match UserRepository::create_user(&pool, new_user).await {
        Ok(user) => {
            info!("Created user with ID: {}", user.id);
            Ok(Json(user))
        }
        Err(e) => {
            tracing::error!("Failed to create user: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get all users
#[instrument(skip(pool))]
async fn get_users(State(pool): State<DbPool>) -> Result<Json<Vec<User>>, StatusCode> {
    match UserRepository::get_all_users(&pool).await {
        Ok(users) => {
            info!("Retrieved {} users", users.len());
            Ok(Json(users))
        }
        Err(e) => {
            tracing::error!("Failed to retrieve users: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Get a user by ID
#[instrument(skip(pool))]
async fn get_user(
    State(pool): State<DbPool>,
    Path(user_id): Path<Uuid>,
) -> Result<Json<User>, StatusCode> {
    match UserRepository::get_user_by_id(&pool, user_id).await {
        Ok(user) => {
            info!("Retrieved user with ID: {}", user.id);
            Ok(Json(user))
        }
        Err(e) => {
            tracing::error!("Failed to retrieve user {}: {}", user_id, e);
            Err(StatusCode::NOT_FOUND)
        }
    }
}

/// Update a user
#[instrument(skip(pool))]
async fn update_user(
    State(pool): State<DbPool>,
    Path(user_id): Path<Uuid>,
    Json(update_user): Json<UpdateUser>,
) -> Result<Json<User>, StatusCode> {
    match UserRepository::update_user(&pool, user_id, update_user).await {
        Ok(user) => {
            info!("Updated user with ID: {}", user.id);
            Ok(Json(user))
        }
        Err(e) => {
            tracing::error!("Failed to update user {}: {}", user_id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Delete a user
#[instrument(skip(pool))]
async fn delete_user(
    State(pool): State<DbPool>,
    Path(user_id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    match UserRepository::delete_user(&pool, user_id).await {
        Ok(true) => {
            info!("Deleted user with ID: {}", user_id);
            Ok(StatusCode::NO_CONTENT)
        }
        Ok(false) => {
            tracing::warn!("User {} not found for deletion", user_id);
            Err(StatusCode::NOT_FOUND)
        }
        Err(e) => {
            tracing::error!("Failed to delete user {}: {}", user_id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Create the Axum router with all routes
fn create_router(pool: DbPool) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/hello", get(hello))
        .route("/greet", post(greet_json))
        // User endpoints
        .route("/users", get(get_users))
        .route("/users", post(create_user))
        .route("/users/:id", get(get_user))
        .route("/users/:id", put(update_user))
        .route("/users/:id", delete(delete_user))
        .with_state(pool)
        .layer(ServiceBuilder::new().layer(TraceLayer::new_for_http()))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables
    dotenvy::dotenv().ok();

    // Initialize tracing
    init_tracing()?;

    info!("Starting wanderer-connector API server");

    // Set up database connection pool
    info!("Setting up database connection pool");
    let pool = db::establish_connection_pool()?;
    info!("Database connection pool established");

    // Create the router with database pool
    let app = create_router(pool);

    // Start the server
    let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("{}:{}", host, port);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!("Server listening on http://{}", addr);

    axum::serve(listener, app).await?;

    // Shutdown OpenTelemetry
    global::shutdown_tracer_provider();

    Ok(())
}
