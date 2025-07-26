use std::{collections::HashMap, sync::Arc};

use tokio::sync::Mutex;

use crate::error::Error;

/// A context-aware SessionTools implementation that handles reply routing
struct ContextAwareSessionTools<'a> {
    tools: &'a dyn SessionTools,
    context: &'a CommandContext,
}

impl<'a> ContextAwareSessionTools<'a> {
    fn new(tools: &'a dyn SessionTools, context: &'a CommandContext) -> Self {
        Self { tools, context }
    }
}

#[async_trait::async_trait]
impl<'a> SessionTools for ContextAwareSessionTools<'a> {
    async fn play_sound(&self, file_path: &str) -> Result<(), Error> {
        self.tools.play_sound(file_path).await
    }

    async fn play_sound_with_effects(&self, file_path: &str, effects: &[crate::audio::effects::AudioEffect]) -> Result<(), Error> {
        self.tools.play_sound_with_effects(file_path, effects).await
    }

    async fn stop_all_streams(&self) -> Result<(), Error> {
        self.tools.stop_all_streams().await
    }
    
    async fn send_channel_message(&self, channel_id: u32, message: &str) -> Result<(), Error> {
        self.tools.send_channel_message(channel_id, message).await
    }
    
    async fn broadcast(&self, message: &str) -> Result<(), Error> {
        self.tools.broadcast(message).await
    }
    
    async fn send_private_message(&self, user_id: u32, message: &str) -> Result<(), Error> {
        self.tools.send_private_message(user_id, message).await
    }
    
    async fn reply(&self, message: &str) -> Result<(), Error> {
        // Always send as private message to the triggering user
        if let Some(user_id) = self.context.triggering_user_id {
            self.tools.send_private_message(user_id, &crate::session::markdown_to_html(message)).await
        } else {
            // Fallback to broadcast if no user ID
            self.tools.broadcast(&crate::session::markdown_to_html(message)).await
        }
    }
    
    async fn reply_html(&self, html: &str) -> Result<(), Error> {
        // Always send as private message to the triggering user
        if let Some(user_id) = self.context.triggering_user_id {
            self.tools.send_private_message(user_id, html).await
        } else {
            // Fallback to broadcast if no user ID
            self.tools.broadcast(html).await
        }
    }
    
    fn current_user_id(&self) -> Option<u32> {
        self.tools.current_user_id()
    }
    
    fn current_channel_id(&self) -> Option<u32> {
        self.tools.current_channel_id()
    }
    
    fn get_user_info(&self, user_id: u32) -> Option<&crate::protos::generated::Mumble::UserState> {
        self.tools.get_user_info(user_id)
    }
    
    fn get_channel_info(&self, channel_id: u32) -> Option<&crate::protos::generated::Mumble::ChannelState> {
        self.tools.get_channel_info(channel_id)
    }
    
    fn get_sounds_manager(&self) -> Option<Arc<crate::sounds::SoundsManager>> {
        self.tools.get_sounds_manager()
    }
    
    fn get_alias_manager(&self) -> Option<Arc<crate::alias::AliasManager>> {
        self.tools.get_alias_manager()
    }
    
    fn get_user_settings_manager(&self) -> Option<Arc<crate::user_settings::UserSettingsManager>> {
        self.tools.get_user_settings_manager()
    }
    
    async fn execute_command(&self, command: &str, context: &CommandContext) -> Result<(), Error> {
        self.tools.execute_command(command, context).await
    }
    
    fn behavior_settings(&self) -> &crate::config::BehaviorSettings {
        self.tools.behavior_settings()
    }
    
    fn audio_effect_settings(&self) -> &crate::config::AudioEffectSettings {
        self.tools.audio_effect_settings()
    }
    
    fn create_html_table(&self, headers: &[&str], rows: &[Vec<String>]) -> String {
        self.tools.create_html_table(headers, rows)
    }
}

/// Context and tools available to commands for interacting with the session
#[async_trait::async_trait]
pub trait SessionTools: Send + Sync {
    /// Play an audio file through the audio mixer
    async fn play_sound(&self, file_path: &str) -> Result<(), Error>;
    
    /// Play an audio file with effects through the audio mixer
    async fn play_sound_with_effects(&self, file_path: &str, effects: &[crate::audio::effects::AudioEffect]) -> Result<(), Error>;
    
    /// Stop all currently playing audio streams
    async fn stop_all_streams(&self) -> Result<(), Error>;
    
