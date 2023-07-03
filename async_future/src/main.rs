use async_future::server;

/// 因为async/await就是通过Generator实现的，Generator是通过匿名结构体实现的。
/// 如果async函数中存在跨await的引用，会导致底层Generator存在跨yield的引用，
/// 那根据Generator生成的匿名结构体就会是一个自引用结构体！
/// 然后这个自引用结构体会impl Future，
/// 异步的Runtime在调用Future::poll()函数查询状态的时候，需要一个可变借用（即&mut Self）。
/// 如果这个&mut Self不包裹在Pin里面的话，
/// 开发者自己impl Future就会利用std::mem::swap()之类的函数move掉&mut Self！
/// 所以这就是Future的poll()必须要使用Pin<&mut Self>的原因。


#[async_std::main]
async fn main() {
    // println!("Test async_future::executor::exec_test()");
    // executor::exec_test();

    // println!("Test async_future::pin_test::pin_test()");
    // pin_test::pin_test();

    // println!("Test async_future::pin_test::unpin_future()");
    // pin_test::unpin_future();

    // server::server().await;
    server::test_handle_connection().await;
}