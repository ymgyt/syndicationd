pub type Time = synd_feed::types::Time;

pub trait TimeExt {
    fn local_ymd(&self) -> String;
    fn local_ymd_hm(&self) -> String;
}

impl TimeExt for Time {
    fn local_ymd(&self) -> String {
        self.naive_local().format("%Y-%m-%d").to_string()
    }

    fn local_ymd_hm(&self) -> String {
        self.naive_local().format("%Y-%m-%d %H:%M").to_string()
    }
}