    /// Send a text message to a specific channel
    async fn send_channel_message(&self, channel_id: u32, message: &str) -> Result<(), Error>;
    
    /// Send a text message to the current channel
    async fn broadcast(&self, message: &str) -> Result<(), Error>;
    
    /// Send a private message to a specific user
    async fn send_private_message(&self, user_id: u32, message: &str) -> Result<(), Error>;
    
    /// Reply to the user who triggered the command (context-aware)
    async fn reply(&self, message: &str) -> Result<(), Error>;
    
    /// Reply with raw HTML (bypasses markdown conversion)
    async fn reply_html(&self, html: &str) -> Result<(), Error>;
    
    /// Get the current user's session ID
    fn current_user_id(&self) -> Option<u32>;
    
    /// Get the current channel ID
    fn current_channel_id(&self) -> Option<u32>;
    
    /// Get information about a user by ID
    fn get_user_info(&self, user_id: u32) -> Option<&crate::protos::generated::Mumble::UserState>;
    
    /// Get information about a channel by ID
    fn get_channel_info(&self, channel_id: u32) -> Option<&crate::protos::generated::Mumble::ChannelState>;
    
    /// Get access to the sounds manager for sound-related operations
    fn get_sounds_manager(&self) -> Option<Arc<crate::sounds::SoundsManager>>;
    
    /// Get access to the alias manager for alias-related operations
    fn get_alias_manager(&self) -> Option<Arc<crate::alias::AliasManager>>;
    
    /// Get access to the user settings manager for user-specific settings
    fn get_user_settings_manager(&self) -> Option<Arc<crate::user_settings::UserSettingsManager>>;
    
    /// Execute a command string
    async fn execute_command(&self, command: &str, context: &CommandContext) -> Result<(), Error>;
    
    /// Get the current behavior settings
    fn behavior_settings(&self) -> &crate::config::BehaviorSettings;
    
    /// Get the current audio effect settings
    fn audio_effect_settings(&self) -> &crate::config::AudioEffectSettings;
    
    /// Creates an HTML table with no borders, bold centered headers, and standard text rows
    fn create_html_table(&self, headers: &[&str], rows: &[Vec<String>]) -> String {
        let mut table = String::from("<table style=\"border-collapse: collapse; width: 100%; border: none;\">");
        
        // Add header row
        table.push_str("<tr>");
        for header in headers {
            table.push_str(&format!(
                "<th style=\"text-align: center; font-weight: bold; padding: 0 8px; border: none;\">{}</th>",
                header
            ));
        }
        table.push_str("</tr>");
        
        // Add data rows
        for row in rows {
            table.push_str("<tr>");
            for cell in row {
                table.push_str(&format!(
                    "<td style=\"text-align: left; padding: 0 8px; border: none;\">{}</td>",
                    cell
                ));
            }
            table.push_str("</tr>");
        }
        
        table.push_str("</table>");
        table
    }
}

/// Command execution context
#[derive(Clone)]
pub struct CommandContext {
    /// The user who triggered the command
    pub triggering_user_id: Option<u32>,
    /// The channel where the command was triggered
    pub source_channel_id: Option<u32>,
    /// Whether this was a private message
    pub is_private_message: bool,
}

#[async_trait::async_trait]
pub trait Command: Send + Sync {
    async fn execute(&mut self, tools: &dyn SessionTools, context: CommandContext, args: Vec<String>) -> Result<(), Error>;
    fn description(&self) -> &str { "No description available" }
}

pub mod ping;
pub mod sound;
pub mod alias;
pub mod bind;
pub mod greeting;
pub mod farewell;

// Include the generated command mappings
include!(concat!(env!("OUT_DIR"), "/commands_generated.rs"));

pub struct Executor {
    commands: HashMap<String, Arc<Mutex<Box<dyn Command>>>>, // arc/mutex to maintain state across multi-named commands
}

const MAX_ALIAS_DEPTH: u32 = 10; // Maximum alias expansion depth

