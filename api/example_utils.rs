pub fn init_tracing() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("near_api=trace"));

    tracing_subscriber::fmt().with_env_filter(filter).init();
}
