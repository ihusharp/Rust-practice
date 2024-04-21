mod fence;
mod loom_check;
mod release_and_acquire;
mod releaxed;
mod seq;

fn main() {
    // releaxed::releaxed();
    // releaxed::check_a_b();
    // release_and_acquire::release_and_acquire1();
    // release_and_acquire::release_and_acquire2();
    release_and_acquire::locking();
    // release_and_acquire::get_data();
    // seq::seq();
    // fence::fence_check();
}
