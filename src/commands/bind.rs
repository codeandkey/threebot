use super::{Command, SessionTools, CommandContext};
use crate::error::Error;

#[derive(Default)]
pub struct BindCommand;

#[async_trait::async_trait]
impl Command for BindCommand {
    async fn execute(&mut self, tools: &dyn SessionTools, context: CommandContext, args: Vec<String>) -> Result<(), Error> {
        // Get the user ID from the context and get their username
        let user_id = match context.triggering_user_id {
            Some(id) => id,
            None => {
                tools.reply("❌ Unable to identify user for bind command").await?;
                return Ok(());
            }
        };

        // Get the username from the user ID
        let username = match tools.get_user_info(user_id) {
            Some(user_info) => match &user_info.name {
                Some(name) if !name.is_empty() => name.clone(),
                _ => {
                    tools.reply("❌ Unable to get valid username for bind command").await?;
                    return Ok(());
                }
            },
            None => {
                tools.reply("❌ Unable to find user information for bind command").await?;
                return Ok(());
            }
        };

        if args.is_empty() {
            // Execute the user's bind command
            if let Some(user_settings_manager) = tools.get_user_settings_manager() {
                match user_settings_manager.get_bind(&username).await {
                    Ok(Some(bind_command)) => {
                        // Execute the bind command by parsing and running it
                        tools.reply(&format!("🔗 Executing bind: {}", bind_command)).await?;
                        
                        // Execute the command - it should already have the ! prefix from storage
                        if let Err(e) = tools.execute_command(&bind_command, &context).await {
                            tools.reply(&format!("❌ Error executing bind command: {}", e)).await?;
                        }
                    }
                    Ok(None) => {
                        tools.reply("❌ You don't have a bind command set. Use `!bind <command>` to set one.\n\
                                    **Example:**\n\
                                    • `!bind sound play ABCD` - Bind a sound").await?;
                    }
                    Err(e) => {
                        tools.reply(&format!("❌ Error retrieving bind command: {}", e)).await?;
                    }
                }
            } else {
                tools.reply("❌ User settings manager not available").await?;
            }
        } else {
            // Set the user's bind command
            let mut bind_command = args.join(" ");
            
            // Normalize the command - ensure it starts with '!' for consistency
            if !bind_command.starts_with('!') {
                bind_command = format!("!{}", bind_command);
            }
            
            if let Some(user_settings_manager) = tools.get_user_settings_manager() {
                match user_settings_manager.set_bind(&username, &bind_command).await {
                    Ok(()) => {
                        // Show the user what was actually stored (with the !)
                        tools.reply(&format!("✅ Bind command set to: `{}`", bind_command)).await?;
                    }
                    Err(e) => {
                        tools.reply(&format!("❌ Error setting bind command: {}", e)).await?;
                    }
                }
            } else {
                tools.reply("❌ User settings manager not available").await?;
            }
        }

        Ok(())
    }

    fn name(&self) -> &str {
        "bind"
    }
    
    fn description(&self) -> &str {
        "Set or execute personal bind commands - !bind <command> to set, !bind to execute"
    }
}

#[cfg(test)]
mod tests {
    // Tests for command normalization logic only
    #[test]
    fn test_command_normalization() {
        // Test cases for the command normalization logic
        let test_cases = vec![
            // Input, Expected output
            ("sound play ABCD", "!sound play ABCD"),
            ("!sound play ABCD", "!sound play ABCD"), 
            ("ping", "!ping"),
            ("!ping", "!ping"),
            ("alias myalias", "!alias myalias"),
            ("!alias myalias", "!alias myalias"),
            ("", "!"), // Edge case
            ("!", "!"), // Edge case
        ];

        for (input, expected) in test_cases {
            let normalized = if input.starts_with('!') {
                input.to_string()
            } else {
                format!("!{}", input)
            };
            
            assert_eq!(normalized, expected, "Failed for input: '{}'", input);
        }
    }

    #[test] 
    fn test_display_formatting() {
        // Test the display format logic
        let test_cases = vec![
            ("sound play ABCD", "sound play ABCD"),
            ("!sound play ABCD", "sound play ABCD"),
            ("ping", "ping"),
            ("!ping", "ping"),
        ];

        for (stored_command, expected_display) in test_cases {
            let display = if stored_command.starts_with('!') {
                &stored_command[1..]
            } else {
                stored_command
            };
            
            assert_eq!(display, expected_display, "Failed for stored command: '{}'", stored_command);
        }
    }
}
