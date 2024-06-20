pub type Time = synd_feed::types::Time;

pub trait TimeExt {
    fn local_ymd(&self) -> String;
    fn local_ymd_hm(&self) -> String;
}

#[cfg(feature = "integration")]
impl TimeExt for Time {
    fn local_ymd(&self) -> String {
        self.format("%Y-%m-%d").to_string()
    }

    fn local_ymd_hm(&self) -> String {
        self.format("%Y-%m-%d %H:%M (%:z)").to_string()
    }
}

#[cfg(not(feature = "integration"))]
impl TimeExt for Time {
    fn local_ymd(&self) -> String {
        self.with_timezone(&chrono::Local)
            .format("%Y-%m-%d")
            .to_string()
    }

    fn local_ymd_hm(&self) -> String {
        self.with_timezone(&chrono::Local)
            .format("%Y-%m-%d %H:%M (%:z)")
            .to_string()
    }
}
