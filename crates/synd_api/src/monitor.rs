use tokio_metrics::TaskMonitor;

pub struct Monitors {
    pub gql: TaskMonitor,
}

impl Monitors {
    pub fn new() -> Self {
        Self {
            gql: TaskMonitor::new(),
        }
    }
}
