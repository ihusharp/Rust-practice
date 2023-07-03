use std::thread;
use crate::future::TimerFuture;

use {
    futures::{
        future::{BoxFuture, FutureExt},
        task::{waker_ref, ArcWake},
    },
    std::{
        future::Future,
        sync::mpsc::{sync_channel, Receiver, SyncSender},
        sync::{Arc, Mutex},
        task::Context,
        time::Duration,
    },
};

struct Spawner {
    task_sender: SyncSender<Arc<Task>>,
}

impl Spawner {
    fn spawn(&self, future: impl Future<Output = ()> + 'static + Send) {
        let task = Arc::new(Task {
            future: Mutex::new(Some(future.boxed())),
            task_sender: self.task_sender.clone(),
        });
        self.task_sender.send(task).expect("queue is full");
    }
}

struct Executer {
    ready_queue: Receiver<Arc<Task>>,
}

impl Executer {
    fn run(&self) {
        while let Ok(task) = self.ready_queue.recv() {
            println!("run: {:?}", thread::current().id());
            let mut future = task.future.lock().unwrap();
            // Check future is some
            if let Some(mut f) = future.take() {
                let waker = waker_ref(&task);
                let context = &mut Context::from_waker(&*waker);
                println!("run: poll: {:?}", thread::current().id());
                if f.as_mut().poll(context).is_pending() {
                    println!("run: pending: {:?}", thread::current().id());
                    // Pending for next poll
                    *future = Some(f);
                }
            }
        }
    }
}

struct Task {
    future: Mutex<Option<BoxFuture<'static, ()>>>,
    task_sender: SyncSender<Arc<Task>>,
}

impl ArcWake for Task {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        println!("wake_by_ref: {:?}", thread::current().id());
        arc_self
            .task_sender
            .send(arc_self.clone())
            .expect("queue is full");
    }
}

fn new_executer_and_spawner() -> (Executer, Spawner) {
    let (task_sender, ready_queue) = sync_channel(100);
    (Executer { ready_queue }, Spawner { task_sender })
}

pub fn exec_test() {
    let (executer, spawner) = new_executer_and_spawner();
    spawner.spawn(async {
        println!("start!");
        TimerFuture::new(Duration::from_secs(2)).await;
        println!("end!");
    });

    drop(spawner);
    executer.run();
}