use std::io::Write;

// set RUST_LOG=DEBUG to see debug logs
pub fn init() {
  env_logger::Builder::from_default_env()
    .format_timestamp_millis()
    .format(|buf, record| {
      writeln!(
        buf,
        "{} <{}> {}",
        buf.timestamp(),
        record.level(),
        record.args()
      )
    })
    .init();
}

pub fn log_debug(msg: &str) {
  log::debug!("{}", msg);
}

pub fn log_info(msg: &str) {
  log::info!("{}", msg);
}

pub fn log_warn(msg: &str) {
  log::warn!("{}", msg);
}

pub fn log_error(msg: &str) {
  log::error!("{}", msg);
}
