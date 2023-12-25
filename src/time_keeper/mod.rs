extern crate time;

pub struct TimeKeeper {
    timestamp:std::time::Instant
}
impl TimeKeeper {
    pub fn init() -> TimeKeeper {
        TimeKeeper {
            timestamp:std::time::Instant::now()
        }
    }
    pub fn println_set_mark(&mut self, message:&str) {
        let elapsed = self.timestamp.elapsed();
        println!("{}: {:?}!", message, (elapsed.as_nanos() as f64)/1e9);
        self.timestamp = std::time::Instant::now();
    }
}