use crate::{
    audio::{AudioMixer, AudioMixerTask},
    commands::{CommandContext, Executor, SessionTools},
    config::{
        AudioEffectSettings, BehaviorSettings, ExternalToolsSettings, FarewellMode, GreetingMode,
    },
    error::Error,
    protos::{self, generated::Mumble::CryptSetup},
};
use protobuf::{Message, SpecialFields};
use rustls_pki_types::{CertificateDer, PrivateKeyDer, ServerName, pem::PemObject};
use std::{collections::HashMap, sync::Arc, vec};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::mpsc,
};
use tokio_rustls::{
    TlsConnector,
    client::TlsStream,
    rustls::{ClientConfig, RootCertStore},
};

use crate::protos::generated::Mumble;
use crate::verifier;

/// Escapes HTML entities in a string for safe display
fn escape_html(input: &str) -> String {
    input
        .replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
        .replace("\"", "&quot;")
        .replace("'", "&#x27;")
}

/// Converts minimal markdown to HTML for better formatting in Mumble
pub fn markdown_to_html(input: &str) -> String {
    let mut result = input.to_string();

    // Convert **bold** to <b>bold</b>
    while let Some(start) = result.find("**") {
        if let Some(end) = result[start + 2..].find("**") {
            let end_pos = start + 2 + end;
            let bold_text = &result[start + 2..end_pos];
            let replacement = format!("<b>{}</b>", escape_html(bold_text));
            result.replace_range(start..end_pos + 2, &replacement);
        } else {
            break;
        }
    }

    // Convert `code` to <tt>code</tt> for monospace with proper HTML escaping
    while let Some(start) = result.find("`") {
        if let Some(end) = result[start + 1..].find("`") {
            let end_pos = start + 1 + end;
            let code_text = &result[start + 1..end_pos];
            let replacement = format!("<tt>{}</tt>", escape_html(code_text));
            result.replace_range(start..end_pos + 1, &replacement);
        } else {
            break;
        }
    }

    // Convert newlines to HTML line breaks
    result = result.replace("\n", "<br>");

    result
}

pub struct ConnectionOptions {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: Option<String>,
    pub cert: String,
    pub key: String,
    pub timeout: Option<u64>,
    pub data_dir: Option<String>,
    pub behavior_settings: BehaviorSettings,
    pub audio_effects: AudioEffectSettings,
    pub external_tools: ExternalToolsSettings,
}

pub enum OutgoingMessage {
    AudioData(Vec<u8>),       // audio data, encoded through opus
    TextMessage(String, u32), // channel message
    PrivMessage(String, u32), // private message
    Raw(u16, Vec<u8>),        // raw message type and payload
    Ping,
}

pub struct WriterTask {
    sender: mpsc::Sender<OutgoingMessage>,
    task: tokio::task::JoinHandle<Result<(), Error>>,
}

pub struct Writer {
    writer: tokio::io::WriteHalf<TlsStream<TcpStream>>,
    receiver: mpsc::Receiver<OutgoingMessage>,
}

impl WriterTask {
    pub fn new(writer: tokio::io::WriteHalf<TlsStream<TcpStream>>) -> Self {
        let (sender, receiver) = mpsc::channel(100); // Channel with a buffer size of 100

        let task = tokio::spawn(async move {
            let writer_task = Writer::new(writer, receiver);
            writer_task.run().await
        });

        WriterTask { sender, task }
    }
}

impl Writer {
    pub fn new(
        writer: tokio::io::WriteHalf<TlsStream<TcpStream>>,
        receiver: mpsc::Receiver<OutgoingMessage>,
    ) -> Self {
        Self { writer, receiver }
    }

    pub async fn run(mut self) -> Result<(), Error> {
        loop {
            match self.receiver.recv().await {
                Some(OutgoingMessage::AudioData(data)) => {
                    self.write_mumble_frame(protos::types::MESSAGE_UDP_TUNNEL, data)
                        .await?;
                }
                Some(OutgoingMessage::TextMessage(msg, channel)) => {
                    let payload = Mumble::TextMessage {
                        message: Some(msg),
                        channel_id: vec![channel],
                        ..Default::default()
                    }
                    .write_to_bytes()?;
                    self.write_mumble_frame(protos::types::MESSAGE_TEXT_MESSAGE, payload)
                        .await?;
                }
                Some(OutgoingMessage::PrivMessage(msg, target)) => {
                    let payload = Mumble::TextMessage {
                        message: Some(msg),
                        session: vec![target],
                        ..Default::default()
                    }
                    .write_to_bytes()?;
                    self.write_mumble_frame(protos::types::MESSAGE_TEXT_MESSAGE, payload)
                        .await?;
                }
                Some(OutgoingMessage::Ping) => {
                    self.write_mumble_frame(protos::types::MESSAGE_PING, vec![])
                        .await?;
                }
                Some(OutgoingMessage::Raw(msg_type, payload)) => {
                    self.write_mumble_frame(msg_type, payload).await?;
                }
                None => {
                    // Channel closed, exit the loop
                    info!("Writer task channel closed, exiting");
                    return Ok(());
                }
            }
        }
    }

