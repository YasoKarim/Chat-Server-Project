//Thanks to https://book.async.rs/tutorial/
//https://www.youtube.com/watch?v=TCERYbgvbq0
//https://doc.rust-lang.org/book/

//client/src/main.rs
extern crate async_std;
extern crate futures;

use async_std::{
    io::{stdin, BufReader, self},
    net::{TcpStream, ToSocketAddrs},
    prelude::*,
    task,
};
use futures::{select, FutureExt};

// Define a Result type for handling errors
type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

// Function to run the asynchronous main logic
fn run() -> Result<()> {
    // Block the current thread until the try_run function completes
    task::block_on(try_run("127.0.0.1:8080"))
}

// Main function to handle errors in the run function
fn main() {
    if let Err(err) = run() {
        eprintln!("Error: {:?}", err);
    }
}

// Asynchronous function to establish a connection and handle communication
async fn try_run(addr: impl ToSocketAddrs) -> Result<()> {
    // Connect to the server using TcpStream
    let stream = TcpStream::connect(addr).await?;
    // Create references to the stream for reading and writing
    let (reader, mut writer) = (&stream, &stream); 
    // Create a fused stream for reading lines from the server
    let mut lines_from_server = BufReader::new(reader).lines().fuse(); 
    // Create a fused stream for reading lines from stdin
    let mut lines_from_stdin = BufReader::new(stdin()).lines().fuse(); 

    // Main event loop
    loop {
        select! { // 3
            // Handle incoming lines from the server
            line = lines_from_server.next().fuse() => match line {
                Some(line) => {
                    let line = line?;
                    println!("Received from server: {}", line);
                },
              None => {
                    println!("No more lines from server, exiting event loop");
                    break;
                }},
            // Handle user input from stdin
            line = lines_from_stdin.next().fuse() => match line {
                Some(line) => {
                    let line = line?;
                  
                    
                    // Send the user's message to the server
                    // Prompt the user to enter a message
                    print!("Enter the message: ");
                    io::stdout().flush().await?;

                    // Write the user's input to the server
                    writer.write_all(line.as_bytes()).await?;
                    writer.write_all(b"\n").await?;
                    writer.flush().await?; // Flush the writer to send the data immediately

                    //writer.flush().await?; // Add this line if flushing is needed

                }
                None => break,
            }
        }
    }

    Ok(())
}
