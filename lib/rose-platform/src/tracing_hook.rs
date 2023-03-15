use std::fs::File;

use eyre::Result;
use tracing_error::ErrorLayer;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt::format::FmtSpan, layer::SubscriberExt, EnvFilter, Layer};

pub fn enable() -> Result<()> {
    color_eyre::install()?;
    let fmt_layer =
        tracing_subscriber::fmt::Layer::default().with_filter(EnvFilter::from_default_env());
    let json_layer = tracing_subscriber::fmt::Layer::default()
        .json()
        .with_file(true)
        .with_level(true)
        .with_line_number(true)
        .with_thread_names(true)
        .with_thread_ids(true)
        .with_span_events(FmtSpan::ENTER | FmtSpan::EXIT)
        .with_writer(File::create("log.jsonl").unwrap());
    let registry = tracing_subscriber::registry()
        .with(ErrorLayer::default())
        .with(fmt_layer)
        .with(json_layer);
    #[cfg(feature = "tracy")]
    let registry = {
        let tracy_layer = tracing_tracy::TracyLayer::new();
        registry.with(tracy_layer)
    };
    registry.init();
    Ok(())
}
