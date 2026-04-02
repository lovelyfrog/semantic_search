use std::path::Path;

use log::{Level, LevelFilter, Record};
use log4rs::{
    Config,
    append::file::FileAppender,
    config::{Appender, Root},
    encode::{Encode, Write},
};
use std::sync::OnceLock;

static LOG4RS_HANDLE: OnceLock<log4rs::Handle> = OnceLock::new();

#[derive(Debug)]
struct ModulePatternEncoder;

impl Encode for ModulePatternEncoder {
    fn encode(&self, writer: &mut dyn Write, record: &Record) -> anyhow::Result<()> {
        let now = chrono::Local::now();
        let timestamp = now.format("%Y-%m-%d %H:%M:%S%.3f").to_string();

        let level = match record.level() {
            Level::Error => "ERROR",
            Level::Warn => "WARN",
            Level::Info => "INFO",
            Level::Debug => "DEBUG",
            Level::Trace => "TRACE",
        };

        let module_path = record.module_path().unwrap_or("unknown");

        if !module_path.starts_with("vnext_semantic_search") {
            return Ok(());
        }

        let line = record.line().unwrap_or(0);

        let formatted = format!(
            "[{}] [{}] [module = {}:{}] {}\n",
            timestamp,
            level,
            module_path,
            line,
            record.args()
        );

        writer.write_all(formatted.as_bytes())?;
        Ok(())
    }
}

fn level_filter_from_str(level: &str) -> LevelFilter {
    match level.to_lowercase().as_str() {
        "error" => LevelFilter::Error,
        "warn" => LevelFilter::Warn,
        "info" => LevelFilter::Info,
        "debug" => LevelFilter::Debug,
        "trace" => LevelFilter::Trace,
        _ => LevelFilter::Info,
    }
}

pub fn init_logger(log_path: &str, level: &str) -> anyhow::Result<()> {
    let level_filter = level_filter_from_str(level);

    if let Some(parent) = Path::new(log_path).parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| anyhow::anyhow!("failed to create log directory: {}", e))?;
    }

    let file_appender = FileAppender::builder()
        .encoder(Box::new(ModulePatternEncoder))
        .build(log_path)
        .map_err(|e| anyhow::anyhow!("failed to create log file: {}", e))?;

    let config = Config::builder()
        .appender(Appender::builder().build("file", Box::new(file_appender)))
        .build(Root::builder().appender("file").build(level_filter))
        .map_err(|e| anyhow::anyhow!("failed to build log config: {}", e))?;

    if let Some(handle) = LOG4RS_HANDLE.get() {
        handle.set_config(config);
        return Ok(());
    }

    let handle = match log4rs::init_config(config) {
        Ok(handle) => handle,
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("already initialized") || msg.contains("already been initialized") {
                return Err(anyhow::anyhow!("logger already initialized"));
            }
            return Err(anyhow::anyhow!("failed to init logger: {}", e));
        }
    };
    let _ = LOG4RS_HANDLE.set(handle);

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    /// End-to-end: `init_logger` + file appender + [`ModulePatternEncoder`].
    ///
    /// Only one test may call [`init_logger`] per process (`log` allows a single global logger).
    #[test]
    fn init_logger_writes_crate_logs_to_file() {
        let path =
            std::env::temp_dir().join(format!("vnext_init_logger_{}.log", uuid::Uuid::new_v4()));
        let _ = fs::remove_file(&path);

        if let Err(e) = init_logger(path.to_str().expect("utf8 path"), "info") {
            if e.to_string().contains("logger already initialized") {
                // Another test (or dependency) installed a global logger first.
                // This test can't reliably assert file output in that situation.
                return;
            }
            panic!("init_logger: {e}");
        }

        log::info!("integration probe");
        log::logger().flush();

        let contents = fs::read_to_string(&path).expect("read log file");
        assert!(
            contents.contains("integration probe"),
            "expected message in log, got: {contents:?}"
        );
        assert!(
            contents.contains("[INFO]"),
            "expected level tag, got: {contents:?}"
        );
        assert!(
            contents.contains("[module ="),
            "expected module line from encoder, got: {contents:?}"
        );
        assert!(
            contents.contains("vnext_semantic_search::common::logger::tests"),
            "expected crate module path in output, got: {contents:?}"
        );

        let _ = fs::remove_file(&path);
    }
}
