#![cfg(feature = "pytorch")]
use tch::{Kind, Tensor};

#[test]
fn test_broadcast_5d_with_2d() {
    let a = Tensor::randn([1, 300, 8, 12, 2], (Kind::Float, tch::Device::Cpu));
    let b = Tensor::randn([12, 1], (Kind::Float, tch::Device::Cpu));

    println!("a shape: {:?}", a.size());
    println!("b shape: {:?}", b.size());

    let c = &a * &b;
    println!("result shape: {:?}", c.size());

    assert_eq!(c.size(), vec![1, 300, 8, 12, 2]);
    println!("Broadcasting succeeded!");
}