impl Executor {
    pub fn new() -> Self {
        let mut commands = HashMap::new();
        
        // Manually register commands with their inferred names from filenames
        commands.insert("alias".to_string(), Arc::new(Mutex::new(Box::new(alias::AliasCommand::default()) as Box<dyn Command>)));
        commands.insert("bind".to_string(), Arc::new(Mutex::new(Box::new(bind::BindCommand::default()) as Box<dyn Command>)));
        commands.insert("farewell".to_string(), Arc::new(Mutex::new(Box::new(farewell::FarewellCommand::default()) as Box<dyn Command>)));
        commands.insert("greeting".to_string(), Arc::new(Mutex::new(Box::new(greeting::GreetingCommand::default()) as Box<dyn Command>)));
        commands.insert("ping".to_string(), Arc::new(Mutex::new(Box::new(ping::PingCommand::default()) as Box<dyn Command>)));
        commands.insert("sound".to_string(), Arc::new(Mutex::new(Box::new(sound::SoundCommand::default()) as Box<dyn Command>)));
        
        Executor { 
            commands,
        }
    }

    /// Sanitize command line by removing HTML link tags
    fn sanitize_command_line(cmdline: &str) -> String {
        let mut result = cmdline.to_string();
        
        // Remove <a href="...">...</a> tags, keeping only the inner text
        loop {
            if let Some(start_tag_pos) = result.find("<a ") {
                if let Some(close_tag_start) = result[start_tag_pos..].find('>') {
                    let close_tag_pos = start_tag_pos + close_tag_start + 1;
                    if let Some(end_tag_pos) = result[close_tag_pos..].find("</a>") {
                        let end_tag_start = close_tag_pos + end_tag_pos;
                        let inner_text = result[close_tag_pos..end_tag_start].to_string();
                        let full_end = end_tag_start + 4; // "</a>".len()
                        result.replace_range(start_tag_pos..full_end, &inner_text);
                    } else {
                        // Malformed tag, just remove the opening tag
                        result.replace_range(start_tag_pos..close_tag_pos, "");
                    }
                } else {
                    // Malformed tag, remove what we found
                    result.replace_range(start_tag_pos..start_tag_pos + 3, "");
                }
            } else {
                break;
            }
        }
        
        // Remove any other HTML tags as a fallback
        loop {
            if let Some(start) = result.find('<') {
                if let Some(end) = result[start..].find('>') {
                    result.replace_range(start..start + end + 1, "");
                } else {
                    // Malformed tag, just remove the '<'
                    result.replace_range(start..start + 1, "");
                }
            } else {
                break;
            }
        }
        
        result
    }

    pub async fn execute(&self, cmdline: &str, tools: &dyn SessionTools, context: CommandContext) -> Result<(), Error> {
        // Start with depth 0 for the public entry point
        self.execute_with_depth(cmdline, tools, context, 0).await
    }

    /// Internal method that tracks alias expansion depth
    async fn execute_with_depth(&self, cmdline: &str, tools: &dyn SessionTools, context: CommandContext, current_depth: u32) -> Result<(), Error> {
        // Sanitize the command line to remove HTML tags
        let sanitized_cmdline = Self::sanitize_command_line(cmdline);
        
        let mut parts = sanitized_cmdline.split_whitespace();
        let mut command_name = parts.next().ok_or_else(|| Error::InvalidArgument("No command provided".to_string()))?;

        if command_name.starts_with("!") {
            // Remove the leading '!' if present
            command_name = &command_name[1..];
        } else {
            return Err(Error::InvalidArgument("Command must start with '!'".to_string()));
        }

        let args: Vec<String> = parts.map(String::from).collect();

        // First, check if this is a built-in command
        if let Some(command) = self.commands.get(command_name) {
            let mut cmd = command.lock().await;
            let context_aware_tools = ContextAwareSessionTools::new(tools, &context);
            return cmd.execute(&context_aware_tools, context.clone(), args).await;
        }

        // If not a built-in command, check if it's an alias
        if let Some(alias_manager) = tools.get_alias_manager() {
            if let Ok(Some(alias)) = alias_manager.get_alias(command_name).await {
                // Check for maximum expansion depth
                if current_depth >= MAX_ALIAS_DEPTH {
                    return Err(Error::InvalidArgument(format!(
                        "Maximum alias expansion depth ({}) exceeded. Possible recursive alias: {}", 
                        MAX_ALIAS_DEPTH, command_name
                    )));
                }
                
                // Execute the alias commands with incremented depth
                let context_aware_tools = ContextAwareSessionTools::new(tools, &context);
                return self.execute_alias_commands(&alias.commands, &context_aware_tools, context.clone(), &args, current_depth + 1).await;
            }
        }

        // Neither built-in command nor alias found
        Err(Error::InvalidArgument(format!("Unknown command: {}", command_name)))
    }

