//! A tiny module for specifying what shall be ignored during simulation

#[derive(Debug, clap::ValueEnum, Clone, Copy)]
pub enum IgnoreOps {
    Boxes,
    Nothing,
}
