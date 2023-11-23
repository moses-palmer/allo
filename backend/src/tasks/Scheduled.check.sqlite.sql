SELECT last_run
FROM ScheduledTasks
WHERE task = ? AND last_run = ?
