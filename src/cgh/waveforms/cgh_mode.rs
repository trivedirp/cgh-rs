use clap::ArgEnum;

#[derive(Debug, Clone, Copy, PartialEq, ArgEnum)]
pub enum CghMode {
    Ondemand,
    SpimCalib,
    Stack488,
    Stack561,
    Stack2ch,
    CghCalib,
    CghInplane,
    CghFreerun,
    CghPrecalc,
}
