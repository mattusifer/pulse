use actix::prelude::*;
use std::process;

use crate::{
    config::{config, CommandConfig, CommandsConfig},
    error::Result,
    services::broadcast::OUTBOX,
    services::messages::{BroadcastEvent, ScheduleMessage},
};

pub struct CommandRunner {
    config: Vec<CommandConfig>,
}
impl CommandRunner {
    pub fn new() -> Self {
        let config = config().commands;
        if let Some(CommandsConfig { commands }) = config {
            Self { config: commands }
        } else {
            Self { config: vec![] }
        }
    }

    fn run_command(&self, command_config: &CommandConfig) -> Result<()> {
        let command_output = process::Command::new("bash")
            .arg(command_config.command.clone())
            .output()?;
        if command_config.alert {
            let message = BroadcastEvent::GenericMessage {
                title: "COMMAND OUTPUT".to_string(),
                body: String::from_utf8(command_output.stdout)?,
            };
            OUTBOX.push(message)?;
        }
        Ok(())
    }
}

impl Actor for CommandRunner {
    type Context = Context<Self>;
}

impl Handler<ScheduleMessage> for CommandRunner {
    type Result = Result<()>;

    fn handle(
        &mut self,
        msg: ScheduleMessage,
        _ctx: &mut Context<Self>,
    ) -> Self::Result {
        match msg {
            ScheduleMessage::RunCommand { command_id } => self
                .config
                .iter()
                .filter(|cmd_config| cmd_config.command_id == command_id)
                .map(|cmd_config| self.run_command(cmd_config))
                .collect::<Result<Vec<_>>>()
                .map(|_| ()),
            _ => Ok(()),
        }
    }
}
