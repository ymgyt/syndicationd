use bitflags::bitflags;

bitflags! {
    pub(super) struct Should: u64 {
        const Render = 1 << 0;
        const Quit = 1 << 1;

    }
}
