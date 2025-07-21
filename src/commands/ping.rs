use super::{Command, SessionTools, CommandContext};

#[derive(Default)]
pub struct PingCommand;

#[async_trait::async_trait]
impl Command for PingCommand {
    async fn execute(&mut self, tools: &dyn SessionTools, _context: CommandContext, _args: Vec<String>) -> Result<(), crate::error::Error> {
        tools.reply("Pong!").await?;
        Ok(())
    }

    fn name(&self) -> &str { "ping" }
    
    fn description(&self) -> &str {
        "Responds with 'Pong!' to test command functionality"
    }
}