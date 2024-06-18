#[derive(Debug)]
pub struct SpiralData {
    pub period_s: f64,
    pub offset_s: f64,
    pub start_volts: f64,
    pub end_volts: f64,
    pub sine_on: bool,
}

impl SpiralData {
    pub fn midpoint_s(&self) -> f64 { self.offset_s + self.period_s / 4.0 }
}
