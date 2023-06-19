mod id_allocation;
mod progress_report;
mod progress_report_multiple_threads;
mod stop_flag;
mod synchronization;

fn main() {
    // stop_flag::stop_flag();
    // progress_report::progress_report();
    // synchronization::synchronization();
    // progress_report_multiple_threads::progress_report_multiple_threads();
    // progress_report_multiple_threads::statistics();
    // id_allocation::id_allocation();
    id_allocation::get_key();
    id_allocation::get_key();
}
