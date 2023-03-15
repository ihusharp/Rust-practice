use std::thread;

fn f() {
    println!("Hello from another");

    let id = thread::current().id();
    println!("This my thread id {:?}", id);
}

fn main() {
    println!("Hello from the main thread.");

    let mut nums = vec![1, 2, 3];
    thread::scope(|s| {
        s.spawn(|| { 
            nums.push(1);
            println!("length: {}", nums.len());
        });
        s.spawn(|| { 
            for n in &nums {
                println!("{n}");
            }
        });
    });
}