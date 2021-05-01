use actix::prelude::*;

use std::error;
use std::fmt;
use std::time::Duration;

use async_trait::async_trait;
use futures::executor::block_on;
use sqlx::Acquire;

use crate::db;
use crate::db::values::Timestamp;

/// A repeating collection of tasks.
pub struct Scheduled {
    /// A database pool used by this manager and the tasks.
    pool: db::Pool,

    /// The list of scheduled tasks.
    tasks: Vec<ScheduledTask>,

    /// The repeated task runner.
    runner: Option<SpawnHandle>,
}

/// A runnable task.
#[async_trait]
pub trait Task {
    /// The unique name of this task.
    fn name(&self) -> &str;

    /// Run this task.
    ///
    /// # Arguments
    /// *  `transaction` - The containing transaction.
    /// *  `timestamp` - The current timestamp.
    async fn run(
        &self,
        transaction: &mut db::Transaction,
        timestamp: Timestamp,
    ) -> Result<(), Error>;
}

/// A scheduled task.
///
/// The exact time within the interval when a task is actually run is undefined.
pub enum ScheduledTask {
    /// The task is run daily.
    Daily(Box<dyn Task>),

    /// The task has a custom interval.
    #[allow(unused)]
    Custom(Box<dyn Task>, Box<dyn Fn(Duration) -> String>, Duration),
}

/// An error yielded by a task.
#[derive(Debug)]
pub enum Error {
    /// A database error occcurred.
    DatabaseError(db::Error),
}

/// A collection of multiple task related errors.
#[derive(Debug)]
pub struct MultipleErrors(pub Vec<Error>);

impl Scheduled {
    /// The SQL used to check whether a scheduled task has run for a specific
    /// timestamp.
    const CHECK: &'static str = concat!(
        "\
        SELECT last_run \
        FROM ScheduledTasks \
        WHERE task = ",
        parameter!(task),
        " ",
        "AND last_run = ",
        parameter!(last_run),
    );

    /// The SQL used to update the last run timestamp of a scheduled task.
    const UPDATE: &'static str = concat!(
        "\
        INSERT INTO ScheduledTasks (task, last_run, time)
        VALUES (",
        parameter!(task),
        ", ",
        parameter!(last_run),
        ", ",
        parameter!(time),
        ")",
    );

    /// Creates a new scheduled task runner.
    ///
    /// # Arguments
    /// *  `pool` - The database connection pool.
    pub fn new(pool: db::Pool) -> Self {
        Self {
            pool,
            tasks: Vec::new(),
            runner: None,
        }
    }

    /// Schedules a new task for this repeated task runner.
    ///
    /// # Arguments
    /// *  `task` - The task to schedule.
    pub fn with(mut self, scheduled_task: ScheduledTask) -> Self {
        self.tasks.push(scheduled_task);
        self
    }

    /// Iterates over all scheduled tasks and runs those who have not been run.
    ///
    /// # Argumnets
    /// *  `timestamp` - The timestamp to use when checking whether a task
    ///    should be run.
    pub async fn run(
        &self,
        timestamp: Timestamp,
    ) -> Result<(), MultipleErrors> {
        let mut connection = self.pool.acquire().await?;

        let mut errors = None;

        for scheduled_task in self.tasks.iter() {
            if let Err(e) = self
                .check_and_run(scheduled_task, &mut connection, timestamp)
                .await
            {
                log::error!(
                    "Failed to run task {}: {}",
                    scheduled_task.task().name(),
                    e,
                );
                errors.get_or_insert_with(Vec::new).push(e)
            }
        }

        errors
            .map(|errors| Err(MultipleErrors(errors)))
            .unwrap_or_else(|| Ok(()))
    }

    /// Checks whether a task should be run, and in that case runs it and
    /// updates the database.
    ///
    /// This method will start a new transaction.
    ///
    /// # Arguments
    /// *  `connection` - The database connection to use.
    /// *  `timestamp` - The timestamp for which to run this method.
    /// *  `task` - The task to run.
    pub async fn check_and_run(
        &self,
        task: &ScheduledTask,
        connection: &mut db::Connection,
        timestamp: Timestamp,
    ) -> Result<(), Error> {
        let mut transaction = connection.begin().await?;
        if self.check(&task, &mut transaction, timestamp).await? {
            task.task().run(&mut transaction, timestamp).await?;
            self.update(&task, &mut transaction, timestamp).await?;
            transaction.commit().await?;
        }

        Ok(())
    }

    /// Checks whether a scheduled task should be run for a specific timestamp.
    ///
    /// This is performed by converting the timestamp to a string
    /// representation, taking the resolution into account, and then checking
    /// whether that timestamp exists for this task in the database.
    ///
    /// # Arguments
    /// *  `task` - The task to check.
    /// *  `transaction` - The database transaction.
    /// *  `timestamp` - The timestamp to check for.
    async fn check<'a>(
        &self,
        task: &ScheduledTask,
        transaction: &mut db::Transaction<'a>,
        timestamp: Timestamp,
    ) -> Result<bool, Error> {
        let name = task.task().name();
        let last_run = ScheduledTaskTimestamp(task, timestamp).to_string();
        Ok(sqlx::query(Self::CHECK)
            .bind(name)
            .bind(&last_run)
            .fetch_optional(transaction)
            .await?
            .is_none())
    }

    /// Updates the database with a new last run timestamp for a task.
    ///
    /// # Arguments
    /// *  `task` - The task to update.
    /// *  `transaction` - The database transaction.
    /// *  `timestamp` - The timestamp to update to.
    async fn update<'a>(
        &self,
        task: &ScheduledTask,
        transaction: &mut db::Transaction<'a>,
        timestamp: Timestamp,
    ) -> Result<(), db::Error> {
        let name = task.task().name();
        let last_run = ScheduledTaskTimestamp(task, timestamp).to_string();
        sqlx::query(Self::UPDATE)
            .bind(name)
            .bind(last_run)
            .bind(timestamp)
            .execute(transaction)
            .await
            .map(|_| ())
    }
}

