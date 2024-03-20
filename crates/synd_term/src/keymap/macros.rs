macro_rules! keymap {
    ( @trie $cmd:ident ) =>  { $crate::keymap::KeyTrie::Command($crate::command::Command::$cmd()) };

    (@trie
        { $( $($key:literal)|+ => $value:tt, )+ }
    ) => {
        keymap!({ $( $($key)|+ => $value, )+ })
    };

    (
        { $( $($key:literal)|+ => $value:tt, )+ }
    ) => {
        {
            // TODO: calcu cap
            let mut map = ::std::collections::HashMap::new();
            $(
                $(
                    // TODO: parse KeyEvent from literal
                    let key_event = $crate::keymap::parse($key).unwrap();
                    let trie = keymap!(@trie $value );
                    map.insert(key_event, trie);
                )*
            )*
            let node =  $crate::keymap::KeyTrieNode { map };
            $crate::keymap::KeyTrie::Node(node)
        }
    };
}

pub(crate) use keymap;
