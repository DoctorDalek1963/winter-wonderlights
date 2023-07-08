use chrono::{naive::NaiveDate, Datelike, NaiveDateTime};
use lazy_static::lazy_static;
use regex::Regex;
use std::{
    collections::HashMap,
    fs::{self, DirEntry},
    path::PathBuf,
};
use tokio::process::Command;
use tracing::{debug, error, instrument, trace};
use tracing_appender::{non_blocking, non_blocking::WorkerGuard, rolling};
use tracing_subscriber::{filter::LevelFilter, fmt::Layer, prelude::*, EnvFilter};
use tracing_unwrap::ResultExt;

/// The directory for the server's log files.
const LOG_DIR: &str = concat!(env!("DATA_DIR"), "/logs");

/// The common prefix for the server's log files.
const LOG_PREFIX: &str = "server.log";

/// Initialise a subscriber for tracing to log to `stdout` and a file.
pub fn init_tracing() -> WorkerGuard {
    let (appender, guard) = non_blocking(rolling::hourly(LOG_DIR, LOG_PREFIX));

    let subscriber = tracing_subscriber::registry()
        .with(
            Layer::new()
                .with_writer(appender)
                .with_ansi(false)
                .with_filter(
                    EnvFilter::builder()
                        .with_default_directive(LevelFilter::DEBUG.into())
                        .parse_lossy(""),
                ),
        )
        .with(
            Layer::new()
                .with_writer(std::io::stdout)
                .with_ansi(true)
                .with_filter(
                    EnvFilter::builder()
                        .with_default_directive(LevelFilter::INFO.into())
                        .from_env_lossy(),
                ),
        );

    tracing::subscriber::set_global_default(subscriber)
        .expect_or_log("Setting the global default for tracing should be okay");

    guard
}

/// Individually compress all log files older than the given number of hours using `gzip`.
#[instrument]
pub async fn zip_log_files_older_than_hours(hours: u64) {
    let now = chrono::offset::Local::now().naive_utc();

    // Look through everything in the log files folder and filter it down to just the log files and
    // parse their datetimes, and then filter down to only the log files which are older than the
    // given number of hours
    let log_files_older_than_hours: Vec<PathBuf> = fs::read_dir(LOG_DIR)
        .expect_or_log(&format!("Should be able to read entries in {LOG_DIR}"))
        .filter_map(|file_result| match file_result {
            Ok(dir_entry)
                if dir_entry
                    .file_type()
                    .is_ok_and(|filetype| filetype.is_file()) =>
            {
                dir_entry.file_name().to_str().and_then(
                    |name| -> Option<(DirEntry, NaiveDateTime)> {
                        Some((dir_entry, unzipped_file_to_naive_datetime(name)?))
                    },
                )
            }
            _ => None,
        })
        .filter_map(|(file, datetime)| {
            if (now - datetime).num_hours() > hours as i64 {
                Some(file.path())
            } else {
                None
            }
        })
        .collect();

    if log_files_older_than_hours.is_empty() {
        return;
    }

    debug!(?log_files_older_than_hours, "gzipping old log files");

    let mut gzip_command = Command::new("gzip")
        .args(log_files_older_than_hours)
        .spawn()
        .expect_or_log("Should be able to run `gzip` on old log files");
    gzip_command.wait().await.unwrap_or_log();

    if let Some(stderr) = gzip_command.stderr {
        error!(?stderr, "gzip command failed when zipping old log files");
    }

    debug!("Finished gzipping old log files");
}

/// Try to parse a [`NaiveDateTime`] from a filename of an unzipped hourly server log.
fn unzipped_file_to_naive_datetime(name: &str) -> Option<NaiveDateTime> {
    lazy_static! {
        /// A RegEx to match against the filenames of the server's log files and extract the date parts.
        static ref REGEX: Regex = Regex::new(&{
            let mut s = regex::escape(LOG_PREFIX);
            s.push_str(r"\.(\d{4})-(\d{2})-(\d{2})-(\d{2})$");
            s
        }).expect("Regex should compile successfully");
    }

    let captures = REGEX.captures(name)?;

    let year = captures.get(1)?.as_str().parse().ok()?;
    let month = captures.get(2)?.as_str().parse().ok()?;
    let day = captures.get(3)?.as_str().parse().ok()?;
    let hour = captures.get(4)?.as_str().parse().ok()?;

    Some(NaiveDate::from_ymd_opt(year, month, day)?.and_hms_opt(hour, 0, 0)?)
}

