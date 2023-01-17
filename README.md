There are HuSharp's Rust learning resoures.

## Done

### [Tokio](https://tokio.rs/tokio/tutorial)

This tutorial will take you step by step through the process of building a [Redis](https://redis.io/) client and server. We will start with the basics of asynchronous programming with Rust and build up from there. We will implement a subset of Redis commands but will get a comprehensive tour of Tokio.

### [Hecto](https://www.flenker.blog/hecto/)

This is a series of blog posts that shows you how to build a text editor in Rust. It’s a re-implementation of [kilo](http://antirez.com/news/108) in Rust, as outlined in [this fantastic tutorial](https://viewsourcecode.org/snaptoken/kilo/index.html). Same as the original booklet, these blog posts guide you through all the steps to build a basic text editor, `hecto`.

### [Lists](https://rust-unofficial.github.io/too-many-lists/)

This is a series of how to implement a linked list in Rust. The answer honestly depends on what your requirements are, and it's obviously not super easy to answer the question on the spot. As such I've decided to write this book to comprehensively answer the question once and for all.

### [Writing an OS in Rust](https://os.phil-opp.com/)

This blog series creates a small operating system in the [Rust programming language](https://www.rust-lang.org/). Each post is a small tutorial and includes all needed code, so you can follow along if you like. The source code is also available in the corresponding [repository](https://github.com/phil-opp/blog_os).

### [Talent Plan](https://github.com/pingcap/talent-plan)

This series is core to TALENT-PLAN. It builds your understanding of Rust as a programming language and provides opportunities for you to practice with it.

- [TP 201: Practical Networked Applications in Rust](courses/rust/README.md). A series of projects that incrementally develop a single Rust project from the ground up into a high-performance, networked, parallel and asynchronous kv-store. Along the way various real-world Rust development subject matter are explored and discussed.

- [TP 202: Distributed Systems in Rust](courses/dss/README.md). Adapted from the [MIT 6.824](http://nil.csail.mit.edu/6.824/2017/index.html) distributed systems coursework, this course focuses on implementing important distributed algorithms, including the [Raft](https://raft.github.io/) consensus algorithm, and the [Percolator](https://storage.googleapis.com/pub-tools-public-publication-data/pdf/36726.pdf) distributed transaction protocol.

  

## WIP

### [Type Exercise in Rust](https://github.com/skyzh/type-exercise-in-rust)

This is a short lecture on how to use the Rust type system to build necessary components in a database system.

### [LSM in a Week](https://github.com/skyzh/mini-lsm)

Build a simple key-value storage engine in a week!

### [RisingLight Tutorial](https://risinglightdb.github.io/risinglight-tutorial/00-lets-build-a-database.html)

RisingLight 是一个 Rust 语言编写的单机分析型（OLAP）数据库系统。

在这个教程中，我们将会带领大家从零开始，一步一步地实现自己的数据库！ 从一个最简单的 SQL 解析器开始，逐步实现查询引擎、存储引擎、优化器和事务，最终能够运行工业级的 TPC-H 基准测试。

除了标准教科书上的内容以外，你还可以体验到业界最前沿的流式计算引擎 lol。
