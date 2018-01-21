extern crate time;

pub struct TimeKeeper {
    timestamp:u64
}
impl TimeKeeper {
    pub fn init() -> TimeKeeper {
        TimeKeeper {
            timestamp:time::precise_time_ns()
        }
    }
    pub fn println_set_mark(&mut self, message:&str) {
        let elapsed = (time::precise_time_ns()-self.timestamp) as f64;
        println!("{}: {:?}!", message, elapsed/1e9);
        self.timestamp = time::precise_time_ns();
    }
}