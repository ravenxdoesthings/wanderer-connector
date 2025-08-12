use axum::{
    extract::Query,
    response::Json,
    routing::{get, post},
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

/// Create the Axum router with all routes
fn create_router() -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/hello", get(hello))
        .route("/greet", post(greet_json))
        .layer(ServiceBuilder::new().layer(TraceLayer::new_for_http()))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    init_tracing()?;

    info!("Starting wanderer-connector API server");

    // Create the router
    let app = create_router();

    // Start the server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    info!("Server listening on http://0.0.0.0:3000");

    axum::serve(listener, app).await?;

    // Shutdown OpenTelemetry
    global::shutdown_tracer_provider();

    Ok(())
}
