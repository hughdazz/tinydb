pub mod commands;

use std::{io, net::SocketAddr};

use async_recursion::async_recursion;
use byteorder::{BigEndian, ByteOrder};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
};

use anyhow::{Error, Result};

use crate::sql::PGResult;

use self::commands::{Message, StartupMessage, PROTOCOL_VERSION_NUMBER, SSL_REQUEST_NUMBER};

/// Connection implements the Postgres wire protocol (version 3 of the protocol, implemented
/// by Postgres 7.4 an later). receive() reads protocol messages, and return a Message type
/// to be executed by connection handler.
///
/// The connection handler execute the commands returned by receive() method and use the
/// connection to send the appropriate messages back to the client.
#[derive(Debug)]
pub struct Connection {
    // The `TcpStream` used to read and write data back and from the client.
    stream: BufReader<TcpStream>,
}

impl Connection {
    /// Create a new `Connection`, backed by `socket`. Read and write buffers
    /// are initialized.
    pub fn new(socket: TcpStream) -> Connection {
        Connection {
            stream: BufReader::new(socket),
        }
    }

    /// Read a single message from tcp stream.
    ///
    /// The function waits until it has retrieved enough data to parse a message.
    pub async fn receive(&mut self) -> Result<Message> {
        let message = commands::decode(&mut self.stream).await?;
        Ok(message)
    }

    /// Send a query result back to the client.
    pub async fn send_result(&mut self, result: PGResult) -> Result<()> {
        let rows = result.tuples.len();

        commands::encode(
            &mut self.stream,
            Message::RowDescriptor(result.desc.clone()),
        )
        .await?;
        commands::encode(&mut self.stream, Message::DataRow(result)).await?;
        self.command_complete(&format!("SELECT {}", rows)).await?;
        Ok(())
    }

    /// Send the given error back to the client.
    pub async fn send_error(&mut self, err: Error) -> Result<()> {
        commands::encode(
            &mut self.stream,
            Message::ErrorResponse(commands::ErrorResponse { error: err }),
        )
        .await?;
        Ok(())
    }

    /// Send to the client that the command returned by receive() is completed.
    pub async fn command_complete(&mut self, tag: &str) -> Result<()> {
        commands::encode(
            &mut self.stream,
            Message::CommandComplete(String::from(tag)),
        )
        .await?;
        self.ready_for_query().await?;
        Ok(())
    }

    /// Send a ReadyForQuery to the client.
    pub async fn ready_for_query(&mut self) -> Result<()> {
        commands::encode(&mut self.stream, Message::ReadyForQuery).await?;
        Ok(())
    }

    /// Return the startup message from the client.
    #[async_recursion]
    pub async fn startup_message(&mut self) -> Result<StartupMessage> {
        let msg_size = self.stream.read_u32().await? - 4;

        let mut buf = vec![0; msg_size as usize];
        self.stream.read(&mut buf).await?;
        let code = BigEndian::read_u32(&buf);

        match code {
            PROTOCOL_VERSION_NUMBER => StartupMessage::decode(&buf),
            SSL_REQUEST_NUMBER => {
                self.stream.write(&"N".as_bytes()).await?;
                self.startup_message().await
            }
            _ => anyhow::bail!("Unexpected startup code: {}", code),
        }
    }

    /// Send an AuthenticationOk with a ReadForQuery command back to the client.
    pub async fn send_authentication_ok(&mut self) -> Result<()> {
        commands::encode(&mut self.stream, Message::AuthenticationOk).await?;
        self.ready_for_query().await?;
        Ok(())
    }

    /// Returns the remote address that this stream is connected to.
    pub fn peer_addr(&self) -> io::Result<SocketAddr> {
        self.stream.get_ref().peer_addr()
    }
}
