use std::sync::Arc;
use sqlx::postgres::PgPoolOptions;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting SutraERP Server...");

    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/sutra".to_string());
    
    // Connect lazily so it doesn't crash on startup if DB is missing
    let db = PgPoolOptions::new()
        .max_connections(5)
        .connect_lazy(&db_url)?;
        
    let state = Arc::new(sutra_api::state::AppState::new(db, None));
    let app = sutra_api::router::create_router(state);
    
    let listener = TcpListener::bind("0.0.0.0:3000").await?;
    println!("Listening on http://0.0.0.0:3000");
    axum::serve(listener, app).await?;
    
    Ok(())
}
