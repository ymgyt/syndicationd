pub type Time = synd_feed::types::Time;

pub trait TimeExt {
    fn local_ymd(&self) -> String;
}

impl TimeExt for Time {
    fn local_ymd(&self) -> String {
        self.naive_local().format("%Y/%m/%d").to_string()
    }
}
