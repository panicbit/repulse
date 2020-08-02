use anyhow::*;
use tokio::fs;
use tokio::net::UnixStream;
use crate::{command, Broker, PROTOCOL_VERSION, tag_struct};
use command::{ServerInfo, Command, AuthReply};

pub struct Client {
    pub broker: Broker,
}

impl Client {
    pub async fn connect() -> Result<Self> {
        let cookie = load_cookie().await
            .context("Loading cookie failed")?;
        let conn = UnixStream::connect("/run/user/1000/pulse/native").await
            .context("Connecting to server failed")?;
        let broker = Broker::start(conn);
        let mut client = Self {
            broker,
        };

        client.send_command::<_, AuthReply>(command::Auth {
            protocol_version: PROTOCOL_VERSION,
            cookie,
        }).await?;

        Ok(client)
    }

    pub async fn get_server_info(&mut self) -> Result<ServerInfo> {
        self.send_command::<_, ServerInfo>(command::GetServerInfo).await
    }

    async fn send_command<C, R>(&mut self, command: C) -> Result<R>
    where
        C: Command + tag_struct::Put,
        R: tag_struct::Pop,
    {
        let mut reply = self.broker.send_command(command)?.await?;
        let parsed_reply = reply.pop::<R>()?;

        if !reply.is_empty() {
            eprintln!("Incomplete packet parse. Remaining fields: {:#?}", reply);
        }

        Ok(parsed_reply)
    }
}

async fn load_cookie() -> Result<Vec<u8>> {
    let path = dirs::config_dir()
        .unwrap_or_default()
        .join("pulse/cookie");
    let cookie = fs::read(path).await?;

    Ok(cookie)
}
