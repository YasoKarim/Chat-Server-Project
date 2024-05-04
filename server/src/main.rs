//Thanks to https://book.async.rs/tutorial/
//https://www.youtube.com/watch?v=TCERYbgvbq0
//https://doc.rust-lang.org/book/
//server/src/main.rs

// Import necessary libraries
extern crate async_std;
extern crate futures;

// Use statements to simplify code
use async_std::{
    io::BufReader,
    net::{TcpListener, TcpStream, ToSocketAddrs},
    prelude::*,
    task,
};
use futures::channel::mpsc;
use futures::sink::SinkExt;
use futures::{select, FutureExt};
use std::{
    collections::hash_map::{Entry, HashMap},
    sync::Arc,
};

// Define Result, Sender, and Receiver types
type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
type Sender<T> = mpsc::UnboundedSender<T>;
type Receiver<T> = mpsc::UnboundedReceiver<T>;

// Define an enum for representing a void type
#[derive(Debug)]
enum Void {}

// Main function
fn main() {
    if let Err(err) = run() {
        eprintln!("Error: {:?}", err);
    }
}

// Run function, entry point of the program
fn run() -> Result<()> {
    let fut = accept_loop("127.0.0.1:8080");
    task::block_on(fut)
}

// Function to handle accepting connections
async fn accept_loop(addr: impl ToSocketAddrs) -> Result<()> {
    // Bind a TcpListener to the specified address
    let listener = TcpListener::bind(addr).await?;
    // Create a channel for communication between the broker and other components
    let (broker_sender, broker_receiver) = mpsc::unbounded();
    // Spawn the broker loop to handle events in the background
    let broker_handle = task::spawn(broker_loop(broker_receiver));
    // Accept incoming connections
    let mut incoming = listener.incoming();

    while let Some(stream) = incoming.next().await {
        let stream = stream?;
        println!("Accepting from: {}", stream.peer_addr()?);
        // Spawn a connection loop for each accepted connection
        spawn_and_log_error(connection_loop(broker_sender.clone(), stream));
    }

    // Drop the broker sender to signal the broker to shut down
    drop(broker_sender);
    // Wait for the broker to finish processing events
    broker_handle.await;
    Ok(())
}

// Function to handle a connection with a client
async fn connection_loop(mut broker: Sender<Event>, stream: TcpStream) -> Result<()> {
    let stream = Arc::new(stream);
    let reader = BufReader::new(&*stream);
    let mut lines = reader.lines();

    // Read the name of the client from the first line
    let name = match lines.next().await {
        None => Err("peer disconnected immediately")?,
        Some(line) => line?,
    };

    println!("Client connected with name: {}", name);

    // Create a channel for communication between the connection loop and the broker
    let (_shutdown_sender, shutdown_receiver) = mpsc::unbounded::<Void>();
    // Send a NewPeer event to the broker
    broker
        .send(Event::NewPeer {
            name: name.clone(),
            stream: Arc::clone(&stream),
            shutdown: shutdown_receiver,
        })
        .await
        .unwrap();

    // Process messages from the client
    while let Some(line) = lines.next().await {
        let line = line?;
        println!("Received message from {}: {}", name, line);

        let mut parts = line.split('\n');

        // Split the message into destination and content
        let (dest, msg) = match line.find(" ") {
            None => {
                println!("No ' ' found in the message from {}: {}", name, line);
                continue;
            }
            Some(idx) => (&line[..idx], line[idx + 1..].trim()),
        };

        // Split the destination into a vector of strings
        let dest: Vec<String> = dest.split(',').map(|name| name.trim().to_string()).collect();
        let msg: String = msg.trim().to_string();

        println!("Name: {}", name);
        println!("Destination: {:?}", dest);
        println!("Message: {}", msg);

        // Send a Message event to the broker
        if let Err(send_error) = broker
            .send(Event::Message {
                from: name.clone(),
                to: dest.clone(),
                msg: format!("{}\n", msg),
            })
            .await
        {
            // Handle the error, e.g., print an error message
            eprintln!("Failed to send message to broker: {:?}", send_error);
        }

        // Insert the message into the database
        broker
            .send(Event::DatabaseInsert {
                sender: name.clone(),
                receiver: dest.clone(),
                message_text: msg,
            })
            .await
            .expect("Failed to insert into database ");
    }

    Ok(())
}

// Function to insert a message into the database
async fn insert_message_into_db(sender: &str, receiver: &[String], message_text: &str) -> Result<()> {
    // Database connection URL
    let db_url = "postgresql://postgres:root@localhost:5432/chat";
    // Create a connection pool to the database
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(db_url)
        .await?;

    // SQL query to insert the message into the database
    let query = "INSERT INTO messages (sender, message_text,receiver) VALUES ($1, $2, $3)";

    // Iterate over the recipients and execute the query for each
    for recipient in receiver {
        sqlx::query(query)
            .bind(sender)
            .bind(message_text)
            .bind(receiver)
            .execute(&pool)
            .await?;
    }

    Ok(())
}

