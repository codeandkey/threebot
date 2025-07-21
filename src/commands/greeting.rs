use super::{Command, SessionTools, CommandContext};
use crate::error::Error;

#[derive(Default)]
pub struct GreetingCommand;

#[async_trait::async_trait]
impl Command for GreetingCommand {
    async fn execute(&mut self, tools: &dyn SessionTools, context: CommandContext, args: Vec<String>) -> Result<(), Error> {
        // Get the user ID from the context and get their username
        let user_id = match context.triggering_user_id {
            Some(id) => id,
            None => {
                tools.reply("❌ Unable to identify user for greeting command").await?;
                return Ok(());
            }
        };

        // Get the username from the user ID
        let username = match tools.get_user_info(user_id) {
            Some(user_info) => match &user_info.name {
                Some(name) if !name.is_empty() => name.clone(),
                _ => {
                    tools.reply("❌ Unable to get valid username for greeting command").await?;
                    return Ok(());
                }
            },
            None => {
                tools.reply("❌ Unable to find user information for greeting command").await?;
                return Ok(());
            }
        };

        if args.is_empty() {
            // Clear/unset the user's greeting command
            if let Some(user_settings_manager) = tools.get_user_settings_manager() {
                match user_settings_manager.clear_greeting(&username).await {
                    Ok(true) => {
                        tools.reply("✅ Your greeting command has been removed").await?;
                    }
                    Ok(false) => {
                        tools.reply("❌ You don't have a greeting command set to remove").await?;
                    }
                    Err(e) => {
                        tools.reply(&format!("❌ Error removing greeting command: {}", e)).await?;
                    }
                }
            } else {
                tools.reply("❌ User settings manager not available").await?;
            }
        } else {
            // Set the user's greeting command
            let mut greeting_command = args.join(" ");
            
            // Normalize the command - ensure it starts with '!' for consistency
            if !greeting_command.starts_with('!') {
                greeting_command = format!("!{}", greeting_command);
            }
            
            if let Some(user_settings_manager) = tools.get_user_settings_manager() {
                match user_settings_manager.set_greeting(&username, &greeting_command).await {
                    Ok(()) => {
                        // Show the user what was actually stored (with the !)
                        tools.reply(&format!("✅ Greeting command set to: `{}`", greeting_command)).await?;
                    }
                    Err(e) => {
                        tools.reply(&format!("❌ Error setting greeting command: {}", e)).await?;
                    }
                }
            } else {
                tools.reply("❌ User settings manager not available").await?;
            }
        }

        Ok(())
    }

    fn name(&self) -> &str {
        "greeting"
    }
    
    fn description(&self) -> &str {
        "Set or remove personal greeting commands - !greeting <command> to set, !greeting to remove"
    }
}
