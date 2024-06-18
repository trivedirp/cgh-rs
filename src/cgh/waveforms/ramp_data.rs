#[derive(Debug)]
pub struct RampData {
    pub period_s: f64,
    pub offset_s: f64,
    pub start_volts: f64,
    pub end_volts: f64,
    pub bidir_on: bool,
}

impl RampData {
    pub fn midpoint_s(&self) -> f64 { self.offset_s + self.period_s / 4.0 }
}
