pub mod lifetime;
pub mod perform;
pub mod spinlock;

fn main() {
    // perform::perform();
    spinlock::spinlock();
}
