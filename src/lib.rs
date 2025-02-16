mod reactor;
pub mod runtime;
mod task;

pub use reactor::io;
pub use runtime::Executor;

#[cfg(test)]
mod tests {
    use super::*;
    use log::{Level, Metadata, Record};

    struct Logger;

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

    fn log_init(logger: &'static Logger) -> Result<(), log::SetLoggerError> {
        log::set_logger(logger).map(|()| log::set_max_level(log::LevelFilter::Info))
    }

    fn test_tcp_server() -> std::thread::JoinHandle<u8> {
        use std::io::{Read, Write};
        use std::net::TcpListener;
        use std::thread;

        thread::spawn(|| {
            let mut buf: [u8; 1] = [0u8; 1];
            let listener = TcpListener::bind("127.0.0.1:8011").unwrap();

            let (mut stream, _) = listener.accept().unwrap();
            stream.write(&[1_u8]).unwrap();
            stream.read(&mut buf).unwrap();

            buf[0]
        })
    }

    #[test]
    fn delayed_task() {
        static LOG: Logger = Logger;
        log_init(&LOG).unwrap();

        let mut exec = Executor::new(2);
        exec.block_on(async {
            let h1 = Executor::spawn(async {
                std::thread::sleep(std::time::Duration::from_millis(500));
                println!("async task: hello after 500 ms!");
                0
            });

            let n = h1.await;
            println!("Value: {n}");

            println!("Guh");
        });

        exec.shutdown();
    }
    #[test]
    fn read_write_network() {
        use crate::io::{AsyncReadExt, AsyncWriteExt};
        //static LOG: Logger = Logger;

        //log_init(&LOG).unwrap();
        let handle = test_tcp_server();
        let mut exec = Executor::new(4);
        exec.block_on(async move {
            let mut stream = io::TcpStream::new("127.0.0.1:8011").unwrap();
            let mut buf: [u8; 1] = [0u8; 1];
            stream.read(&mut buf).await.unwrap();

            assert_eq!(buf[0], 1_u8);

            stream.write(&buf).await.unwrap();
            let value = handle.join().unwrap();

            assert_eq!(value, 1_u8);
        });

        exec.shutdown();
    }
}
