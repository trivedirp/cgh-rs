pub const DO_SAMPLE_CLK: f64 = 10e6;
pub const AO_SAMPLE_CLK: f64 = 1e6;

pub fn do_clk(t_s: f64) -> usize {
    (t_s * DO_SAMPLE_CLK).round() as usize
}

pub fn ao_clk(t_s: f64) -> usize {
    (t_s * AO_SAMPLE_CLK).round() as usize
}
