use super::{Command, CommandContext, SessionTools};
use crate::error::Error;

#[derive(Default)]
pub struct AliasCommand;

#[async_trait::async_trait]
impl Command for AliasCommand {
    async fn execute(
        &mut self,
        tools: &dyn SessionTools,
        context: CommandContext,
        mut args: Vec<String>,
    ) -> Result<(), Error> {
        if args.is_empty() {
            // List first page of aliases
            self.list_aliases_paginated(tools, 0).await
        } else if args.len() == 1 {
            match args[0].as_str() {
                "list" => {
                    // Explicit list command (first page)
                    self.list_aliases_paginated(tools, 0).await
                }
                "help" => {
                    // Show help
                    tools.reply("🔧 Alias Command Help\n\n\
                        • `!alias` or `!alias list` - List first page of aliases\n\
                        • `!alias list <page>` - List aliases by page (20 per page)\n\
                        • `!alias search <term> [page]` - Search aliases\n\
                        • `!alias create <name> <commands...>` - Create an alias\n\
                        • `!alias <name> <commands...>` - Create an alias\n\
                        • `!alias remove <name>` - Remove an alias\n\
                        • `!alias help` - Show this help\n\n\
                        Variable substitution:\n\
                        • `$@` - All arguments passed to alias\n\
                        • `$1`, `$2`, etc. - Individual arguments\n\
                        • `$#` - Number of arguments\n\
                        • `$recent` - Most recently played sound code\n\n\
                        Examples:\n\
                        • `!alias greet sound play hello; sound play $1`\n\
                        • `!alias welcome greet $@; sound play fanfare`\n\
                        • `!alias again sound play $recent`").await
                }
                _ => {
                    tools.reply("Usage: !alias [list|help] or !alias <name> <commands...> or !alias remove <name>").await
                }
            }
        } else if args.len() == 2 && args[0] == "remove" {
            // Remove an alias: !alias remove <name>
            let alias_name = &args[1];
            self.remove_alias(tools, alias_name).await
        } else if args.len() == 2 && args[0] == "list" {
            // List with page number: !alias list <page>
            match args[1].parse::<u64>() {
                Ok(page) => {
                    let page = if page > 0 { page - 1 } else { 0 }; // Convert 1-based to 0-based
                    self.list_aliases_paginated(tools, page).await
                }
                Err(_) => {
                    tools
                        .reply("Invalid page number. Usage: !alias list <page>")
                        .await
                }
            }
        } else if args.len() == 2 && args[0] == "search" {
            // Search: !alias search <term> (first page)
            self.search_aliases(tools, &args[1], 0).await
        } else if args.len() == 2 {
            // Create an alias: !alias <name> <command>
            let alias_name = &args[0];
            let commands = &args[1];

            // Get author name from user info
            let author = if let Some(user_id) = context.triggering_user_id {
                tools
                    .get_user_info(user_id)
                    .and_then(|user| user.name.as_ref())
                    .cloned()
                    .unwrap_or_else(|| "unknown".to_string())
            } else {
                "unknown".to_string()
            };

            self.create_alias(tools, alias_name, &author, commands)
                .await
        } else if args.len() == 3 && args[0] == "search" {
            // Search with page: !alias search <term> <page>
            match args[2].parse::<u64>() {
                Ok(page) => {
                    let page = if page > 0 { page - 1 } else { 0 }; // Convert 1-based to 0-based
                    self.search_aliases(tools, &args[1], page).await
                }
                Err(_) => {
                    tools
                        .reply("Invalid page number. Usage: !alias search <term> [page]")
                        .await
                }
            }
        } else {
            // Allow the keyword 'create' to be dropped here to explicitly create an alias
            // for when the alias name matches one of the subcommands

            if &args[0] == "create" {
                args.remove(0); // Remove 'create' keyword
            }

            // Create an alias: !alias <name> <commands...>
            let alias_name = &args[0];
            let commands = args[1..].join(" ");

            // Get author name from user info
            let author = if let Some(user_id) = context.triggering_user_id {
                tools
                    .get_user_info(user_id)
                    .and_then(|user| user.name.as_ref())
                    .cloned()
                    .unwrap_or_else(|| "unknown".to_string())
            } else {
                "unknown".to_string()
            };

            self.create_alias(tools, alias_name, &author, &commands)
                .await
        }
    }

    fn description(&self) -> &str {
        "Create or list command aliases. Usage: !alias <name> <commands...> or !alias list"
    }
}

impl AliasCommand {
    /// Creates a new alias
    async fn create_alias(
        &self,
        tools: &dyn SessionTools,
        name: &str,
        author: &str,
        commands: &str,
    ) -> Result<(), Error> {
        // Get the alias manager
        if let Some(alias_manager) = tools.get_alias_manager() {
            match alias_manager.create_alias(name, author, commands).await {
                Ok(_) => {
                    tools
                        .reply(&format!("✅ Alias '{}' created successfully", name))
                        .await?;
                }
                Err(e) => {
                    tools
                        .reply(&format!("❌ Failed to create alias: {}", e))
                        .await?;
                }
            }
            return Ok(());
        }

        tools.reply("❌ Alias manager not available").await
    }

