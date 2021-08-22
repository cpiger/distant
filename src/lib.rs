mod cli;
mod core;

pub use self::core::{data, net};
use log::error;

/// Main entrypoint into the program
pub fn run() {
    let opt = cli::Opt::load();
    let logger = init_logging(&opt.common);
    if let Err(x) = opt.subcommand.run(opt.common) {
        error!("Exiting due to error: {}", x);
        logger.flush();
        logger.shutdown();

        std::process::exit(x.to_i32());
    }
}

fn init_logging(opt: &cli::CommonOpt) -> flexi_logger::LoggerHandle {
    use flexi_logger::{FileSpec, LevelFilter, LogSpecification, Logger};
    let module = "distant";

    // Disable logging for everything but our binary, which is based on verbosity
    let mut builder = LogSpecification::builder();
    builder.default(LevelFilter::Off).module(
        module,
        match opt.verbose {
            0 => LevelFilter::Warn,
            1 => LevelFilter::Info,
            2 => LevelFilter::Debug,
            _ => LevelFilter::Trace,
        },
    );

    // If quiet, we suppress all output
    if opt.quiet {
        builder.module(module, LevelFilter::Off);
    }

    // Create our logger, but don't initialize yet
    let logger = Logger::with(builder.build()).format_for_files(flexi_logger::opt_format);

    // If provided, log to file instead of stderr
    let logger = if let Some(path) = opt.log_file.as_ref() {
        logger.log_to_file(FileSpec::try_from(path).expect("Failed to create log file spec"))
    } else {
        logger
    };

    logger.start().expect("Failed to initialize logger")
}
