// PyTorch backend for LayoutPredictor (RT-DETR v2)
// Implements full model architecture using tch-rs
//
// Ported from transformers/models/rt_detr_v2/modeling_rt_detr_v2.py

pub mod decoder;
pub mod deformable_attention;
pub mod encoder;
pub mod model;
pub mod resnet;
pub mod transformer;
pub mod weights;
