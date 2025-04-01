mod cgh;
pub use cgh::*;

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;
    use crate::{Pulse, PulseData};
    #[test]
    fn test_chunk() {
        let data = PulseData { period_s: 10.0, on_s: 1.0, offset_s: 0.0 };
        let pulse = Pulse::new(data);
        let n_samples = 100_000_000;
        let samples_per_write = 20_000;
        assert_eq!(n_samples % samples_per_write, 0);
        let n_writes = n_samples / samples_per_write;
        let mut buf = vec![false; samples_per_write];
        let mut offset = 0;
        let t0 = Instant::now();
        for _ in 0..n_writes {
            pulse.chunk(&mut buf, offset);
            offset += samples_per_write;
        }
        let elapsed = t0.elapsed();
        let t_per_sample_us = elapsed.as_secs_f64() / n_samples as f64 * 1e6;
        println!("t_per_sample: {t_per_sample_us} us");
        let t_per_chunk_ms = elapsed.as_secs_f64() / n_writes as f64 * 1e3;
        println!("t_per_chunk: {t_per_chunk_ms} ms");
    }
    #[test]
    fn digitalor_test() {
        let data = PulseData { period_s: 100.0e-3, on_s: 0.1, offset_s: 0.0 };
        let data2 = PulseData { period_s: 100.0e-3, on_s: 0.2, offset_s: 1.0 };
        let data3 = PulseData { period_s: 100.0e-3, on_s: 0.3, offset_s: 2.0 };
        let data4 = PulseData { period_s: 100.0e-3, on_s: 0.4, offset_s: 3.0 };
        let pulse = LivePulseTrain::new(data, 1.0, 6.0, 0);
        let pulse2 = LivePulseTrain::new(data2, 1.0, 6.0, 0);
        let pulse3 = LivePulseTrain::new(data3, 1.0, 6.0, 0);
        let pulse4 = LivePulseTrain::new(data4, 1.0, 6.0, 0);
        let sources = [pulse, pulse2, pulse3, pulse4];
        let mut pulse = DigitalOr { sources };
        let n_samples = 10_000_000;
        let samples_per_write = 20_000;
        assert_eq!(n_samples % samples_per_write, 0);
        let n_writes = n_samples / samples_per_write;
        let mut buf = vec![false; samples_per_write];
        let mut offset = 0;
        let t0 = Instant::now();
        for _ in 0..n_writes {
            pulse.chunk(&mut buf, offset);
            offset += samples_per_write;
        }
        let elapsed = t0.elapsed();
        let t_per_sample_us = elapsed.as_secs_f64() / n_samples as f64 * 1e6;
        println!("t_per_sample: {t_per_sample_us} us");
        let t_per_chunk_ms = elapsed.as_secs_f64() / n_writes as f64 * 1e3;
        println!("t_per_sample: {t_per_chunk_ms} ms");
    }
}