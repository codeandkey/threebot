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
            // Execute the user's greeting command
            if let Some(user_settings_manager) = tools.get_user_settings_manager() {
                match user_settings_manager.get_greeting(&username).await {
                    Ok(Some(greeting_command)) => {
                        // Execute the greeting command
                        tools.reply(&format!("🎉 Executing greeting: {}", greeting_command)).await?;
                        
                        // Execute the command - it should already have the ! prefix from storage
                        if let Err(e) = tools.execute_command(&greeting_command, &context).await {
                            tools.reply(&format!("❌ Error executing greeting command: {}", e)).await?;
                        }
                    }
                    Ok(None) => {
                        tools.reply("❌ You don't have a greeting command set. Use `!greeting <command>` to set one.\n\
                                    **Examples:**\n\
                                    • `!greeting sounds play ABCD` - Play a sound when you join\n\
                                    • `!greeting alias myhello` - Execute an alias when you join").await?;
                    }
                    Err(e) => {
                        tools.reply(&format!("❌ Error retrieving greeting command: {}", e)).await?;
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
        "Set or execute personal greeting commands - !greeting <command> to set, !greeting to execute"
    }
}
