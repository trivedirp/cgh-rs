#[derive(Debug)]
pub struct PulseData {
    pub period_s: f64,
    pub on_s: f64,
    pub offset_s: f64,
}

impl PulseData {
    pub fn start_at(period_s: f64, on_s: f64, start_s: f64) -> Self {
        let offset_s = start_s;
        Self { period_s, on_s, offset_s }
    }
    pub fn midpoint_at(period_s: f64, on_s: f64, midpoint_s: f64) -> Self {
        let offset_s = midpoint_s - on_s / 2.0;
        Self { period_s, on_s, offset_s }
    }
    pub fn end_at(period_s: f64, on_s: f64, end_s: f64) -> Self {
        let offset_s = end_s - on_s;
        Self { period_s, on_s, offset_s }
    }
    pub fn start_s(&self) -> f64 { self.offset_s }
    pub fn midpoint_s(&self) -> f64 { self.offset_s + self.on_s / 2.0 }
    pub fn end_s(&self) -> f64 { self.offset_s + self.on_s }
}