    async fn write_mumble_frame(&mut self, msg_type: u16, payload: Vec<u8>) -> Result<(), Error> {
        let msg_len = payload.len() as u32;
        let mut header = [0u8; 6];

        header[0..2].copy_from_slice(&msg_type.to_be_bytes());
        header[2..6].copy_from_slice(&msg_len.to_be_bytes());

        self.writer
            .write_all(&header)
            .await
            .map_err(|e| Error::ConnectionError(format!("Failed to write header: {}", e)))?;

        self.writer
            .write_all(&payload)
            .await
            .map_err(|e| Error::ConnectionError(format!("Failed to write payload: {}", e)))?;

        Ok(())
    }
}

pub struct SessionCommandTools {
    mixer: AudioMixerTask,
}

pub struct Session {
    crypt_setup: Option<CryptSetup>,
    channels: HashMap<u32, Mumble::ChannelState>,
    users: HashMap<u32, Mumble::UserState>,
    writer: WriterTask,
    reader: tokio::io::ReadHalf<TlsStream<TcpStream>>,
    last_server_ping: Option<Mumble::Ping>,
    server_version: Option<Mumble::Version>,
    audio_mixer: AudioMixerTask,
    command_executor: Executor,
    current_user_id: Option<u32>,
    current_channel_id: Option<u32>,
    sounds_manager: Option<Arc<crate::sounds::SoundsManager>>,
    alias_manager: Option<Arc<crate::alias::AliasManager>>,
    user_settings_manager: Option<Arc<crate::user_settings::UserSettingsManager>>,
    behavior_settings: BehaviorSettings,
    audio_effects: AudioEffectSettings,
    external_tools: ExternalToolsSettings,
    sound_history:
        std::sync::Mutex<std::collections::VecDeque<(String, chrono::DateTime<chrono::Utc>)>>,
}

impl Session {
    /// Get the threebot configuration paths
    fn get_threebot_paths_from_dir(
        data_dir: Option<&str>,
    ) -> Result<(std::path::PathBuf, std::path::PathBuf, std::path::PathBuf), Error> {
        let threebot_dir = if let Some(dir) = data_dir {
            std::path::PathBuf::from(dir)
        } else {
            // Get home directory using dirs crate for cross-platform compatibility
            let home_dir = dirs::home_dir().ok_or_else(|| {
                Error::ConnectionError("Unable to determine home directory".to_string())
            })?;
            home_dir.join(".threebot")
        };

        let sounds_dir = threebot_dir.join("sounds");
        let database_path = threebot_dir.join("database.sql");
        let trusted_certs_dir = threebot_dir.join("trusted_certificates");

        // Ensure the .threebot directory exists
        std::fs::create_dir_all(&threebot_dir).map_err(|e| {
            Error::ConnectionError(format!("Failed to create .threebot directory: {}", e))
        })?;

        Ok((sounds_dir, database_path, trusted_certs_dir))
    }

    /// Get the threebot configuration paths (using default ~/.threebot)
    fn get_threebot_paths()
    -> Result<(std::path::PathBuf, std::path::PathBuf, std::path::PathBuf), Error> {
        Self::get_threebot_paths_from_dir(None)
    }

