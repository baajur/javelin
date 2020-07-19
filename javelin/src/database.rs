#[cfg(all(
    feature = "db-sqlite",
    feature = "db-mongo")
)]
compile_error!("Cannot enable multiple database backends simultaneously");


#[cfg(not(any(
    feature = "db-sqlite",
    feature = "db-mongo")
))]
compile_error!("One database backend is required");


#[cfg_attr(feature = "db-sqlite", path = "database/sqlite.rs")]
#[cfg_attr(feature = "db-mongo", path = "database/mongo.rs")]
mod backend;

pub use self::backend::Database;


#[cfg(test)]
mod tests {
    use {
        std::fs::remove_file,
        javelin_core::Config,
        javelin_types::models::{UserRepository, User},
        super::*,
    };

    async fn database() -> Database {
        let mut config = Config::new();

        let path = "../javelin_test.db";
        let _ = remove_file(&path);
        config.set("database.sqlite.path", path).unwrap();

        config.set("database.mongo.password", "s3cur3").unwrap();
        config.set("database.mongo.username", "dev").unwrap();
        config.set("database.mongo.dbname", "javelin_test").unwrap();

        Database::new(&config).await
    }

    #[tokio::test]
    async fn create_user() {
        let mut db = database().await;

        db.add_user_with_key("tester", "12345").await
            .expect("Failed to create new user");

        let user = db.user_by_name("tester").await
            .expect("Failed to find user");

        assert_eq!(
            Some(User { name: "tester".to_string(), key: "12345".to_string() }),
            user
        );
    }

    #[tokio::test]
    async fn update_user() {
        let mut db = database().await;

        db.add_user_with_key("tester", "12345").await
            .expect("Failed to create new user");

        db.add_user_with_key("tester", "123456").await
            .expect("Failed to update user");

        let user = db.user_by_name("tester").await
            .expect("Failed to find user");

        assert_eq!(
            Some(User { name: "tester".to_string(), key: "123456".to_string() }),
            user
        );
    }
}
