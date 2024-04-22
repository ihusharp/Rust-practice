mod oneshot;
mod oneshot_borrow;
mod oneshot_borrow_blocking;
mod oneshot_non_copy;
mod oneshot_state;
mod vec_channel;

fn main() {
    vec_channel::channel();
    oneshot::oneshot_channel();
    oneshot_state::oneshot_channel();
    oneshot_non_copy::oneshot_channel();
    oneshot_borrow::oneshot_channel();
    oneshot_borrow_blocking::oneshot_channel();
}