    pub async fn new(options: ConnectionOptions) -> Result<Self, Error> {
        let mut root_cert_store = RootCertStore::empty();
        root_cert_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

        let cert_chain = CertificateDer::pem_file_iter(&options.cert)
            .map_err(|e| {
                Error::InvalidCertificate(format!(
                    "Error opening certificate: {}: {}",
                    options.cert,
                    e.to_string()
                ))
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| {
                Error::InvalidCertificate(format!(
                    "Error reading certificate: {}: {}",
                    options.cert,
                    e.to_string()
                ))
            })?;

        let key_der = PrivateKeyDer::from_pem_file(&options.key).map_err(|e| {
            Error::InvalidCertificate(format!(
                "Error reading private key: {}: {}",
                options.key,
                e.to_string()
            ))
        })?;

        info!("Connecting to {} as {}", options.host, options.username);

        // Resolve hostname to IP address
        let ip = tokio::net::lookup_host((options.host.as_str(), options.port))
            .await
            .map_err(|e| {
                Error::ConnectionError(format!("Failed to resolve {}: {}", options.host, e))
            })?
            .next()
            .ok_or_else(|| {
                Error::ConnectionError(format!("No IP address found for {}", options.host))
            })?;

        debug!("Resolved {} to {}", options.host, ip);

        // Initialize a new session with the given destination address
        let socket = TcpStream::connect(ip).await.map_err(|e| {
            Error::ConnectionError(format!("Failed to connect to {}: {}", options.host, e))
        })?;

        // Initialize paths
        let (sounds_dir, database_path, trusted_certs_dir) =
            Self::get_threebot_paths_from_dir(options.data_dir.as_deref())?;

        let config = ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(
                verifier::PromptingCertVerifier::new(Some(trusted_certs_dir)),
            ))
            .with_client_auth_cert(cert_chain, key_der)?;

        let server_name = if let Ok(ip_addr) = options.host.parse::<std::net::IpAddr>() {
            ServerName::IpAddress(ip_addr.into())
        } else {
            ServerName::try_from(options.host.clone()).map_err(|e| {
                Error::ConnectionError(format!("Invalid server name {}: {}", options.host, e))
            })?
        };

        debug!("Resolved server name: {:?}", server_name);

        let stream = TlsConnector::from(Arc::new(config))
            .connect(server_name, socket)
            .await?;

        let (reader, writer) = tokio::io::split(stream);

        info!("TLS session established OK");

        let writer_task = WriterTask::new(writer);

        writer_task
            .sender
            .send(OutgoingMessage::Raw(
                protos::types::MESSAGE_VERSION,
                Mumble::Version {
                    version_v1: Some(1),
                    version_v2: Some(2),
                    release: Some("1.0.0".into()),
                    os: Some("rust".into()),
                    os_version: Some("5.4.0".into()),
                    special_fields: SpecialFields::default(),
                }
                .write_to_bytes()?,
            ))
            .await
            .map_err(|e| {
                Error::ConnectionError(format!("Failed to send version message: {}", e))
            })?;

        info!("Sent version message to server");

        // Write Authenticate message
        writer_task
            .sender
            .send(OutgoingMessage::Raw(
                protos::types::MESSAGE_AUTHENTICATE,
                Mumble::Authenticate {
                    username: Some(options.username.clone()),
                    password: options.password.clone(),
                    tokens: vec![],
                    celt_versions: vec![0, 1, 2],
                    opus: Some(true),
                    client_type: Some(1),
                    special_fields: SpecialFields::default(),
                }
                .write_to_bytes()?,
            ))
            .await
            .map_err(|e| {
                Error::ConnectionError(format!("Failed to send authenticate message: {}", e))
            })?;

        info!("Sent authenticate message to server");

        let audio_mixer = AudioMixer::spawn(
            writer_task.sender.clone(),
            &options.behavior_settings,
            &options.audio_effects,
        );

        // Initialize database manager
        let database_manager = match crate::database::DatabaseManager::new(&database_path).await {
            Ok(manager) => {
                info!("Database manager initialized successfully");
                manager
            }
            Err(e) => {
                return Err(Error::DatabaseError(format!(
                    "Failed to initialize database: {}",
                    e
                )));
            }
        };

        // Initialize sounds manager
        let sounds_manager =
            match crate::sounds::SoundsManager::new(database_manager.pool_clone(), sounds_dir) {
                Ok(manager) => {
                    info!("Sounds manager initialized successfully");
                    Some(Arc::new(manager))
                }
                Err(e) => {
                    warn!("Failed to initialize sounds manager: {}", e);
                    None
                }
            };

        // Initialize alias manager
        let alias_manager = {
            let manager = crate::alias::AliasManager::new(database_manager.pool_clone());
            info!("Alias manager initialized successfully");
            Some(Arc::new(manager))
        };

        // Initialize user settings manager
        let user_settings_manager = {
            let manager =
                crate::user_settings::UserSettingsManager::new(database_manager.pool_clone());
            info!("User settings manager initialized successfully");
            Some(Arc::new(manager))
        };

        Ok(Session {
            reader,
            audio_mixer,
            writer: writer_task,
            crypt_setup: None,
            channels: HashMap::new(),
            users: HashMap::new(),
            last_server_ping: None,
            server_version: None,
            command_executor: Executor::new(),
            current_user_id: None,
            current_channel_id: None,
            sounds_manager,
            alias_manager,
            user_settings_manager,
            behavior_settings: options.behavior_settings,
            audio_effects: options.audio_effects,
            external_tools: options.external_tools,
            sound_history: std::sync::Mutex::new(std::collections::VecDeque::new()),
        })
    }

    /// Writes a Mumble frame to the stream.
    ///
    /// ## Arguments
    /// * `stream` - The TLS stream to write to.
    /// * `msg_type` - The type of the message to write.
    /// * `payload` - The payload of the message to write.
    ///
    /// ## Returns
    /// * `Ok(())` if the frame was written successfully.
    /// * `Err(Error)` if there was an error writing the frame.
    async fn write_mumble_frame(
        writer: &mut tokio::io::WriteHalf<TlsStream<TcpStream>>,
        msg_type: u16,
        payload: Vec<u8>,
    ) -> Result<(), Error> {
        let msg_len = payload.len() as u32;
        let mut header = [0u8; 6];
        header[0..2].copy_from_slice(&msg_type.to_be_bytes());
        header[2..6].copy_from_slice(&msg_len.to_be_bytes());

        writer.write_all(&header).await?;
        writer.write_all(&payload).await?;

        Ok(())
    }

    /// Receives a Mumble frame from the stream.
    ///
    /// ## Arguments
    /// * `stream` - The TLS stream to read from.
    ///
    /// ## Returns
    /// * `Ok((u16, Vec<u8>))` containing the message type and payload if the frame was received successfully.
    /// * `Err(Error)` if there was an error receiving the frame.
    async fn receive_mumble_frame(
        reader: &mut tokio::io::ReadHalf<TlsStream<TcpStream>>,
    ) -> Result<(u16, Vec<u8>), Error> {
        // Read the 6-byte header
        let mut header = [0u8; 6];
        reader.read_exact(&mut header).await?;

        let msg_type = u16::from_be_bytes([header[0], header[1]]);
        let msg_len = u32::from_be_bytes([header[2], header[3], header[4], header[5]]) as usize;

        let mut buf = vec![0u8; msg_len];
        reader.read_exact(&mut buf).await?;

        Ok((msg_type, buf))
    }

