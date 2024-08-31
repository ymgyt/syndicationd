#[rustfmt::skip]
macro_rules! icon {
    (browse)            => { "󰏋" };
    (feeds)             => { "󰑫" };
    (feedsoff)          => { "󰑫" };
    (entries)           => { "󱉯" };
    (category)          => { "" };
    (calendar)          => { "" };
    (chat)              => { "󰭻" };
    (check)             => { "" };
    (comment)           => { "" };
    (cross)             => { "" };
    (discussion)        => { "" };
    (entry)             => { "󰯂" };
    (filter)            => { "󰈶" };
    (github)            => { "󰊤" };
    (google)            => { "󰊭" };
    (issueopen)         => { "" };
    (issuereopened)     => { "" };
    (issuenotplanned)   => { "" };
    (issueclosed)       => { "" };
    (label)             => { "󱍵" };
    (requirement)       => { "" };
    (open)              => { "󰏌" };
    (pullrequest)       => { "" };
    (pullrequestmerged) => { "" };
    (pullrequestclosed) => { "" };
    (pullrequestdraft)  => { "" };
    (repository)        => { "" };
    (search)            => { "" };
    (summary)           => { "󱙓" };
    (tag)               => { "󰓹" };
    (unread)            => { "󰮒" };
}

pub(crate) use icon;
