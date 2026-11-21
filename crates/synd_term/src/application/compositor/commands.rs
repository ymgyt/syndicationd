use tracing::instrument;

use super::Compositor;
use crate::{application::effects::ComponentEffects, command::Command, operation::Operations};

impl Compositor {
    pub(super) fn apply_command(&mut self, command: Command) -> Operations {
        let operations = self.dispatch_command(command);
        self.sync_keymap();
        operations
    }

    #[instrument(name = "apply_command", skip_all, fields(%command))]
    fn dispatch_command(&mut self, command: Command) -> Operations {
        let Some(mut reply) = self.components.apply_command(command) else {
            return Operations::Nop;
        };
        let mut replies = self.notify(&std::mem::take(&mut reply.effects.events));
        replies.push(reply);
        self.run(replies)
    }

    pub(super) fn run(&mut self, replies: Vec<ComponentEffects>) -> Operations {
        let mut operations = Vec::new();
        for ComponentEffects { sender, effects } in replies {
            operations.push(effects.operations);
            for command in effects.commands {
                match command {
                    Command::Open => self.overlay.open(sender),
                    Command::Close => self.overlay.close(sender),
                    command => operations.push(self.dispatch_command(command)),
                }
            }
        }
        operations.into()
    }
}
