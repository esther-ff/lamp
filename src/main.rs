mod runtime;
mod task;

use log::{Level, Metadata, Record};

struct Logger;

static LOG: Logger = Logger;

pub fn init() -> Result<(), log::SetLoggerError> {
    log::set_logger(&LOG).map(|()| log::set_max_level(log::LevelFilter::Info))
}
impl log::Log for Logger {
    fn enabled(&self, m: &Metadata) -> bool {
        m.level() <= Level::Info
    }

    fn log(&self, r: &Record) {
        if self.enabled(r.metadata()) {
            println!(
                "[{}: {}] => {}",
                r.level(),
                r.file().map_or("undetected", |x| x),
                r.args()
            )
        }
    }

    fn flush(&self) {}
}

use runtime::Executor;

fn main() {
    init().unwrap();
    Executor::build();
    Executor::start(async {
        let h1 = Executor::spawn(async {
            std::thread::sleep(std::time::Duration::from_millis(500));
            println!("async task: hello after 500 ms!");
            0
        });
        let n = h1.await;
        println!("Value: {n}");

        println!("Guh");
    });
}
