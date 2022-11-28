use lists::first::List;

fn main() {
    let mut list = List::new();
    list.push(1);
    list.push(2);
    println!("list = {:?}", list);
    println!("pop = {:?}", list.pop());
}
