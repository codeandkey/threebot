use super::{Command, CommandContext, SessionTools};

#[derive(Default)]
pub struct PingCommand;

#[async_trait::async_trait]
impl Command for PingCommand {
    async fn execute(
        &mut self,
        tools: &dyn SessionTools,
        _context: CommandContext,
        _args: Vec<String>,
    ) -> Result<(), crate::error::Error> {
        tools.reply("Pong!").await?;
        Ok(())
    }
}
