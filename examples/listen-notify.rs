use dotenvy::dotenv;
use futures::{stream, StreamExt};
use futures::{FutureExt, TryStreamExt};
use std::env;
use tokio_postgres::NoTls;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    // Connect using tokio-postgres
    println!("Connecting to PostgreSQL...");
    let (client, mut connection) = tokio_postgres::connect(&database_url, NoTls).await?;

    let (tx, rx) = futures_channel::mpsc::unbounded();
    let stream =
        stream::poll_fn(move |cx| connection.poll_message(cx)).map_err(|e| panic!("{}", e));
    let connection = stream.forward(tx).map(|r| r.unwrap());
    tokio::spawn(connection);

    tokio::spawn(async move {
        let notifications = rx
            .filter_map(|m| match m {
                tokio_postgres::AsyncMessage::Notification(n) => {
                    println!("Notification {:?}", n);
                    futures_util::future::ready(Some(n))
                }
                _ => futures_util::future::ready(None),
            })
            .collect::<Vec<_>>()
            .await;

        // All notifications?
        println!("All notifications {:?}", notifications);
    });

    // Set up triggers using tokio-postgres
    println!("Setting up database triggers...");

    client
        .execute(
            r#"
        CREATE OR REPLACE FUNCTION new_system_notify() RETURNS TRIGGER AS $$
        DECLARE
        BEGIN
            PERFORM pg_notify('system_insert', row_to_json(NEW)::text);
            RETURN NEW;
        END;
        $$ LANGUAGE plpgsql;
    "#,
            &[],
        )
        .await?;

    client.execute("CREATE OR REPLACE TRIGGER system_insert AFTER INSERT ON test_table FOR EACH ROW EXECUTE FUNCTION new_system_notify()", &[]).await?;

    println!("Database triggers set up successfully!");

    // Start listening for notifications
    client.execute("LISTEN system_insert", &[]).await?;

    loop {}
}
