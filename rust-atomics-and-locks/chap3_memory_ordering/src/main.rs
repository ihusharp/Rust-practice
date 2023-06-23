mod acquire_and_release;
mod releaxed;
mod seq;
mod  fence;

fn main() {
    // releaxed::releaxed();
    // acquire_and_release::acquire_and_release();
    // acquire_and_release::locking();
    // acquire_and_release::get_data();
    // seq::seq();
    fence::fence_check();
}
