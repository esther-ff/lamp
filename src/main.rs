mod runtime;
mod task;

use runtime::Executor;

fn main() {
    Executor::build();
    Executor::start(async {
        let h1 = Executor::spawn(async { 0 });
        let n = h1.await;
        //println!("Value: {n}");

        println!("Guh");
    });
}
