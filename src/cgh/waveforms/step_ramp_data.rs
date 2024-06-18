#[derive(Debug)]
pub struct StepRampData {
    pub start_volts: f64,
    pub step_volts: f64,
    pub end_volts: f64,
    pub step_s: f64,
    pub offset_s: f64,
}
