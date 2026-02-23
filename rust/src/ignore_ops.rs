#[derive(Debug, clap::ValueEnum, Clone, Copy)]
pub enum IgnoreOps {
    Boxes,
    Nothing,
}
