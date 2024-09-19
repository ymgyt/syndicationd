use crate::{types::Time, uow::Work};

pub(crate) struct PingWork(Work<(), Time>);
