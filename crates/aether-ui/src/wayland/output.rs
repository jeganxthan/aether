use smithay_client_toolkit::output::{OutputHandler, OutputInfo, OutputState};
use wayland_client::{
    Connection, QueueHandle,
    protocol::wl_output::{Transform, WlOutput},
};

use crate::Monitor;
use crate::scale::ScaleFactor;

use super::{WaylandState, surface::View};

impl OutputHandler for WaylandState {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output
    }

    // also sent for the monitors that were already there when aether started
    fn new_output(&mut self, _: &Connection, _: &QueueHandle<Self>, output: WlOutput) {
        let Some(info) = self.output.info(&output) else {
            return;
        };

        let (output_names, origin_output) = self.output_layout(None);
        let mut monitor = describe(&info, self.scale_factor);
        monitor.outputs = output_names.clone();
        monitor.origin_output = origin_output.clone();

        let views = self.per_monitor.clone();

        for view in views {
            let view = View::Monitor(view, monitor.clone());

            self.open(view, Some(output.clone()));
        }

        self.update_monitor_outputs(&output_names, origin_output.as_deref());

        // a monitor plugged in while locked needs a lock screen too
        self.open_lock(output);
    }

    fn update_output(&mut self, _: &Connection, _: &QueueHandle<Self>, output: WlOutput) {
        let Some(info) = self.output.info(&output) else {
            return;
        };

        let (output_names, origin_output) = self.output_layout(None);
        let mut monitor = describe(&info, self.scale_factor);
        monitor.outputs = output_names.clone();
        monitor.origin_output = origin_output.clone();

        for window in &mut self.windows {
            if window.output.as_ref() != Some(&output) {
                continue;
            }

            if let View::Monitor(_, current) = &mut window.view {
                *current = monitor.clone();
            }

            window.request_frame();
        }

        self.update_monitor_outputs(&output_names, origin_output.as_deref());
    }

    // dropping a window destroys its layer surface
    fn output_destroyed(&mut self, _: &Connection, _: &QueueHandle<Self>, output: WlOutput) {
        self.windows
            .retain(|window| window.output.as_ref() != Some(&output));

        let (output_names, origin_output) = self.output_layout(Some(&output));
        self.update_monitor_outputs(&output_names, origin_output.as_deref());
    }
}

impl WaylandState {
    fn output_layout(&self, excluding: Option<&WlOutput>) -> (Vec<String>, Option<String>) {
        let mut names = Vec::new();
        let mut origin_output = None;

        for output in self
            .output
            .outputs()
            .filter(|output| excluding != Some(output))
        {
            let Some(info) = self.output.info(&output) else {
                continue;
            };
            let Some(name) = info.name else {
                continue;
            };

            if origin_output.is_none() && info.logical_position == Some((0, 0)) {
                origin_output = Some(name.clone());
            }
            names.push(name);
        }

        (names, origin_output)
    }

    fn update_monitor_outputs(&mut self, output_names: &[String], origin_output: Option<&str>) {
        for window in &mut self.windows {
            if let View::Monitor(_, monitor) = &mut window.view {
                monitor.outputs = output_names.to_vec();
                monitor.origin_output = origin_output.map(str::to_owned);
                window.request_frame();
            }
        }
    }
}

pub fn describe(info: &OutputInfo, scale_factor: ScaleFactor) -> Monitor {
    let name = info.name.clone().unwrap_or_default();

    let current = info.modes.iter().find(|mode| mode.current);

    // the logical size already has scaling and rotation applied, the mode does not
    let (width, height) = match (info.logical_size, current) {
        (Some((width, height)), _) => (width as f32, height as f32),
        (None, Some(mode)) => logical_mode_size(mode.dimensions, info.transform, info.scale_factor),
        (None, None) => (0.0, 0.0),
    };

    Monitor {
        name,

        width: scale_factor.logical(width).round() as u32,
        height: scale_factor.logical(height).round() as u32,
        x: info.logical_position.map_or(0, |(x, _)| x),
        y: info.logical_position.map_or(0, |(_, y)| y),
        outputs: info.name.iter().cloned().collect(),
        origin_output: (info.logical_position == Some((0, 0)))
            .then(|| info.name.clone())
            .flatten(),
    }
}

fn logical_mode_size(
    (width, height): (i32, i32),
    transform: Transform,
    output_scale: i32,
) -> (f32, f32) {
    // outputs without xdg-output only give us the physical mode
    let (width, height) = match transform {
        Transform::_90 | Transform::_270 | Transform::Flipped90 | Transform::Flipped270 => {
            (height, width)
        }
        _ => (width, height),
    };

    (
        width as f32 / output_scale as f32,
        height as f32 / output_scale as f32,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_fallback_accounts_for_output_scale_and_rotation_before_app_scale() {
        let factor = ScaleFactor::new(1.5);
        let (width, height) = logical_mode_size((1920, 1080), Transform::Normal, 2);
        assert_eq!(
            (factor.logical(width), factor.logical(height)),
            (640.0, 360.0)
        );

        for transform in [
            Transform::_90,
            Transform::_270,
            Transform::Flipped90,
            Transform::Flipped270,
        ] {
            let (width, height) = logical_mode_size((1920, 1080), transform, 2);
            assert_eq!(
                (factor.logical(width), factor.logical(height)),
                (360.0, 640.0)
            );
        }
    }
}
