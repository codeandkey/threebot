use super::{Command, CommandContext, SessionTools};
use crate::error::Error;

#[derive(Default)]
pub struct FarewellCommand;

#[async_trait::async_trait]
impl Command for FarewellCommand {
    async fn execute(
        &mut self,
        tools: &dyn SessionTools,
        context: CommandContext,
        args: Vec<String>,
    ) -> Result<(), Error> {
        // Get the user ID from the context and get their username
        let user_id = match context.triggering_user_id {
            Some(id) => id,
            None => {
                tools
                    .reply(" Unable to identify user for farewell command")
                    .await?;
                return Ok(());
            }
        };

        // Get the username from the user ID
        let username = match tools.get_user_info(user_id) {
            Some(user_info) => match &user_info.name {
                Some(name) if !name.is_empty() => name.clone(),
                _ => {
                    tools
                        .reply(" Unable to get valid username for farewell command")
                        .await?;
                    return Ok(());
                }
            },
            None => {
                tools
                    .reply(" Unable to find user information for farewell command")
                    .await?;
                return Ok(());
            }
        };

        if args.is_empty() {
            // Clear/unset the user's farewell command
            if let Some(user_settings_manager) = tools.get_user_settings_manager() {
                match user_settings_manager.clear_farewell(&username).await {
                    Ok(true) => {
                        tools
                            .reply(" Your farewell command has been removed")
                            .await?;
                    }
                    Ok(false) => {
                        tools
                            .reply(" You don't have a farewell command set to remove")
                            .await?;
                    }
                    Err(e) => {
                        tools
                            .reply(&format!(" Error removing farewell command: {}", e))
                            .await?;
                    }
                }
            } else {
                tools.reply(" User settings manager not available").await?;
            }
        } else {
            // Set the user's farewell command
            let mut farewell_command = args.join(" ");

            // Normalize the command - ensure it starts with '!' for consistency
            if !farewell_command.starts_with('!') {
                farewell_command = format!("!{}", farewell_command);
            }

            if let Some(user_settings_manager) = tools.get_user_settings_manager() {
                match user_settings_manager
                    .set_farewell(&username, &farewell_command)
                    .await
                {
                    Ok(()) => {
                        // Show the user what was actually stored (with the !)
                        tools
                            .reply(&format!(" Farewell command set to: `{}`", farewell_command))
                            .await?;
                    }
                    Err(e) => {
                        tools
                            .reply(&format!(" Error setting farewell command: {}", e))
                            .await?;
                    }
                }
            } else {
                tools.reply(" User settings manager not available").await?;
            }
        }

        Ok(())
    }

    fn description(&self) -> &str {
        "Set or remove personal farewell commands - !farewell <command> to set, !farewell to remove"
    }
}