    pub async fn start_main_loop(mut self) -> Result<(), Error> {
        // Main loop for handling incoming messages

        // Start ping writer task
        let ping_interval = 15; // seconds
        let ping_writer = self.writer.sender.clone();

        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(ping_interval)).await;

                if let Err(e) = ping_writer
                    .send(OutgoingMessage::Raw(protos::types::MESSAGE_PING, vec![]))
                    .await
                {
                    warn!("Failed to send ping message: {}", e);
                    break;
                }
            }
        });

        loop {
            let (msg_type, msg_payload) = Session::receive_mumble_frame(&mut self.reader).await?;

            match msg_type {
                protos::types::MESSAGE_VERSION => {
                    self.server_version = Some(Mumble::Version::parse_from_bytes(&msg_payload)?);
                    info!("Received server version");
                }
                protos::types::MESSAGE_UDP_TUNNEL => {}
                protos::types::MESSAGE_AUTHENTICATE => {
                    warn!("Unexpected Authenticate message received")
                }
                protos::types::MESSAGE_PING => {
                    let ping = Mumble::Ping::parse_from_bytes(&msg_payload)?;
                    self.last_server_ping = Some(ping);
                }
                protos::types::MESSAGE_REJECT => {
                    let reject = Mumble::Reject::parse_from_bytes(&msg_payload)?;
                    let err = format!(
                        "Server rejected connection: {}",
                        reject.reason.unwrap_or("(no reason provided)".into())
                    );
                    warn!("{}", err);
                    return Err(Error::ConnectionError(err));
                }
                protos::types::MESSAGE_SERVER_SYNC => {
                    let server_sync = Mumble::ServerSync::parse_from_bytes(&msg_payload)?;

                    // Set current user and channel from server sync
                    if let Some(session_id) = server_sync.session {
                        self.current_user_id = Some(session_id);
                        debug!("Set current user ID to: {}", session_id);

                        // Try to set channel from user state
                        self.try_set_channel_from_user_state();

                        // Fallback: set to root channel if we still don't have one
                        if self.current_channel_id.is_none() {
                            self.current_channel_id = Some(0);
                            debug!("Set fallback channel ID to root channel (0)");
                        }
                    }
                    if let Some(max_bandwidth) = server_sync.max_bandwidth {
                        // We can use this or other fields if needed
                        debug!("Server max bandwidth: {}", max_bandwidth);
                    }

                    info!(
                        "Server synchronized. Welcome message: {}",
                        server_sync.welcome_text()
                    );
                }
                protos::types::MESSAGE_CRYPT_SETUP => {
                    let crypt_setup = Mumble::CryptSetup::parse_from_bytes(&msg_payload)?;
                    self.crypt_setup = Some(crypt_setup);

                    debug!("Received voice crypt data");
                }
                protos::types::MESSAGE_CODEC_VERSION => {}
                protos::types::MESSAGE_PERMISSION_QUERY => {}
                protos::types::MESSAGE_CHANNEL_STATE => {
                    let channel_state = Mumble::ChannelState::parse_from_bytes(&msg_payload)?;
                    if channel_state.channel_id.is_none() {
                        warn!("Received ChannelState message without channel_id");
                        continue;
                    }

                    debug!(
                        "Received channel state for {}",
                        channel_state.name.as_ref().unwrap()
                    );
                    self.channels
                        .insert(channel_state.channel_id.unwrap(), channel_state);
                }
                protos::types::MESSAGE_CHANNEL_REMOVE => {
                    let channel_remove = Mumble::ChannelRemove::parse_from_bytes(&msg_payload)?;
                    if channel_remove.channel_id.is_none() {
                        warn!("Received ChannelRemove message without channel_id");
                        continue;
                    }

                    self.channels.remove(&channel_remove.channel_id.unwrap());
                }
                protos::types::MESSAGE_USER_STATE => {
                    let user_state = Mumble::UserState::parse_from_bytes(&msg_payload)?;
                    if user_state.session.is_none() {
                        warn!("Received UserState message without session");
                        continue;
                    }

                    let session_id = user_state.session.unwrap();

                    debug!(
                        "Received user state for {} (session: {})",
                        user_state.name.as_ref().unwrap_or(&"(unknown)".to_string()),
                        session_id
                    );

                    // Check if this is a new user joining (not already in our users map)
                    let is_new_user = !self.users.contains_key(&session_id)
                        && Some(session_id) != self.current_user_id;

                    // Store the user state, but preserve username if it exists in previous state
                    let mut updated_user_state = user_state.clone();
                    if updated_user_state.name.is_none()
                        || updated_user_state.name.as_ref().unwrap().is_empty()
                    {
                        // If the new state has no username, try to preserve the old one
                        if let Some(existing_user) = self.users.get(&session_id) {
                            if let Some(existing_name) = &existing_user.name {
                                if !existing_name.is_empty() {
                                    debug!(
                                        "Preserving username '{}' for session {}",
                                        existing_name, session_id
                                    );
                                    updated_user_state.name = Some(existing_name.clone());
                                }
                            }
                        }
                    }

                    self.users.insert(session_id, updated_user_state.clone());

                    // If this is our user, try to update current channel
                    if Some(session_id) == self.current_user_id {
                        self.try_set_channel_from_user_state();
                    }
                    // Also try if we haven't identified our user yet but this might be us
                    // (this handles the case where USER_STATE comes before SERVER_SYNC)
                    else if self.current_user_id.is_none() && self.current_channel_id.is_none() {
                        debug!(
                            "Received user state for session {} before knowing our own ID",
                            session_id
                        );
                    }
                    // Handle new user joining - play their greeting sound
                    else if is_new_user {
                        let user_name = updated_user_state
                            .name
                            .as_ref()
                            .unwrap_or(&"(unknown)".to_string())
                            .clone();
                        info!("New user joined: {} (session: {})", user_name, session_id);

                        // Play greeting sound in the background only if auto_greetings is enabled
                        if !matches!(self.behavior_settings.auto_greetings, GreetingMode::None) {
                            if let Err(e) = self.play_user_greeting(session_id).await {
                                warn!("Failed to play greeting for user {}: {}", user_name, e);
                            }
                        } else {
                            debug!(
                                "Auto greetings disabled, skipping greeting for user {}",
                                user_name
                            );
                        }
                    }
                }
                protos::types::MESSAGE_USER_REMOVE => {
                    let user_remove = Mumble::UserRemove::parse_from_bytes(&msg_payload)?;
                    if user_remove.session.is_none() {
                        warn!("Received UserRemove message without session");
                        continue;
                    }

                    let session_id = user_remove.session.unwrap();

                    // Get user info before removing them
                    let user_name = self
                        .users
                        .get(&session_id)
                        .and_then(|user| user.name.as_ref())
                        .unwrap_or(&"(unknown)".to_string())
                        .clone();

                    info!("User left: {} (session: {})", user_name, session_id);

                    // Play farewell sound before removing user data only if auto_farewells is enabled
                    if !matches!(self.behavior_settings.auto_farewells, FarewellMode::None) {
                        if let Err(e) = self.play_user_farewell(session_id).await {
                            warn!("Failed to play farewell for user {}: {}", user_name, e);
                        }
                    } else {
                        debug!(
                            "Auto farewells disabled, skipping farewell for user {}",
                            user_name
                        );
                    }

                    self.users.remove(&session_id);
                }
                protos::types::MESSAGE_TEXT_MESSAGE => {
                    let text_message = Mumble::TextMessage::parse_from_bytes(&msg_payload)?;

                    if text_message.actor.is_none() {
                        warn!("Received TextMessage without actor");
                        continue;
                    }

                    let actor_id = text_message.actor.unwrap();
                    let name = self
                        .users
                        .get(&actor_id)
                        .and_then(|user| user.name.clone())
                        .unwrap_or_else(|| "(unknown)".to_string());

                    let message_text = text_message
                        .message
                        .as_ref()
                        .unwrap_or(&"(no message)".to_string())
                        .clone();

                    info!("{} > {}", name, message_text);

                    // Check if this is a command (starts with !)
                    if message_text.starts_with("!") {
                        // Determine if this is a private message or channel message
                        let is_private_message = !text_message.session.is_empty();
                        let source_channel_id = if is_private_message {
                            None
                        } else {
                            text_message.channel_id.first().copied()
                        };

                        // Check if private commands are allowed
                        if is_private_message && !self.behavior_settings.allow_private_commands {
                            debug!(
                                "Private command from {} ignored (private commands disabled)",
                                name
                            );
                            let error_msg = "Private commands are disabled on this bot.";
                            if let Err(reply_err) =
                                self.send_private_message(actor_id, error_msg).await
                            {
                                warn!(
                                    "Failed to send private command disabled message: {}",
                                    reply_err
                                );
                            }
                            continue;
                        }

                        // Create command context
                        let context = CommandContext {
                            triggering_user_id: Some(actor_id),
                            source_channel_id,
                            is_private_message,
                        };

                        // Execute command - we need to handle this carefully due to borrowing
                        match self.execute_command_internal(&message_text, context).await {
                            Ok(_) => {
                                debug!("Command executed successfully");
                            }
                            Err(e) => {
                                warn!("Command execution failed: {}", e);
                                // Send error message back to user
                                let error_msg = format!("error: {}", e);
                                if let Err(reply_err) =
                                    self.send_error_reply(&error_msg, actor_id).await
                                {
                                    warn!("Failed to send error reply: {}", reply_err);
                                }
                            }
                        }
                    }
                }
                _ => {
                    warn!(
                        "Received unknown message type {} with payload length {}",
                        msg_type,
                        msg_payload.len()
                    );
                }
            }
        }
    }

    pub fn writer(&self) -> &mpsc::Sender<OutgoingMessage> {
        &self.writer.sender
    }

    async fn execute_command_internal(
        &mut self,
        command_text: &str,
        context: CommandContext,
    ) -> Result<(), Error> {
        debug!(
            "Executing command '{}' with current_channel_id: {:?}",
            command_text, self.current_channel_id
        );

        // Execute the command using self directly as SessionTools
        self.command_executor
            .execute(command_text, self, context)
            .await
    }

    async fn send_error_reply(&self, error_msg: &str, actor_id: u32) -> Result<(), Error> {
        let html = format!(
            "<span style=\"color: #ff4d4f;\">error: {}</span>",
            markdown_to_html(error_msg.trim_start_matches("error:").trim_start())
        );
        self.send_private_message(actor_id, &html).await
    }

    /// Attempts to set the current channel ID from our user state
    fn try_set_channel_from_user_state(&mut self) {
        if let Some(user_id) = self.current_user_id {
            if let Some(user_state) = self.users.get(&user_id) {
                if let Some(channel_id) = user_state.channel_id {
                    if self.current_channel_id != Some(channel_id) {
                        self.current_channel_id = Some(channel_id);
                        debug!("Set current channel to {} from user state", channel_id);
                    }
                } else {
                    debug!("Our user state (session {}) has no channel_id yet", user_id);
                }
            } else {
                debug!(
                    "Our user state (session {}) not found in users map",
                    user_id
                );
            }
        }
    }

    /// Plays a greeting sound for a user who just joined
    async fn play_user_greeting(&self, user_id: u32) -> Result<(), Error> {
        // Check if greetings are enabled
        match self.behavior_settings.auto_greetings {
            GreetingMode::None => return Ok(()), // No greetings
            _ => {}                              // Continue with greeting logic
        }

        // Get the username from the user ID
        let username = match self.users.get(&user_id) {
            Some(user_info) => match &user_info.name {
                Some(name) if !name.is_empty() => name.clone(),
                _ => {
                    warn!("User {} has no valid username", user_id);
                    // Only play random greeting in "All" mode when no username
                    if matches!(self.behavior_settings.auto_greetings, GreetingMode::All) {
                        self.play_random_greeting_sound().await?;
                    }
                    return Ok(());
                }
            },
            None => {
                warn!("User {} not found in users map", user_id);
                // Only play random greeting in "All" mode when user not found
                if matches!(self.behavior_settings.auto_greetings, GreetingMode::All) {
                    self.play_random_greeting_sound().await?;
                }
                return Ok(());
            }
        };

        debug!(
            "Attempting to play greeting for user {} ({})",
            username, user_id
        );

        if let Some(user_settings_manager) = &self.user_settings_manager {
            // Try to get the user's custom greeting
            match user_settings_manager.get_greeting(&username).await {
                Ok(Some(greeting_command)) => {
                    info!(
                        "Playing custom greeting for user {} ({}): {}",
                        username, user_id, greeting_command
                    );

                    // Create a context for the greeting command execution
                    let context = crate::commands::CommandContext {
                        triggering_user_id: Some(user_id),
                        source_channel_id: self.current_channel_id,
                        is_private_message: false,
                    };

                    // Execute the greeting command
                    if let Err(e) = self
                        .command_executor
                        .execute(&greeting_command, self, context)
                        .await
                    {
                        warn!(
                            "Failed to execute greeting command '{}' for user {} ({}): {}",
                            greeting_command, username, user_id, e
                        );
                        // Fall back to random sound only in "All" mode
                        if matches!(self.behavior_settings.auto_greetings, GreetingMode::All) {
                            self.play_random_greeting_sound().await?;
                        }
                    }
                }
                Ok(None) => {
                    // No custom greeting set
                    match self.behavior_settings.auto_greetings {
                        GreetingMode::All => {
                            // Play random sound when no custom greeting exists
                            info!(
                                "No custom greeting for user {} ({}), playing random sound",
                                username, user_id
                            );
                            self.play_random_greeting_sound().await?;
                        }
                        GreetingMode::Custom => {
                            // In custom mode, only play custom greetings - stay silent if none exists
                            debug!(
                                "No custom greeting for user {} ({}) and in custom mode, staying silent",
                                username, user_id
                            );
                        }
                        GreetingMode::None => {
                            // Already handled above, but included for completeness
                        }
                    }
                }
                Err(e) => {
                    warn!(
                        "Error checking greeting for user {} ({}): {}",
                        username, user_id, e
                    );
                    // Fall back to random sound only in "All" mode
                    if matches!(self.behavior_settings.auto_greetings, GreetingMode::All) {
                        self.play_random_greeting_sound().await?;
                    }
                }
            }
        } else {
            // No user settings manager - only play random sound in "All" mode
            if matches!(self.behavior_settings.auto_greetings, GreetingMode::All) {
                self.play_random_greeting_sound().await?;
            }
        }
        Ok(())
    }

    /// Plays a farewell sound for a user who just left
    async fn play_user_farewell(&self, user_id: u32) -> Result<(), Error> {
        // Check if farewells are enabled
        match self.behavior_settings.auto_farewells {
            FarewellMode::None => return Ok(()), // No farewells
            _ => {}                              // Continue with farewell logic
        }

        // Get the username from the user ID
        let username = match self.users.get(&user_id) {
            Some(user_info) => match &user_info.name {
                Some(name) if !name.is_empty() => name.clone(),
                _ => {
                    warn!("User {} has no valid username", user_id);
                    // Only play random farewell in "All" mode when no username
                    if matches!(self.behavior_settings.auto_farewells, FarewellMode::All) {
                        self.play_random_greeting_sound().await?;
                    }
                    return Ok(());
                }
            },
            None => {
                warn!("User {} not found in users map", user_id);
                // Only play random farewell in "All" mode when user not found
                if matches!(self.behavior_settings.auto_farewells, FarewellMode::All) {
                    self.play_random_greeting_sound().await?;
                }
                return Ok(());
            }
        };

        debug!(
            "Attempting to play farewell for user {} ({})",
            username, user_id
        );

        if let Some(user_settings_manager) = &self.user_settings_manager {
            // Try to get the user's custom farewell
            match user_settings_manager.get_farewell(&username).await {
                Ok(Some(farewell_command)) => {
                    info!(
                        "Playing custom farewell for user {} ({}): {}",
                        username, user_id, farewell_command
                    );

                    // Create a context for the farewell command execution
                    let context = crate::commands::CommandContext {
                        triggering_user_id: Some(user_id),
                        source_channel_id: self.current_channel_id,
                        is_private_message: false,
                    };

                    // Execute the farewell command
                    if let Err(e) = self
                        .command_executor
                        .execute(&farewell_command, self, context)
                        .await
                    {
                        warn!(
                            "Failed to execute farewell command '{}' for user {} ({}): {}",
                            farewell_command, username, user_id, e
                        );
                        // Fall back to random sound only in "All" mode
                        if matches!(self.behavior_settings.auto_farewells, FarewellMode::All) {
                            self.play_random_greeting_sound().await?;
                        }
                    }
                }
                Ok(None) => {
                    // No custom farewell set
                    match self.behavior_settings.auto_farewells {
                        FarewellMode::All => {
                            // Play random sound when no custom farewell exists
                            info!(
                                "No custom farewell for user {} ({}), playing random sound",
                                username, user_id
                            );
                            self.play_random_greeting_sound().await?;
                        }
                        FarewellMode::Custom => {
                            // In custom mode, only play custom farewells - stay silent if none exists
                            debug!(
                                "No custom farewell for user {} ({}) and in custom mode, staying silent",
                                username, user_id
                            );
                        }
                        FarewellMode::None => {
                            // Already handled above, but included for completeness
                        }
                    }
                }
                Err(e) => {
                    warn!(
                        "Error checking farewell for user {} ({}): {}",
                        username, user_id, e
                    );
                    // Fall back to random sound only in "All" mode
                    if matches!(self.behavior_settings.auto_farewells, FarewellMode::All) {
                        self.play_random_greeting_sound().await?;
                    }
                }
            }
        } else {
            // No user settings manager - only play random sound in "All" mode
            if matches!(self.behavior_settings.auto_farewells, FarewellMode::All) {
                self.play_random_greeting_sound().await?;
            }
        }
        Ok(())
    }

    /// Plays a random greeting sound
    async fn play_random_greeting_sound(&self) -> Result<(), Error> {
        // Execute the sound play command without arguments to get a random sound
        let context = crate::commands::CommandContext {
            triggering_user_id: None, // System-triggered
            source_channel_id: self.current_channel_id,
            is_private_message: false,
        };

        if let Err(e) = self
            .command_executor
            .execute("!sound play", self, context)
            .await
        {
            warn!("Failed to execute random greeting sound command: {}", e);
        }
        Ok(())
    }

    /// Get the current behavior settings
    pub fn behavior_settings(&self) -> &BehaviorSettings {
        &self.behavior_settings
    }
}