/// Compress log files older than the given number of days into a `.tgz` file for each day.
#[instrument]
pub async fn zip_log_files_older_than_days(days: u64) {
    let today = chrono::offset::Local::now().date_naive();

    // Look through everything in the log files folder and filter it down to just the log files and
    // parse their dates, and then filter down to only the log files which are older than the
    // given number of days
    let log_files_older_than_days: Vec<(NaiveDate, PathBuf)> = fs::read_dir(LOG_DIR)
        .expect_or_log(&format!("Should be able to read entries in {LOG_DIR}"))
        .filter_map(|file_result| match file_result {
            Ok(dir_entry)
                if dir_entry
                    .file_type()
                    .is_ok_and(|filetype| filetype.is_file()) =>
            {
                dir_entry
                    .file_name()
                    .to_str()
                    .and_then(|name| -> Option<(DirEntry, NaiveDate)> {
                        Some((dir_entry, gzipped_file_to_naive_date(name)?))
                    })
            }
            _ => None,
        })
        .filter_map(|(file, datetime)| {
            if (today - datetime).num_days() > days as i64 {
                Some((datetime, file.path()))
            } else {
                None
            }
        })
        .collect();

    if log_files_older_than_days.is_empty() {
        return;
    }

    trace!(?log_files_older_than_days);

    let filename_map: HashMap<NaiveDate, Vec<PathBuf>> = {
        let mut map: HashMap<NaiveDate, Vec<PathBuf>> = HashMap::new();
        for (date, path) in log_files_older_than_days.into_iter() {
            map.entry(date).or_insert(vec![]).push(path);
        }
        map
    };

    debug!(?filename_map, "tar zipping old log files");

    let mut fut_handles = vec![];

    for (date, files) in filename_map {
        fut_handles.push(tokio::spawn(async move {
            let mut gunzip_command = Command::new("gunzip")
                .args(&files)
                .spawn()
                .expect_or_log("Should be able to run `gunzip` on old log files");
            gunzip_command.wait().await.unwrap_or_log();

            if let Some(stderr) = gunzip_command.stderr {
                error!(
                    ?stderr,
                    "`gunzip` command failed when zipping old log files"
                );
            }

            let tgz_name = PathBuf::from(format!(
                "{LOG_DIR}/{LOG_PREFIX}.{}-{:0>2}-{:0>2}.tgz",
                date.year(),
                date.month(),
                date.day()
            ));

            let mut tar_command =
                Command::new("tar")
                    .arg("czf")
                    .arg(tgz_name)
                    .arg("--remove-files")
                    .args(files.into_iter().filter_map(|path| {
                        path.with_extension("").file_name().map(|x| x.to_owned())
                    }))
                    .current_dir(LOG_DIR)
                    .spawn()
                    .expect_or_log("Should be able to run `tar` on old log files");
            tar_command.wait().await.unwrap_or_log();

            if let Some(stderr) = tar_command.stderr {
                error!(?stderr, "`tar` command failed when zipping old log files");
            }
        }));
    }

    for handle in fut_handles {
        handle.await.expect_or_log("Future should not fail");
    }

    debug!("Finished tar zipping old log files");
}

