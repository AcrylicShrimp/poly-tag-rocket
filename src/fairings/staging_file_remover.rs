use crate::services::StagingFileService;
use chrono::Duration;
use parking_lot::Mutex;
use rocket::{
    fairing::{Fairing, Info},
    Orbit, Rocket,
};
use std::sync::Arc;

pub struct StagingFileRemover {
    period: Duration,
    expiration: Duration,
    stop_signal_sender: Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
    task_join_handle: Mutex<Option<tokio::task::JoinHandle<()>>>,
}

impl StagingFileRemover {
    pub fn new(period: Duration, expiration: Duration) -> Self {
        StagingFileRemover {
            period,
            expiration,
            stop_signal_sender: Mutex::new(None),
            task_join_handle: Mutex::new(None),
        }
    }
}

#[rocket::async_trait]
impl Fairing for StagingFileRemover {
    fn info(&self) -> Info {
        Info {
            name: "Staging File Remover",
            kind: rocket::fairing::Kind::Liftoff | rocket::fairing::Kind::Shutdown,
        }
    }

    async fn on_liftoff(&self, rocket: &Rocket<Orbit>) {
        let period = self.period;
        let expiration = self.expiration;

        log::info!(target: "staging_file_remover", period:%, expiration:%; "Starting staging file remover.");

        let (stop_signal_sender, stop_signal_receiver) = tokio::sync::oneshot::channel();
        let staging_file_service = rocket.state::<Arc<StagingFileService>>().unwrap().clone();

        let task_join_handle = tokio::spawn(remove_expired_staging_files_task(
            stop_signal_receiver,
            period,
            expiration,
            staging_file_service,
        ));

        let mut stop_signal_sender_lock = self.stop_signal_sender.lock();
        *stop_signal_sender_lock = Some(stop_signal_sender);
        drop(stop_signal_sender_lock);

        let mut task_join_handle_lock = self.task_join_handle.lock();
        *task_join_handle_lock = Some(task_join_handle);
        drop(task_join_handle_lock);

        log::info!(target: "staging_file_remover", "Staging file remover started.");
    }

    async fn on_shutdown(&self, _rocket: &Rocket<Orbit>) {
        log::info!(target: "staging_file_remover", "Shutting down staging file remover.");

        let task_join_handle = {
            let mut stop_signal_sender_lock = self.stop_signal_sender.lock();
            let stop_signal_sender = stop_signal_sender_lock.take();
            drop(stop_signal_sender_lock);

            if let Some(stop_signal_sender) = stop_signal_sender {
                stop_signal_sender.send(()).ok();
            }

            let mut task_join_handle_lock = self.task_join_handle.lock();
            let task_join_handle = task_join_handle_lock.take();
            drop(task_join_handle_lock);

            task_join_handle
        };

        if let Some(task_join_handle) = task_join_handle {
            task_join_handle.await.ok();
        }

        log::info!(target: "staging_file_remover", "Staging file remover shut down.");
    }
}

async fn remove_expired_staging_files_task(
    mut stop_signal_receiver: tokio::sync::oneshot::Receiver<()>,
    period: Duration,
    expiration: Duration,
    staging_file_service: Arc<StagingFileService>,
) {
    let period = match period.to_std() {
        Ok(period) => period,
        Err(err) => {
            log::warn!(target: "staging_file_remover", err:err; "Failed to convert period to std duration. Defaulting to 1 hour.");
            std::time::Duration::new(3600, 0)
        }
    };

    loop {
        tokio::select! {
            _ = tokio::time::sleep(period) => {
                remove_expired_staging_files(expiration, &staging_file_service).await;
            }
            _ = &mut stop_signal_receiver => {
                break;
            }
        }
    }
}

async fn remove_expired_staging_files(
    expiration: Duration,
    staging_file_service: &StagingFileService,
) {
    log::info!(target: "staging_file_remover", expiration:%; "Removing expired staging files.");

    let result = staging_file_service
        .remove_expired_staging_files(expiration, 100)
        .await;

    match result {
        Ok((total_count, io_errs)) => {
            let io_failed_count = io_errs.len();
            let io_succeeded_count = total_count - io_failed_count;
            log::info!(target: "staging_file_remover", expiration:%, total_count, io_succeeded_count, io_failed_count, io_errs:?; "Removed expired staging files.");
        }
        Err(err) => {
            // failing to remove expired staging files is not a critical error
            log::warn!(target: "staging_file_remover", err:err; "Failed to remove expired staging files.");
        }
    }
}
