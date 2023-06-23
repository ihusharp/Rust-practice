Summary
=======

### Chapter 1: Multiple Threads

Multiple threads can run concurrently within the same program and can be spawned at any time.

When the main thread ends, the entire program ends.

Data races are undefined behavior, which is fully prevented (in safe code) by Rust’s type system.

Data that is Send can be sent to other threads, and data that is Sync can be shared between threads.

Regular threads might run as long as the program does, and thus can only borrow 'static data such as statics and leaked allocations.

Reference counting (Arc) can be used to share ownership to make sure data lives as long as at least one thread is using it.

Scoped threads are useful to limit the lifetime of a thread to allow it to borrow non-'static data, such as local variables.

&T is a shared reference. &mut T is an exclusive reference. Regular types do not allow mutation through a shared reference.

Some types have interior mutability, thanks to UnsafeCell, which allows for mutation through shared references.

Cell and RefCell are the standard types for single-threaded interior mutability. Atomics, Mutex, and RwLock are their multi-threaded equivalents.

Cell and atomics only allow replacing the value as a whole, while RefCell, Mutex, and RwLock allow you to mutate the value directly by dynamically enforcing access rules.

Thread parking can be a convenient way to wait for some condition.

When a condition is about data protected by a Mutex, using a Condvar is more convenient, and can be more efficient, than thread parking.

### Chapter 2: Atomic Operations

Atomic operations are indivisible; they have either fully completed, or they haven’t happened yet.

Atomic operations in Rust are done through the atomic types in std::sync::atomic, such as AtomicI32.

Not all atomic types are available on all platforms.

The relative ordering of atomic operations is tricky when multiple variables are involved. More in Chapter 3.

Simple loads and stores are nice for very basic inter-thread communication, like stop flags and status reporting.

Lazy initialization can be done as a race, without causing a data race.

Fetch-and-modify operations allow for a small set of basic atomic modifications that are especially useful when multiple threads are modifying the same atomic variable.

Atomic addition and subtraction silently wrap around on overflow.

Compare-and-exchange operations are the most flexible and general, and a building block for making any other atomic operation.

A weak compare-and-exchange operation can be slightly more efficient.

### Chapter 3:memory-ordering

- There might not be a global consistent order of all atomic operations, as things can appear to happen in a different order from different threads.
- However, each individual atomic variable has its own *total modification order*, regardless of memory ordering, which all threads agree on.
- The order of operations is formally defined through *happens-before* relationships.
- Within a single thread, there is a happens-before relationship between every single operation.
- Spawning a thread happens-before everything the spawned thread does.
- Everything a thread does happens-before joining that thread.
- Unlocking a mutex happens-before locking that mutex again.
- Acquire-loading the value from a release-store establishes a happens-before relationship. This value may be modified by any number of fetch-and-modify and compare-and-exchange operations.
- A consume-load would be a lightweight version of an acquire-load, if it existed.
- Sequentially consistent ordering results in a globally consistent order of operations, but is almost never necessary and can make code review more complicated.
- Fences allow you to combine the memory ordering of multiple operations or apply a memory ordering conditionally.