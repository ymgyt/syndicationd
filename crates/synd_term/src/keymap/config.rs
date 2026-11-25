use super::{CommandId, KeySequence, Layer};
use tuiframework::keymap::Binding;

pub type KeymapConfig = tuiframework::keymap::KeymapConfig<Layer, CommandId>;

pub(super) fn binding<const N: usize>(
    keys: [&str; N],
    command: CommandId,
    desc: Option<&str>,
) -> Binding<CommandId> {
    Binding {
        on: KeySequence::parse(keys).expect("valid built-in key sequence"),
        command,
        desc: desc.map(str::to_owned),
    }
}