impl Supervised for Scheduled {}

impl Actor for Scheduled {
    type Context = Context<Self>;

    fn started(&mut self, context: &mut Self::Context) {
        if let Some(interval) = self
            .tasks
            .iter()
            .map(|t| t.duration().clone())
            .max()
            .map(|i| i.mul_f32(0.05))
        {
            log::info!(
                "Starting scheduled tasks with interval {}h",
                interval.as_secs() as f32 / 3600.0
            );
            if let Err(e) = block_on(self.run(Timestamp::now())) {
                log::error!(
                    "Failed to execute scheduled tasks first time: {}",
                    e
                );
            }
            self.runner = Some(context.run_interval(interval, move |s, _| {
                log::info!("Executing scheduled tasks");
                if let Err(e) = block_on(s.run(Timestamp::now())) {
                    log::error!("Failed to execute scheduled tasks: {}", e);
                }
            }));
        } else {
            log::warn!("No scheduled tasks registered");
        }
    }
}

impl ScheduledTask {
    /// The approximate duration of the time between subsequent runs of this
    /// task.
    pub fn duration(&self) -> Duration {
        use ScheduledTask::*;
        match self {
            Daily(_) => Duration::from_secs(24 * 60 * 60),
            Custom(_, _, d) => *d,
        }
    }

    /// The scheduled task.
    pub fn task(&self) -> &Box<dyn Task> {
        use ScheduledTask::*;
        match self {
            Daily(t) | Custom(t, _, _) => t,
        }
    }
}

/// A timestamp for a specific scheduled task.
///
/// Timestamps have different resolutions for different schedules.
pub struct ScheduledTaskTimestamp<'a>(pub &'a ScheduledTask, pub Timestamp);

impl<'a> fmt::Display for ScheduledTaskTimestamp<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let ScheduledTaskTimestamp(schedule, timestamp) = self;

        use chrono::Datelike;
        use ScheduledTask::*;
        match schedule {
            Daily(_) => write!(
                f,
                "{:04}-{:02}-{:02}",
                timestamp.0.year(),
                timestamp.0.month(),
                timestamp.0.day(),
            ),
            Custom(_, fmt, d) => write!(f, "{}", fmt(*d)),
        }
    }
}

impl AsRef<Box<dyn Task>> for ScheduledTask {
    #[inline]
    fn as_ref(&self) -> &Box<dyn Task> {
        self.task()
    }
}