#[async_trait::async_trait]
impl SessionTools for Session {
    async fn play_sound(&self, file_path: &str) -> Result<(), Error> {
        self.audio_mixer
            .control()
            .play_sound(file_path)
            .await
            .map_err(|e| Error::ConnectionError(format!("Failed to play sound: {}", e)))
    }

    async fn play_sound_with_effects(
        &self,
        file_path: &str,
        effects: &[crate::audio::effects::AudioEffect],
    ) -> Result<(), Error> {
        self.audio_mixer
            .control()
            .play_sound_with_effects(file_path, effects)
            .await
            .map_err(|e| {
                Error::ConnectionError(format!("Failed to play sound with effects: {}", e))
            })
    }

    async fn play_sound_with_code(&self, file_path: &str, sound_code: &str) -> Result<(), Error> {
        let result = self.play_sound(file_path).await;
        if result.is_ok() {
            self.record_sound_played(sound_code);
        }
        result
    }

    async fn play_sound_with_effects_and_code(
        &self,
        file_path: &str,
        effects: &[crate::audio::effects::AudioEffect],
        sound_code: &str,
    ) -> Result<(), Error> {
        let result = self.play_sound_with_effects(file_path, effects).await;
        if result.is_ok() {
            self.record_sound_played(sound_code);
        }
        result
    }

