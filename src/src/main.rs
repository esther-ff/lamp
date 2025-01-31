mod runtime;
mod task;

use runtime::Executor;

fn main() {
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