impl From<db::Error> for Error {
    #[inline]
    fn from(source: db::Error) -> Self {
        Self::DatabaseError(source)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use Error::*;
        match self {
            DatabaseError(e) => e.fmt(f),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        use Error::*;
        match self {
            DatabaseError(e) => Some(e),
        }
    }
}

impl From<db::Error> for MultipleErrors {
    #[inline]
    fn from(source: db::Error) -> Self {
        Self(vec![Error::DatabaseError(source)])
    }
}

impl fmt::Display for MultipleErrors {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for c in self.0.chunks(2) {
            if c.len() == 2 {
                write!(f, "{}, ", c[0])?;
            } else {
                write!(f, "{}", c[0])?;
            }
        }
        Ok(())
    }
}

impl error::Error for MultipleErrors {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        // There is no single source
        None
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::thread::sleep;
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::db::test_pool;

    use super::*;

    #[test]
    fn triggered() {
        {
            let counter = Arc::new(AtomicUsize::new(0));
            let duration = Duration::from_millis(200);

            let scounter = counter.clone();
            let sduration = duration.clone();
            std::thread::spawn(move || {
                let sys = System::new();
                sys.block_on(async move {
                    let _addr = Scheduled::new(test_pool().await)
                        .with(ScheduledTask::Custom(
                            Box::new(TestTask::new("test-task", scounter)),
                            Box::new(|t| {
                                format!(
                                    "test-{}",
                                    SystemTime::now()
                                        .duration_since(UNIX_EPOCH)
                                        .unwrap()
                                        .as_millis()
                                        / t.as_millis()
                                )
                            }),
                            sduration,
                        ))
                        .start();
                });
                sys.run().unwrap();
            });
            sleep(duration.mul_f32(2.0));

            assert!(counter.load(Ordering::Relaxed) >= 2);
        }
    }

    #[actix_rt::test]
    async fn run_simple() {
        let pool = test_pool().await;
        {
            let counter = Arc::new(AtomicUsize::new(0));
            let s = Scheduled::new(pool).with(ScheduledTask::Daily(Box::new(
                TestTask::new("test-task", counter.clone()),
            )));

            let timestamp = Timestamp::now();
            s.run(timestamp).await.unwrap();
            assert_eq!(counter.load(Ordering::Relaxed), 1);
        }
    }

    #[actix_rt::test]
    async fn run_multiple() {
        let pool = test_pool().await;
        {
            let counter1 = Arc::new(AtomicUsize::new(0));
            let counter2 = Arc::new(AtomicUsize::new(0));
            let s = Scheduled::new(pool)
                .with(ScheduledTask::Daily(Box::new(TestTask::new(
                    "test-task-1",
                    counter1.clone(),
                ))))
                .with(ScheduledTask::Daily(Box::new(TestTask::new(
                    "test-task-2",
                    counter2.clone(),
                ))));

            let timestamp = Timestamp::now();
            s.run(timestamp).await.unwrap();
            assert_eq!(counter1.load(Ordering::Relaxed), 1);
            assert_eq!(counter2.load(Ordering::Relaxed), 1);
        }
    }

    #[actix_rt::test]
    async fn run_concurrently() {
        let pool = test_pool().await;
        {
            let counter = Arc::new(AtomicUsize::new(0));
            let s = Scheduled::new(pool).with(ScheduledTask::Daily(Box::new(
                TestTask::new("test-task", counter.clone()),
            )));

            let timestamp = Timestamp::now();
            s.run(timestamp).await.unwrap();
            assert_eq!(counter.load(Ordering::Relaxed), 1);
            s.run(timestamp).await.unwrap();
            assert_eq!(counter.load(Ordering::Relaxed), 1);

            let timestamp = timestamp
                .0
                .checked_add_signed(chrono::Duration::days(1))
                .unwrap()
                .into();
            s.run(timestamp).await.unwrap();
            assert_eq!(counter.load(Ordering::Relaxed), 2);
        }
    }

    struct TestTask(&'static str, Arc<AtomicUsize>);

    impl TestTask {
        /// Creates a new simple task
        pub fn new(name: &'static str, counter: Arc<AtomicUsize>) -> Self {
            Self(name, counter)
        }
    }

    #[async_trait]
    impl Task for TestTask {
        fn name(&self) -> &'static str {
            self.0
        }

        async fn run(
            &self,
            _transaction: &mut db::Transaction,
            _timestamp: Timestamp,
        ) -> Result<(), Error> {
            self.1.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }
    }
}
