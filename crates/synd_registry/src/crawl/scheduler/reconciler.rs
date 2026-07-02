use crate::{
    crawl::scheduler::{ScanTick, driver::SchedDriver},
    event::{Processor, ProcessorId},
};

const DEFAULT_BATCH_SIZE: usize = 100;

pub struct SchedReconciler {
    batch_size: usize,
    driver: SchedDriver,
}

impl SchedReconciler {
    pub(crate) fn new(driver: SchedDriver) -> Self {
        Self {
            batch_size: DEFAULT_BATCH_SIZE,
            driver,
        }
    }

    pub(crate) fn with_batch_size(driver: SchedDriver, batch_size: usize) -> Self {
        Self { batch_size, driver }
    }
}

impl Processor for SchedReconciler {
    type Input = ScanTick;

    fn id(&self) -> ProcessorId {
        ProcessorId::CrawlScheduler
    }
}
