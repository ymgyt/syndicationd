macro_rules! keymap {
    ( @count $token:tt ) => { () };

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
            // https://danielkeep.github.io/tlborm/book/blk-counting.html#slice-length
            let capacity = <[()]>::len(&[
                 $(
                     $( keymap!(@count $key) ),*
                ),*
            ]);
            let mut map = ::std::collections::HashMap::with_capacity(capacity);
            $(
                $(
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