    async fn stop_all_streams(&self) -> Result<(), Error> {
        self.audio_mixer.control().stop_all_streams().await;
        Ok(())
    }

    async fn send_channel_message(&self, channel_id: u32, message: &str) -> Result<(), Error> {
        self.writer
            .sender
            .send(OutgoingMessage::TextMessage(
                message.to_string(),
                channel_id,
            ))
            .await
            .map_err(|e| Error::ConnectionError(format!("Failed to send channel message: {}", e)))
    }

    async fn broadcast(&self, message: &str) -> Result<(), Error> {
        if let Some(channel_id) = self.current_channel_id {
            self.send_channel_message(channel_id, message).await
        } else {
            Err(Error::ConnectionError("No current channel set".to_string()))
        }
    }

    async fn send_private_message(&self, user_id: u32, message: &str) -> Result<(), Error> {
        self.writer
            .sender
            .send(OutgoingMessage::PrivMessage(message.to_string(), user_id))
            .await
            .map_err(|e| Error::ConnectionError(format!("Failed to send private message: {}", e)))
    }

    async fn reply(&self, message: &str) -> Result<(), Error> {
        // This method is now only used as a fallback by ContextAwareSessionTools
        // The actual routing is handled in the command layer
        let formatted_message = markdown_to_html(message);
        self.broadcast(&formatted_message).await
    }

