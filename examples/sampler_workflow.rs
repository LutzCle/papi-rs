extern crate papi;

fn main() {
    let papi = papi::Papi::init().unwrap();
    let sampler = papi::sampler::SamplerBuilder::new(&papi)
        .add_event("CPU_CLK_UNHALTED")
        .unwrap()
        .build()
        .start()
        .unwrap();

    // Do some work
    work();

    let sample = sampler.stop().unwrap();
    println!("CPU_CLK_UNHALTED: {}", sample);
}

fn work() {
    let collected: u32 = (0..100).map(|x| x * 2).filter(|x| x % 3 == 0).sum();

    println!("Summed up {}", collected);
}
