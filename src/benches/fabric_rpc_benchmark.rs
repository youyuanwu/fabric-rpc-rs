use criterion::{black_box, criterion_group, criterion_main, Criterion};
use fabric_rpc_rs::sys::Message;

pub fn criterion_benchmark(c: &mut Criterion) {
    let header = String::from("myheader");
    let body = String::from("mybody");
    let msg = Message::create(header.clone().into_bytes(), body.clone().into_bytes());
    c.bench_function("Bench_Message", |b| b.iter(|| {
        let _ = msg.clone();
    }));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
