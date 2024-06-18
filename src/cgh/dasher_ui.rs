use crate::{experiments::SharedExperimentState, pipelines::SharedPipelineState, SharedDasherState};
use cust::prelude::*;
use imgui::{Textures, Ui};
use imgui_glium_renderer::glium::backend::Facade;
use imgui_glium_renderer::Texture;
use imgui_utils::ImageWidget;
use npp::StreamContext;
use std::sync::{atomic::Ordering::Relaxed, Arc};

pub struct DasherUI {
    pub v_max: f32,
    image: ImageWidget,
    bg_image: ImageWidget,
    template_image: ImageWidget,
    update_interval: usize,
    // keys_down: Option<Vec<bool>>,
}

impl DasherUI {
    pub fn new<F>(gl_context: &F, textures: &mut Textures<Texture>, stream: Arc<Stream>, stream_context: StreamContext, roi_size: (usize, usize), bg_size: (usize, usize), update_interval: usize) -> Self
    where
        F: Facade,
    {
        let v_max = 4000.0;
        let image = ImageWidget::new(roi_size, gl_context, textures, stream.clone(), stream_context).unwrap();
        let bg_image = ImageWidget::new(bg_size, gl_context, textures, stream.clone(), stream_context).unwrap();
        let template_image = ImageWidget::new(roi_size, gl_context, textures, stream.clone(), stream_context).unwrap();
        // let keys_down = None;
        Self { v_max, image, bg_image, template_image, update_interval }
    }
    pub fn show(&mut self, ui: &Ui, experiment_state: Option<&SharedExperimentState>, pipeline_state: &SharedPipelineState, dasher_state: &SharedDasherState) {
        let camera_frame_index = dasher_state.camera_frame_index.load(Relaxed);
        let io = ui.io();
        let keys = &io.keys_down;
        // if self.keys_down.is_none() {
        //     self.keys_down = Some(vec![false; io.keys_down.len()]);
        // }
        // if let Some(keys_down) = self.keys_down.as_mut() {
        //     for i in 0..io.keys_down.len() {
        //         if keys_down[i] != io.keys_down[i] {
        //             println!("{i}");
        //         }
        //         keys_down[i] = io.keys_down[i];
        //     }
        // }
        let up = keys[568];
        let down = keys[564];
        let left = keys[546];
        let right = keys[549];
        // let up = keys[32];
        // let down = keys[28];
        // let left = keys[10];
        // let right = keys[13];
        let nudge_velocity = dasher_state.nudge_velocity.load();
        let x_nudge = if left {
            -nudge_velocity
        } else {
            if right {
                nudge_velocity
            } else {
                0
            }
        };
        let y_nudge = if up {
            -nudge_velocity
        } else {
            if down {
                nudge_velocity
            } else {
                0
            }
        };
        dasher_state.nudge.store((x_nudge, y_nudge));

        // let mut template_heading = 0u32;
        // Slider::new("template_heading", 0, 359).build(ui, &mut template_heading);
        let v_min = 0.0;
        ui.slider("v_max#ir", 1f32, 4095f32, &mut self.v_max);
        let mut correlation_threshold = pipeline_state.correlation_threshold.load();
        if ui.slider("C_th", 0.0f32, 1.0f32, &mut correlation_threshold) {
            pipeline_state.correlation_threshold.store(correlation_threshold);
        }
        if let Some(t_feedback_ms) = dasher_state.t_feedback_ms.as_ref() {
            let mut value = t_feedback_ms.load();
            if ui.slider("t_feedback_ms", 10.0f32, 100.0f32, &mut value) {
                t_feedback_ms.store(value);
            }
        }
        // let mut fish_scale = dasher_state.fish_scale.load();
        // if ui.slider("fish_scale", 0.0f32, 2.0f32, &mut fish_scale) {
        //     dasher_state.fish_scale.store(fish_scale);
        // }
        let mut nudge_velocity = dasher_state.nudge_velocity.load();
        if ui.slider("nudge_velocity", 1, 16, &mut nudge_velocity) {
            dasher_state.nudge_velocity.store(nudge_velocity);
        }
        let mut brain_offset_px = dasher_state.brain_offset_px.load();
        if ui.slider("brain_offset_px", 10.0f32, 100.0f32, &mut brain_offset_px) {
            dasher_state.brain_offset_px.store(brain_offset_px);
        }
        let track_prev = dasher_state.tracking_allowed.load(Relaxed);
        let mut track = track_prev;
        ui.checkbox("Track", &mut track);
        if track != track_prev {
            dasher_state.tracking_allowed.store(track, Relaxed);
        }
        ui.same_line();
        if ui.button("Update template") {
            pipeline_state.update_template.store(true, Relaxed);
        }
        ui.same_line();
        if ui.button("Reset template") {
            pipeline_state.reset_template.store(true, Relaxed);
        }
        ui.same_line();
        ui.text(format!("Correlation: {:.2}", dasher_state.correlation.load()));
        ui.same_line();
        ui.text(format!("Camera: {camera_frame_index}"));
        if let Some(experiment_state) = experiment_state {
            ui.same_line();
            ui.text(format!("Experiment: {}", experiment_state.experiment_frame_index.load(Relaxed)));
        }
        #[cfg(feature = "mechanical_tracking")]
        {
            ui.same_line();
            if ui.button("Dump stage") {
                dasher_state.dump.store(true, Relaxed);
            }
        }
        if let Some(experiment_state) = experiment_state {
            if let Some(valve_state) = experiment_state.valve_state.as_ref() {
                ui.same_line();
                ui.text(format!("Valves: {}", valve_state.load(Relaxed)));
            }
            if let Some(pwm_state) = experiment_state.pwm_state.as_ref() {
                ui.same_line();
                ui.text(format!("PWM: {}", pwm_state.load(Relaxed)));
            }
            if let Some(pwm_ch0_state) = experiment_state.pwm_ch0_duty_cycle.as_ref() {
                ui.same_line();
                ui.text(format!("PWM0: {}", pwm_ch0_state.load(Relaxed)));
            }
            if let Some(pwm_ch1_state) = experiment_state.pwm_ch1_duty_cycle.as_ref() {
                ui.same_line();
                ui.text(format!("PWM1: {}", pwm_ch1_state.load(Relaxed)));
            }
            if let Some(heat) = experiment_state.heat.as_ref() {
                ui.same_line();
                ui.text(format!("Heat: {:.2}", heat.load()));
            }
            if experiment_state.spatial {
                ui.same_line();
                if ui.button("Set left") {
                    dasher_state.set_loc_left.store(true, Relaxed);
                }
                ui.same_line();
                if ui.button("Set right") {
                    dasher_state.set_loc_right.store(true, Relaxed);
                }
            }
            let token = ui.begin_disabled(dasher_state.set_user_event.load(Relaxed));
            if ui.button("User event") {
                dasher_state.set_user_event.store(true, Relaxed);
            }
            token.end();
        }
        ui.same_line();
        ui.text(format!("Late: {}", dasher_state.late_count.load(Relaxed)));

        ui.same_line();
        let tracking = dasher_state.tracking.load(Relaxed);
        let on_time = dasher_state.on_time.load(Relaxed);
        let p = ui.cursor_pos();
        let p = [p[0] - ui.scroll_x(), p[1] - ui.scroll_y()];
        ui.invisible_button("Experiment", [10.0, 10.0]);
        ui.get_foreground_draw_list().add_rect([p[0], p[1] + 5.0], [p[0] + 10.0, p[1] + 15.0], if on_time { [1.0, 0.0, 0.0] } else { [1.0, 1.0, 1.0] }).filled(tracking).build();

        let mut display_subtracted = dasher_state.display_subtracted.load(Relaxed);
        if ui.checkbox("roi_sub", &mut display_subtracted) {
            dasher_state.display_subtracted.store(display_subtracted, Relaxed);
        }
        let (roi_width, roi_height) = pipeline_state.roi_size;
        let p0 = ui.cursor_pos();
        let p0 = [p0[0] - ui.scroll_x(), p0[1] - ui.scroll_y()];
        let scale = 2.0 / 3.0;
        self.image.update_from_gpu_i16(if display_subtracted { &pipeline_state.d_roi_sub } else { &pipeline_state.d_roi }, v_min, self.v_max);
        self.image.display(ui, scale);
        let fish_anchor_local = dasher_state.fish_anchor_local.load();
        let brain_local = dasher_state.brain_local.load();
        let center = [p0[0] + scale * fish_anchor_local.0 as f32, p0[1] + scale * fish_anchor_local.1 as f32];
        let brain = [p0[0] + scale * brain_local.0 as f32, p0[1] + scale * brain_local.1 as f32];
        ui.get_foreground_draw_list().add_circle(center, 5.0 * scale, [1.0, 0.0, 0.0]).filled(true).build();
        ui.get_foreground_draw_list().add_circle(brain, 5.0 * scale, [0.0, 1.0, 0.0]).filled(true).build();

        ui.get_foreground_draw_list().add_circle([p0[0] + roi_width as f32 / 2.0 * scale, p0[1] + roi_height as f32 / 2.0 * scale], 5.0 * scale * 0.7 / 2.0, [0.0, 0.0, 1.0]).filled(true).build();

        ui.same_line();
        let p1 = ui.cursor_pos();
        let p1 = [p1[0] - ui.scroll_x(), p1[1] - ui.scroll_y()];
        let scale = dasher_state.bg_scale.load().unwrap_or(1.0 / 6.0);
        if camera_frame_index % self.update_interval == 0 {
            self.bg_image.update_from_gpu_i16(&pipeline_state.d_background, v_min, self.v_max);
        }
        self.bg_image.display(ui, scale);
        if let Some(experiment_state) = experiment_state {
            if let Some(loc_left) = experiment_state.loc_left.load() {
                let top = [p1[0] + scale * loc_left as f32, p1[1]];
                let bottom = [p1[0] + scale * loc_left as f32, p1[1] + scale * (self.bg_image.size.1 as i16 - 1) as f32];
                ui.get_foreground_draw_list().add_line(top, bottom, [1.0, 0.0, 0.0]).build();
            }
            if let Some(loc_right) = experiment_state.loc_right.load() {
                let top = [p1[0] + scale * loc_right as f32, p1[1]];
                let bottom = [p1[0] + scale * loc_right as f32, p1[1] + scale * (self.bg_image.size.1 as i16 - 1) as f32];
                ui.get_foreground_draw_list().add_line(top, bottom, [0.0, 0.0, 1.0]).build();
            }
        }
        let (fish_anchor_x, fish_anchor_y) = dasher_state.fish_anchor.load();
        let center = [p1[0] + scale * fish_anchor_x as f32, p1[1] + scale * fish_anchor_y as f32];
        ui.get_foreground_draw_list().add_circle(center, 5.0 * scale * 0.7 / 2.0, [1.0, 0.0, 0.0]).filled(true).build();
        let (offset_x, offset_y) = dasher_state.roi_offset.load();
        let top_left = [p1[0] + scale * offset_x as f32, p1[1] + scale * offset_y as f32];
        let lower_right = [p1[0] + scale * (offset_x + self.image.size.0 as i16 - 1) as f32, p1[1] + scale * (offset_y + self.image.size.1 as i16 - 1) as f32];
        ui.get_foreground_draw_list().add_rect(top_left, lower_right, [1.0, 1.0, 0.0]).build();

        ui.same_line();
        let p2 = ui.cursor_pos();
        let p2 = [p2[0] - ui.scroll_x(), p2[1] - ui.scroll_y()];
        let scale = 2.0 / 3.0;
        if camera_frame_index % self.update_interval == 0 {
            self.template_image.update_from_gpu_f32(&pipeline_state.d_template, v_min, self.v_max);
        }
        self.template_image.display(ui, scale);

        let center = [p2[0] + scale * (roi_width / 2) as f32, p2[1] + scale * (roi_height / 2) as f32];
        let brain = [center[0] + brain_offset_px, center[1]];
        ui.get_foreground_draw_list().add_circle(center, scale, [1.0, 0.0, 0.0]).filled(true).build();
        ui.get_foreground_draw_list().add_circle(brain, scale, [0.0, 1.0, 0.0]).filled(true).build();
    }
}