    async fn reply_html(&self, html: &str) -> Result<(), Error> {
        // This method is now only used as a fallback by ContextAwareSessionTools
        // The actual routing is handled in the command layer
        self.broadcast(html).await
    }

    fn current_user_id(&self) -> Option<u32> {
        self.current_user_id
    }

    fn current_channel_id(&self) -> Option<u32> {
        self.current_channel_id
    }

    fn get_user_info(&self, user_id: u32) -> Option<&crate::protos::generated::Mumble::UserState> {
        self.users.get(&user_id)
    }

    fn get_channel_info(
        &self,
        channel_id: u32,
    ) -> Option<&crate::protos::generated::Mumble::ChannelState> {
        self.channels.get(&channel_id)
    }

    fn get_sounds_manager(&self) -> Option<std::sync::Arc<crate::sounds::SoundsManager>> {
        self.sounds_manager.clone()
    }

    fn get_alias_manager(&self) -> Option<std::sync::Arc<crate::alias::AliasManager>> {
        self.alias_manager.clone()
    }

    fn get_user_settings_manager(
        &self,
    ) -> Option<std::sync::Arc<crate::user_settings::UserSettingsManager>> {
        self.user_settings_manager.clone()
    }

    async fn execute_command(&self, command: &str, context: &CommandContext) -> Result<(), Error> {
        self.command_executor
            .execute(command, self, context.clone())
            .await
    }

