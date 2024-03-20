use crate::keymap::{macros::keymap, Keymaps};

pub fn default() -> Keymaps {
    let login = keymap!({
        "enter" => authenticate,
        "k" => move_up_authentication_provider,
        "j" => move_down_authentication_provider,
    });
    let tabs = keymap!({
        "tab" => move_right_tab_selection,
        "backtab" => move_left_tab_selection,
    });
    let entries = keymap!({
        "k" => move_up_entry,
        "j" => move_down_entry,
        "r" => reload_entries,
        "enter" => open_entry,
        "g" => {
           "g" => move_entry_first,
           "e" => move_entry_last,
        },
    });
    let subscription = keymap!({
        "a" => prompt_feed_subscription,
        "d" => prompt_feed_unsubscription,
        "k" => move_up_subscribed_feed,
        "j" => move_down_subscribed_feed,
        "r" => reload_subscription,
        "enter" => open_feed,
        "g" => {
            "g" => move_subscribed_feed_first,
            "e" => move_subscribed_feed_last,
        },
    });
    let global = keymap!({
        "q" | "C-c" =>  quit ,
    });

    Keymaps {
        login,
        tabs,
        entries,
        subscription,
        global,
    }
}
