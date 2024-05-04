# Async Chat Server Project

This project is a chat server implementation using Rust programming language, PostgreSQL database, and a broker pattern. The code is implemented based on the idea of a broker from the [Async Programming in Rust](https://book.async.rs/tutorial/index.html) book.

## Team

This project is a collaborative effort and we would like to acknowledge the contributions of:

- [SJa-638](https://github.com/SJa-638)
- [CrazySultana](https://github.com/CrazySultana)

We appreciate their hard work and dedication in making this project a success.

## Features

- Allows multiple clients to connect and communicate with each other in real-time.
- Uses a broker pattern to handle message distribution between clients.
- Messages sent by both the sender and receiver are saved in a PostgreSQL database for future reference.
- The broker is responsible for receiving and processing events from various components within the system.
- The broker maintains a HashMap named peers that keeps track of connected clients.
- The broker handles events related to inserting messages into a database.
- The broker manages client disconnections.
- The broker_loop function runs in its own asynchronous task, continuously processing events in the background.

## Prerequisites

Before running the chat server, make sure you have the following installed:

- Rust programming language: [Installation Guide](https://www.rust-lang.org/tools/install)
- PostgreSQL database: [Installation Guide](https://www.postgresql.org/download/)

## Setup

1. Clone the repository:

    ```bash
    git clone https://github.com/YasoKarim/Chat_Server.git
    ```

2. Navigate to the project directory:

    ```bash
    cd Chat-Server-Project
    ```

3. Install the required dependencies:

    ```bash
    cargo build
    ```

4. Set up the PostgreSQL database:

    - Create a new database in PostgreSQL for the chat server.
    - Update the database connection details in the configuration file (`config.toml`) with your PostgreSQL credentials.

5. Run the chat server:

    ```bash
    cargo run
    ```

## Usage

- Clients can connect to the chat server using a WebSocket client.
- Messages sent by clients will be distributed to all connected clients using the broker pattern.
- All messages sent by clients will be saved in the PostgreSQL database for future reference.

## Contributing

Contributions are welcome! If you find any issues or have suggestions for improvements, please open an issue or submit a pull request.

## Note

The server and the client are made in different projects using `cargo new (proj_name)` then `cargo build`. On running the code you write `cargo run` for the server first then the client.