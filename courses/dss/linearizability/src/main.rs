/*
 * @Descripttion: 
 * @Author: HuSharp
 * @Date: 2022-09-08 14:16:19
 * @LastEditTime: 2022-09-08 14:16:26
 * @@Email: ihusharp@gmail.com
 */
fn main() {
    let child = parent.new(o!(
        "thread_id" => slog::FnValue(|| {
            format!("{:?}", std::thread::current().id())
        })
    ));
    // any use of the `child` logger will have `thread_id` as context.
    play(&child);
}