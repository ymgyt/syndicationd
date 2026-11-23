pub type Time = synd_feed::types::Time;

pub trait TimeExt {
    fn local_ymd(&self) -> String;
    fn local_ymd_hm(&self) -> String;
}

impl TimeExt for Time {
    fn local_ymd(&self) -> String {
        format_local(self, "%Y-%m-%d")
    }

    fn local_ymd_hm(&self) -> String {
        format_local(self, "%Y-%m-%d %H:%M (%:z)")
    }
}

pub(crate) fn format_local(time: &Time, format: &str) -> String {
    #[cfg(not(feature = "integration"))]
    let time = time.with_timezone(&chrono::Local);
    time.format(format).to_string()
}