    fn behavior_settings(&self) -> &crate::config::BehaviorSettings {
        &self.behavior_settings
    }

    fn audio_effect_settings(&self) -> &crate::config::AudioEffectSettings {
        &self.audio_effects
    }

    fn external_tools_settings(&self) -> &crate::config::ExternalToolsSettings {
        &self.external_tools
    }

    fn record_sound_played(&self, sound_code: &str) {
        if let Ok(mut history) = self.sound_history.lock() {
            let now = chrono::Utc::now();
            history.push_front((sound_code.to_string(), now));

            // Keep only the last 50 entries to prevent unlimited growth
            while history.len() > 50 {
                history.pop_back();
            }
        }
    }

    fn get_sound_history(&self, limit: usize) -> Vec<(String, chrono::DateTime<chrono::Utc>)> {
        if let Ok(history) = self.sound_history.lock() {
            history.iter().take(limit).cloned().collect()
        } else {
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_markdown_to_html() {
        // Test bold formatting
        assert_eq!(
            markdown_to_html("This is **bold** text"),
            "This is <b>bold</b> text"
        );

        // Test code formatting
        assert_eq!(
            markdown_to_html("Use `!alias` command"),
            "Use <tt>!alias</tt> command"
        );

        // Test combined formatting
        assert_eq!(
            markdown_to_html("**Bold** and `code` together"),
            "<b>Bold</b> and <tt>code</tt> together"
        );

        // Test multiple bold sections
        assert_eq!(
            markdown_to_html("**First** and **Second** bold"),
            "<b>First</b> and <b>Second</b> bold"
        );

        // Test with HTML entities that need escaping in bold text
        assert_eq!(
            markdown_to_html("**<script>** is dangerous"),
            "<b>&lt;script&gt;</b> is dangerous"
        );

        // Test with HTML entities that need escaping in code text
        assert_eq!(
            markdown_to_html("Use `<code>` tags"),
            "Use <tt>&lt;code&gt;</tt> tags"
        );

        // Test unclosed code block (should remain unchanged if no closing backtick)
        assert_eq!(markdown_to_html("Start `code here"), "Start `code here");

        // Test bullets with newlines converted to <br>
        assert_eq!(
            markdown_to_html("• First item\n• Second item"),
            "• First item<br>• Second item"
        );

        // Test newline conversion
        assert_eq!(
            markdown_to_html("Line 1\nLine 2\nLine 3"),
            "Line 1<br>Line 2<br>Line 3"
        );

        // Test combined formatting with newlines
        assert_eq!(
            markdown_to_html("**Header**\nSome text with `code`\nAnother line"),
            "<b>Header</b><br>Some text with <tt>code</tt><br>Another line"
        );
    }
}
