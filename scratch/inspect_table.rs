use sea_orm::{Database, ConnectionTrait, Statement};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/dispatcharr".to_string());
    let db = Database::connect(&database_url).await?;
    
    let query = "SELECT column_name, data_type FROM information_schema.columns WHERE table_name = 'core_systemevent'";
    let results = db.query_all(Statement::from_string(sea_orm::DatabaseBackend::Postgres, query)).await?;
    
    for row in results {
        let col: String = row.try_get("", "column_name")?;
        let dtype: String = row.try_get("", "data_type")?;
        println!("{}: {}", col, dtype);
    }
    
    Ok(())
}
