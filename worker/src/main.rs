use shared::models::{Config, Error, User, Ytdlp};

use shared::utils::worker::do_the_work;

use tracing::{error, info};

use std::{env::var, path::Path};

use sqlx::{
    migrate::{MigrateDatabase, Migrator},
    sqlite::SqlitePoolOptions,
    SqlitePool,
};

static DDBB: &str = "u2vpodcast.db";
static MIGRATIONS_DIR: &str = "migrations";

#[tokio::main]
async fn main() {
    let config = Config::load().await;
    let pool = init_db(config.clone()).await.expect("init db");
    //let auth = HttpAuthentication::bearer(validator);
    info!("**** Start updating yt-dlp ****");
    match Ytdlp::auto_update().await {
        Ok(()) => {}
        Err(e) => error!("{}", e),
    }
    info!("**** Finish updating yt-dlp ****");
    match do_the_work(&pool).await {
        Ok(_) => {}
        Err(e) => {
            error!("Error doing the work: {e}");
        }
    }
}

async fn init_db(config: Config) -> Result<SqlitePool, Error> {
    let db_url = if var("RUST_ENV") == Ok("production".to_string()) {
        std::env::current_exe()?
            .parent()
            .unwrap()
            .join("db")
            .join(DDBB)
            .to_str()
            .unwrap()
            .to_string()
    } else {
        let crate_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        Path::new(&crate_dir)
            .join(DDBB)
            .to_str()
            .unwrap()
            .to_string()
    };
    info!("DB url: {db_url}");
    let db_exists = sqlx::Sqlite::database_exists(&db_url).await.unwrap();
    info!("DB exists: {db_exists}");
    if !db_exists {
        sqlx::Sqlite::create_database(&db_url).await.unwrap();
    }

    let migrations = if var("RUST_ENV") == Ok("production".to_string()) {
        std::env::current_exe()
            .unwrap()
            .parent()
            .unwrap()
            .join(MIGRATIONS_DIR)
    } else {
        let crate_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
        Path::new(&crate_dir).join(MIGRATIONS_DIR)
    };
    info!("{}", &migrations.display());

    let pool = SqlitePoolOptions::new()
        .max_connections(2)
        .connect(&db_url)
        .await
        .expect("Pool failed");

    Migrator::new(migrations).await?.run(&pool).await?;

    if !db_exists {
        User::default(&pool, &config.admin_username, &config.admin_password)
            .await
            .expect("Cant create admin user");
    }

    Ok(pool)
}