// Function to handle writing messages to the client
async fn connection_writer_loop(
    messages: &mut Receiver<String>,
    stream: Arc<TcpStream>,
    shutdown: Receiver<Void>,
) -> Result<()> {
    let mut stream = &*stream;
    let mut messages = messages.fuse();
    let mut shutdown = shutdown.fuse();

    // Loop to select between incoming messages and shutdown signals
    loop {
        select! {
            msg = messages.next().fuse() => match msg {
                Some(msg) => stream.write_all(msg.as_bytes()).await?,
                None => break,
            },
            void = shutdown.next().fuse() => match void {
                Some(void) => match void {},
                None => break,
            }
        }
    }

    Ok(())
}

// Enum to represent different types of events
#[derive(Debug)]
enum Event {
    NewPeer {
        name: String,
        stream: Arc<TcpStream>,
        shutdown: Receiver<Void>,
    },
    Message {
        from: String,
        to: Vec<String>,
        msg: String,
    },
    DatabaseInsert {
        sender: String,
        receiver: Vec<String>,
        message_text: String,
    },
}

// Function to handle events in the broker loop
async fn broker_loop(events: Receiver<Event>) {
    // Channel for disconnecting clients
    let (disconnect_sender, mut disconnect_receiver) = mpsc::unbounded::<(String, Receiver<String>)>();
    // HashMap to store connected clients and their message senders
    let mut peers: HashMap<String, Sender<String>> = HashMap::new();
    // Create a fused stream for handling events
    let mut events = events.fuse();

    // Main event handling loop
    loop {
        let event = select! {
            event = events.next().fuse() => match event {
                Some(event) => event,
                None =>  {println!("No more events, exiting broker loop");
                break;},
            },
            disconnect = disconnect_receiver.next().fuse() => {
                let (name, _pending_messages) = disconnect.unwrap();
                assert!(peers.remove(&name).is_some());
                println!("Client {} disconnected", name);
                continue;
            },
        };

        match event {
            Event::Message { from, to, msg } => {
                // Send messages to the specified recipients
                //println!("Handling Message event: from={}, to={:?}, msg={}", from, to, msg);
                //println!("Peers before sending messages: {:?}", peers);

                for addr in to {
                    if let Some(peer) = peers.get_mut(&addr) {
                        let msg = format!("from {}: {}\n", from, msg);
                        peer.send(msg.clone()).await.unwrap();
                        println!("Sent message to client {}: {}", addr, msg.clone());
                    }
                    else {
                        println!("Client {} not found in peers map", addr);
                    }
                }
                //println!("Peers after sending messages: {:?}", peers);

            }
            Event::NewPeer { name, stream, shutdown } => {
                // Add a new client to the peers HashMap
                match peers.entry(name.clone()) {
                    Entry::Occupied(..) => {
                        println!("Client {} already exists in peers map", name);
                    },
                    Entry::Vacant(entry) => {
                        println!("Handling NewPeer event: name={}", name);
                        let (client_sender, mut client_receiver) = mpsc::unbounded();
                        entry.insert(client_sender);
                        let mut disconnect_sender = disconnect_sender.clone();
                        // Spawn a connection writer loop for each client
                        spawn_and_log_error(async move {
                            let res = connection_writer_loop(&mut client_receiver, stream, shutdown).await;
                            disconnect_sender.send((name, client_receiver)).await
                                .expect("Receiver dropped");
                            res
                        });
                        //println!("Added new client to peers map: {}", name.clone());

                    }
                }
            }
            Event::DatabaseInsert { sender, receiver, message_text } => {
                // Insert messages into the database
                if let Err(err) = insert_message_into_db(&sender, &receiver, &message_text).await {
                    eprintln!("Failed to insert message into the database: {}", err);
                } else {
                    println!("Message successfully inserted into the database");
                }
            }
            _ => println!("Unhandled event: {:?}", event),

        }
    }

    // Drop peers and disconnect_sender to clean up resources
    drop(peers);
    drop(disconnect_sender);

    // Wait for disconnect_receiver to finish
    while let Some((_name, _pending_messages)) = disconnect_receiver.next().await {
    }
}

// Function to spawn a task and log errors
fn spawn_and_log_error<F>(fut: F) -> task::JoinHandle<()>
where
    F: Future<Output = Result<()>> + Send + 'static,
{
    task::spawn(async move {
        if let Err(e) = fut.await {
            eprintln!("{}", e)
        }
    })
}
