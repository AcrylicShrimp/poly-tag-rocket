use std::{path::PathBuf, sync::Arc};

pub struct MetricService {
    file_base_path: PathBuf,
}

impl MetricService {
    pub fn new(file_base_path: impl Into<PathBuf>) -> Arc<Self> {
        Arc::new(Self {
            file_base_path: file_base_path.into(),
        })
    }
}