    /// Lists all aliases
    async fn list_aliases(&self, tools: &dyn SessionTools) -> Result<(), Error> {
        // Get the alias manager
        if let Some(alias_manager) = tools.get_alias_manager() {
            match alias_manager.list_aliases().await {
                Ok(aliases) => {
                    if aliases.is_empty() {
                        tools.reply("📋 No aliases defined").await?;
                    } else {
                        let mut response = String::from("📋 Aliases:\n");
                        for alias in aliases {
                            response.push_str(&format!(
                                "• **{}** (by {}): `{}`\n",
                                alias.name, alias.author, alias.commands
                            ));
                        }
                        tools.reply(&response).await?;
                    }
                }
                Err(e) => {
                    tools
                        .reply(&format!("❌ Failed to list aliases: {}", e))
                        .await?;
                }
            }
            return Ok(());
        }

        tools.reply("❌ Alias manager not available").await
    }

    /// Lists aliases with pagination
    async fn list_aliases_paginated(
        &self,
        tools: &dyn SessionTools,
        page: u64,
    ) -> Result<(), Error> {
        if let Some(alias_manager) = tools.get_alias_manager() {
            match alias_manager.list_aliases_paginated(page, 20).await {
                Ok(aliases) => {
                    if aliases.is_empty() {
                        if page == 0 {
                            tools.reply("📋 No aliases defined").await?;
                        } else {
                            tools.reply("📋 No aliases found on this page").await?;
                        }
                    } else {
                        // Get total count for pagination info
                        let total_count = alias_manager.count_aliases().await.unwrap_or(0);
                        let total_pages = (total_count + 19) / 20; // 20 per page, round up

                        let mut response =
                            format!("📋 Aliases (Page {} of {})\n\n", page + 1, total_pages);

                        // Prepare table data
                        let headers = &["Name", "Author", "Commands"];
                        let rows: Vec<Vec<String>> = aliases
                            .iter()
                            .map(|alias| {
                                vec![
                                    format!("<strong>{}</strong>", alias.name),
                                    alias.author.clone(),
                                    format!("<code>{}</code>", alias.commands),
                                ]
                            })
                            .collect();

                        response.push_str("<div style=\"text-align: center;\">");
                        response.push_str(&tools.create_html_table_paginated(
                            headers, 
                            &rows, 
                            Some((page + 1) as usize), // Convert 0-based to 1-based
                            Some(20), 
                            Some(total_count as usize),
                            "!alias list"
                        ));
                        response.push_str("</div>");
                        tools.reply_html(&response).await?;
                    }
                }
                Err(e) => {
                    tools
                        .reply(&format!("❌ Failed to list aliases: {}", e))
                        .await?;
                }
            }
            return Ok(());
        }

        tools.reply("❌ Alias manager not available").await
    }

    /// Searches aliases with pagination
    async fn search_aliases(
        &self,
        tools: &dyn SessionTools,
        search_term: &str,
        page: u64,
    ) -> Result<(), Error> {
        if let Some(alias_manager) = tools.get_alias_manager() {
            match alias_manager.search_aliases(search_term, page, 20).await {
                Ok(aliases) => {
                    if aliases.is_empty() {
                        if page == 0 {
                            tools
                                .reply(&format!("🔍 No aliases found matching '{}'", search_term))
                                .await?;
                        } else {
                            tools
                                .reply(&format!(
                                    "🔍 No aliases found matching '{}' on this page",
                                    search_term
                                ))
                                .await?;
                        }
                    } else {
                        // Get total count for pagination info
                        let total_count = alias_manager
                            .count_search_aliases(search_term)
                            .await
                            .unwrap_or(0);
                        let total_pages = (total_count + 19) / 20; // 20 per page, round up

                        let mut response = format!(
                            "🔍 Aliases matching '{}' (Page {} of {})\n\n",
                            search_term,
                            page + 1,
                            total_pages
                        );

                        // Prepare table data
                        let headers = &["Name", "Author", "Commands"];
                        let rows: Vec<Vec<String>> = aliases
                            .iter()
                            .map(|alias| {
                                vec![
                                    format!("<strong>{}</strong>", alias.name),
                                    alias.author.clone(),
                                    format!("<code>{}</code>", alias.commands),
                                ]
                            })
                            .collect();

                        response.push_str("<div style=\"text-align: center;\">");
                        response.push_str(&tools.create_html_table_paginated(
                            headers, 
                            &rows, 
                            Some((page + 1) as usize), // Convert 0-based to 1-based
                            Some(20), 
                            Some(total_count as usize),
                            &format!("!alias search {}", search_term)
                        ));
                        response.push_str("</div>");
                        tools.reply_html(&response).await?;
                    }
                }
                Err(e) => {
                    tools
                        .reply(&format!("❌ Failed to search aliases: {}", e))
                        .await?;
                }
            }
            return Ok(());
        }

        tools.reply("❌ Alias manager not available").await
    }

    /// Removes an alias
    async fn remove_alias(&self, tools: &dyn SessionTools, name: &str) -> Result<(), Error> {
        // Get the alias manager
        if let Some(alias_manager) = tools.get_alias_manager() {
            match alias_manager.delete_alias(name).await {
                Ok(true) => {
                    tools
                        .reply(&format!("✅ Alias '{}' removed successfully", name))
                        .await?;
                }
                Ok(false) => {
                    tools
                        .reply(&format!("❌ Alias '{}' not found", name))
                        .await?;
                }
                Err(e) => {
                    tools
                        .reply(&format!("❌ Failed to remove alias: {}", e))
                        .await?;
                }
            }
            return Ok(());
        }

        tools.reply("❌ Alias manager not available").await
    }
}
