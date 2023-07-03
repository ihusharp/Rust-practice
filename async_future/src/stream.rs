use std::pin::Pin;

use futures::{channel::mpsc, SinkExt, StreamExt, Stream, io, TryStreamExt};

#[allow(dead_code)]
async fn send_recv() {
    const BUFFER_SIZE: usize = 10;
    let (mut tx, mut rx) = mpsc::channel::<i32>(BUFFER_SIZE);

    tx.send(1).await.unwrap();
    tx.send(2).await.unwrap();
    drop(tx);

    // `StreamExt::next` 类似于 `Iterator::next`, 但是前者返回的不是值，而是一个 `Future<Output = Option<T>>`，
    // 因此还需要使用`.await`来获取具体的值
    assert_eq!(Some(1), rx.next().await);
    assert_eq!(Some(2), rx.next().await);
    assert_eq!(None, rx.next().await);
}

// all about next
#[allow(dead_code)]
async fn sum_with_next(mut stream: Pin<&mut dyn Stream<Item = i32>>) -> i32 {
    let mut sum = 0;
    while let Some(value) = stream.next().await {
        sum += value;
    }
    sum
}

#[allow(dead_code)]
async fn sum_with_try_next(
    mut stream: Pin<&mut dyn Stream<Item = Result<i32, io::Error>>>,
) -> Result<i32, io::Error> {
    let mut sum = 0;
    while let Some(item) = stream.try_next().await? {
        sum += item;
    }
    Ok(sum)
}

#[allow(dead_code)]
async fn jump_around(
    stream: Pin<&mut dyn Stream<Item = Result<u8, io::Error>>>,
) -> Result<(), io::Error> {
    const MAX_CONCURRENT_JUMPERS: usize = 100;

    stream.try_for_each_concurrent(MAX_CONCURRENT_JUMPERS, |_num| async move {
        // jump_n_times(num).await?;
        // report_n_jumps(num).await?;
        Ok(())
    }).await?;

    Ok(())
}