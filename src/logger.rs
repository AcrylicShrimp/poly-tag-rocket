use env_logger::Env;

pub fn setup_logger() {
    let env = Env::new()
        .filter_or("LOG_LEVEL", "info")
        .write_style_or("LOG_STYLE", "auto");

    env_logger::init_from_env(env);

    log::info!("Logger initialized.");
}