    /// Executes alias commands, handling variable substitution
    async fn execute_alias_commands(&self, alias_commands: &str, tools: &dyn SessionTools, context: CommandContext, original_args: &[String], current_depth: u32) -> Result<(), Error> {
        // Implement sophisticated parameter substitution
        let mut expanded_commands = alias_commands.to_string();
        let mut performed_substitution = false;
        
        // Replace $@ with all original arguments
        if expanded_commands.contains("$@") {
            expanded_commands = expanded_commands.replace("$@", &original_args.join(" "));
            performed_substitution = true;
        }
        
        // Replace $# with argument count
        if expanded_commands.contains("$#") {
            expanded_commands = expanded_commands.replace("$#", &original_args.len().to_string());
            performed_substitution = true;
        }
        
        // Replace $1, $2, $3, etc. with individual arguments
        for (i, arg) in original_args.iter().enumerate() {
            let placeholder = format!("${}", i + 1);
            if expanded_commands.contains(&placeholder) {
                expanded_commands = expanded_commands.replace(&placeholder, arg);
                performed_substitution = true;
            }
        }

        // If no substitutions were performed, append all positional arguments to the last command
        if !performed_substitution {
            if let Some(last_semicolon) = expanded_commands.rfind(';') {
                expanded_commands.insert_str(last_semicolon + 1, &original_args.join(" "));
            } else {
                expanded_commands.push_str(&format!(" {}", original_args.join(" ")));
            }
        }

        // Split multiple commands by semicolon or newline and execute each
        for command_line in expanded_commands.split(';') {
            let command_line = command_line.trim();
            if !command_line.is_empty() {
                // Ensure the command starts with !
                let full_command = if command_line.starts_with('!') {
                    command_line.to_string()
                } else {
                    format!("!{}", command_line)
                };
                
                // Recursively execute the command with current depth (this will handle nested aliases)
                // Use Box::pin to handle recursion
                Box::pin(self.execute_with_depth(&full_command, tools, context.clone(), current_depth)).await?;
            }
        }
        
        Ok(())
    }

    // Get a singleton instance of a command by filename
    pub fn get_command_instance(command_name: &str) -> Option<Box<dyn Command>> {
        match command_name {
            "ping" => Some(create_ping_command()),
            _ => None,
        }
    }

    /// Helper method to create a CommandContext for text message commands
    pub fn create_text_command_context(
        triggering_user_id: Option<u32>,
        source_channel_id: Option<u32>,
        is_private_message: bool,
    ) -> CommandContext {
        CommandContext {
            triggering_user_id,
            source_channel_id,
            is_private_message,
        }
    }

    /// Helper method to execute a command from a text message
    pub async fn execute_from_text_message(
        &self,
        cmdline: &str,
        tools: &dyn SessionTools,
        triggering_user_id: Option<u32>,
        source_channel_id: Option<u32>,
        is_private_message: bool,
    ) -> Result<(), Error> {
        let context = Self::create_text_command_context(
            triggering_user_id,
            source_channel_id,
            is_private_message,
        );
        self.execute(cmdline, tools, context).await
    }
}

#[cfg(test)]
mod tests {
    use super::Executor;

    #[test]
    fn test_sanitize_command_line() {
        // Test basic HTML link removal
        let input = r#"!sounds pull <a href="https://example.com/video">https://example.com/video</a> 30 10"#;
        let expected = "!sounds pull https://example.com/video 30 10";
        assert_eq!(Executor::sanitize_command_line(input), expected);

        // Test multiple links
        let input = r#"!sounds pull <a href="https://example.com">link1</a> <a href="https://test.com">link2</a> 30 10"#;
        let expected = "!sounds pull link1 link2 30 10";
        assert_eq!(Executor::sanitize_command_line(input), expected);

        // Test other HTML tags
        let input = "!sounds <b>pull</b> <i>example</i> 30 10";
        let expected = "!sounds pull example 30 10";
        assert_eq!(Executor::sanitize_command_line(input), expected);

        // Test no HTML
        let input = "!sounds pull https://example.com 30 10";
        let expected = "!sounds pull https://example.com 30 10";
        assert_eq!(Executor::sanitize_command_line(input), expected);

        // Test malformed HTML
        let input = "!sounds pull <a href=example 30 10";
        let expected = "!sounds pull href=example 30 10";
        assert_eq!(Executor::sanitize_command_line(input), expected);

        // Test edge cases
        let input = "!sounds pull <> text <a>inside</a> more";
        let expected = "!sounds pull  text inside more";
        assert_eq!(Executor::sanitize_command_line(input), expected);
    }
}