/// Try to parse a [`NaiveDate`] from a filename of an gzipped hourly server log.
fn gzipped_file_to_naive_date(name: &str) -> Option<NaiveDate> {
    lazy_static! {
        /// A RegEx to match against the filenames of the server's log files and extract the date parts.
        static ref REGEX: Regex = Regex::new(&{
            let mut s = regex::escape(LOG_PREFIX);
            s.push_str(r"\.(\d{4})-(\d{2})-(\d{2})-\d{2}.gz$");
            s
        }).expect("Regex should compile successfully");
    }

    let captures = REGEX.captures(name)?;

    let year = captures.get(1)?.as_str().parse().ok()?;
    let month = captures.get(2)?.as_str().parse().ok()?;
    let day = captures.get(3)?.as_str().parse().ok()?;

    Some(NaiveDate::from_ymd_opt(year, month, day)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unzipped_file_to_naive_datetime_test() {
        fn naive_date_time(year: i32, month: u32, day: u32, hour: u32) -> NaiveDateTime {
            NaiveDate::from_ymd_opt(year, month, day)
                .unwrap()
                .and_hms_opt(hour, 0, 0)
                .unwrap()
        }

        assert_eq!(
            unzipped_file_to_naive_datetime(&format!("{LOG_PREFIX}.2023-07-07-12")),
            Some(naive_date_time(2023, 7, 7, 12))
        );
        assert_eq!(
            unzipped_file_to_naive_datetime(&format!("{LOG_PREFIX}.2023-07-07-13")),
            Some(naive_date_time(2023, 7, 7, 13))
        );
        assert_eq!(
            unzipped_file_to_naive_datetime(&format!("{LOG_PREFIX}.2023-07-07-14")),
            Some(naive_date_time(2023, 7, 7, 14))
        );
        assert_eq!(
            unzipped_file_to_naive_datetime(&format!("{LOG_PREFIX}.2023-07-07-15")),
            Some(naive_date_time(2023, 7, 7, 15))
        );
        assert_eq!(
            unzipped_file_to_naive_datetime(&format!("{LOG_PREFIX}.2023-07-07-16")),
            Some(naive_date_time(2023, 7, 7, 16))
        );
        assert_eq!(
            unzipped_file_to_naive_datetime(&format!("{LOG_PREFIX}.2023-07-07-30")),
            None,
        );
        assert_eq!(
            unzipped_file_to_naive_datetime(&format!("{LOG_PREFIX}.2023-07-13-12")),
            Some(naive_date_time(2023, 7, 13, 12))
        );
        assert_eq!(
            unzipped_file_to_naive_datetime(&format!("{LOG_PREFIX}.2023-13-07-12")),
            None
        );
    }

    #[test]
    fn gzipped_file_to_naive_date_test() {
        fn naive_date(year: i32, month: u32, day: u32) -> NaiveDate {
            NaiveDate::from_ymd_opt(year, month, day).unwrap()
        }

        assert_eq!(
            gzipped_file_to_naive_date(&format!("{LOG_PREFIX}.2023-07-07-12.gz")),
            Some(naive_date(2023, 7, 7))
        );

        assert_eq!(
            gzipped_file_to_naive_date(&format!("{LOG_PREFIX}.2023-07-07-12.gz")),
            Some(naive_date(2023, 7, 7))
        );
        assert_eq!(
            gzipped_file_to_naive_date(&format!("{LOG_PREFIX}.2023-07-07-13.gz")),
            Some(naive_date(2023, 7, 7))
        );
        assert_eq!(
            gzipped_file_to_naive_date(&format!("{LOG_PREFIX}.2023-07-07-14.gz")),
            Some(naive_date(2023, 7, 7))
        );
        assert_eq!(
            gzipped_file_to_naive_date(&format!("{LOG_PREFIX}.2023-07-07-15.gz")),
            Some(naive_date(2023, 7, 7))
        );
        assert_eq!(
            gzipped_file_to_naive_date(&format!("{LOG_PREFIX}.2023-07-07-16.gz")),
            Some(naive_date(2023, 7, 7))
        );
        assert_eq!(
            gzipped_file_to_naive_date(&format!("{LOG_PREFIX}.2023-07-07-30.gz")),
            Some(naive_date(2023, 7, 7))
        );
        assert_eq!(
            gzipped_file_to_naive_date(&format!("{LOG_PREFIX}.2023-07-13-12.gz")),
            Some(naive_date(2023, 7, 13))
        );
        assert_eq!(
            gzipped_file_to_naive_date(&format!("{LOG_PREFIX}.2023-13-07-12.gz")),
            None
        );
    }
}
