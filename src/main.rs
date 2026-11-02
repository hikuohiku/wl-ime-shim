//! Runs a program behind a Wayland proxy that scales the rectangle passed to
//! `set_cursor_rectangle` on `zwp_text_input_v3` and `zwp_text_input_v1`.
//!
//! All other messages are forwarded unchanged.

use clap::Parser;
use error_reporter::Report;
use std::{
    io::{self, Write},
    process::{Command, ExitCode},
    rc::Rc,
};
use wl_proxy::{
    baseline::Baseline,
    object::{ConcreteObject, Object, ObjectCoreApi, ObjectRcUtils},
    protocols::{
        text_input_unstable_v1::{
            zwp_text_input_manager_v1::{ZwpTextInputManagerV1, ZwpTextInputManagerV1Handler},
            zwp_text_input_v1::{ZwpTextInputV1, ZwpTextInputV1Handler},
        },
        text_input_unstable_v3::{
            zwp_text_input_manager_v3::{ZwpTextInputManagerV3, ZwpTextInputManagerV3Handler},
            zwp_text_input_v3::{ZwpTextInputV3, ZwpTextInputV3Handler},
        },
        wayland::{
            wl_display::{WlDisplay, WlDisplayHandler},
            wl_registry::{WlRegistry, WlRegistryHandler},
            wl_seat::WlSeat,
        },
    },
    simple::{SimpleCommandExt, SimpleProxy},
};

#[derive(Parser)]
#[command(version, about)]
struct Args {
    /// Factor applied to the x, y, width and height of the rectangle
    #[arg(long, value_parser = parse_scale)]
    scale: f64,

    /// Print each rectangle to stderr before and after scaling
    #[arg(long)]
    log: bool,

    /// Program to run, followed by its arguments
    #[arg(required = true, trailing_var_arg = true)]
    command: Vec<String>,
}

fn parse_scale(value: &str) -> Result<f64, String> {
    match value.parse::<f64>() {
        Ok(scale) if scale.is_finite() && scale > 0.0 => Ok(scale),
        _ => Err("must be a positive number".to_owned()),
    }
}

type Rect = (i32, i32, i32, i32);

/// Multiplies every field by `scale`, rounding to the nearest integer.
fn scale_rect((x, y, width, height): Rect, scale: f64) -> Rect {
    // A float-to-int `as` cast saturates, so extreme values cannot overflow.
    let apply = |value: i32| (f64::from(value) * scale).round() as i32;
    (apply(x), apply(y), apply(width), apply(height))
}

fn main() -> ExitCode {
    let args = Args::parse();
    match run(&args) {
        Ok(never) => match never {},
        Err(message) => {
            eprintln!("wl-ime-shim: {message}");
            ExitCode::FAILURE
        }
    }
}

/// Returns only on failure: the proxy exits with the child's exit code when
/// the child terminates.
fn run(args: &Args) -> Result<std::convert::Infallible, String> {
    // Offer the client every protocol this version of wl-proxy knows, so that the
    // proxy changes as little as possible besides the rectangle.
    let server = SimpleProxy::new(Baseline::ALL_OF_THEM)
        .map_err(|e| format!("could not create the proxy: {}", Report::new(e)))?;
    Command::new(&args.command[0])
        .args(&args.command[1..])
        .with_wayland_display(server.display())
        .spawn_and_forward_exit_code()
        .map_err(|e| format!("could not run {}: {e}", args.command[0]))?;
    let config = Config {
        scale: args.scale,
        log: args.log,
    };
    let err = server.run(|| config);
    Err(format!("the proxy terminated: {}", Report::new(err)))
}

#[derive(Clone, Copy)]
struct Config {
    scale: f64,
    log: bool,
}

impl Config {
    fn apply(self, interface: &str, rect: Rect) -> Rect {
        let scaled = scale_rect(rect, self.scale);
        if self.log {
            // `eprintln!` panics when stderr is closed, which would drop the app's connection.
            let _ = writeln!(
                io::stderr(),
                "{interface}.set_cursor_rectangle {rect:?} -> {scaled:?}"
            );
        }
        scaled
    }
}

impl WlDisplayHandler for Config {
    fn handle_get_registry(&mut self, slf: &Rc<WlDisplay>, registry: &Rc<WlRegistry>) {
        registry.set_handler(*self);
        slf.send_get_registry(registry);
    }
}

impl WlRegistryHandler for Config {
    fn handle_bind(&mut self, slf: &Rc<WlRegistry>, name: u32, id: Rc<dyn Object>) {
        match id.interface() {
            ZwpTextInputManagerV3::INTERFACE => id
                .clone()
                .downcast::<ZwpTextInputManagerV3>()
                .set_handler(*self),
            ZwpTextInputManagerV1::INTERFACE => id
                .clone()
                .downcast::<ZwpTextInputManagerV1>()
                .set_handler(*self),
            _ => {}
        }
        slf.send_bind(name, id);
    }
}

impl ZwpTextInputManagerV3Handler for Config {
    fn handle_get_text_input(
        &mut self,
        slf: &Rc<ZwpTextInputManagerV3>,
        id: &Rc<ZwpTextInputV3>,
        seat: &Rc<WlSeat>,
    ) {
        id.set_handler(*self);
        slf.send_get_text_input(id, seat);
    }
}

impl ZwpTextInputManagerV1Handler for Config {
    fn handle_create_text_input(
        &mut self,
        slf: &Rc<ZwpTextInputManagerV1>,
        id: &Rc<ZwpTextInputV1>,
    ) {
        id.set_handler(*self);
        slf.send_create_text_input(id);
    }
}

impl ZwpTextInputV3Handler for Config {
    fn handle_set_cursor_rectangle(
        &mut self,
        slf: &Rc<ZwpTextInputV3>,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) {
        let (x, y, width, height) = self.apply("zwp_text_input_v3", (x, y, width, height));
        slf.send_set_cursor_rectangle(x, y, width, height);
    }
}

impl ZwpTextInputV1Handler for Config {
    fn handle_set_cursor_rectangle(
        &mut self,
        slf: &Rc<ZwpTextInputV1>,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) {
        let (x, y, width, height) = self.apply("zwp_text_input_v1", (x, y, width, height));
        slf.send_set_cursor_rectangle(x, y, width, height);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_is_well_formed() {
        Args::command().debug_assert();
    }

    #[test]
    fn command_keeps_its_own_flags() {
        let args = Args::parse_from(["bin", "--scale", "2", "app", "--log", "-x"]);
        assert_eq!(args.command, ["app", "--log", "-x"]);
        assert!(!args.log);
    }

    #[test]
    fn mistyped_option_is_a_usage_error() {
        assert!(Args::try_parse_from(["bin", "--scale", "2", "--lgo", "app"]).is_err());
    }

    #[test]
    fn scale_must_be_positive_and_finite() {
        assert_eq!(parse_scale("1.5"), Ok(1.5));
        for bad in ["0", "-1", "inf", "NaN", "two"] {
            assert!(parse_scale(bad).is_err(), "{bad} was accepted");
        }
    }

    #[test]
    fn rect_is_scaled_and_rounded() {
        assert_eq!(scale_rect((164, 52, 0, 10), 2.0), (328, 104, 0, 20));
        assert_eq!(scale_rect((3, 5, 1, 7), 1.5), (5, 8, 2, 11));
        assert_eq!(scale_rect((-4, 0, 0, 0), 0.5), (-2, 0, 0, 0));
    }

    #[test]
    fn rect_saturates_instead_of_overflowing() {
        assert_eq!(scale_rect((i32::MAX, 0, 0, 0), 2.0).0, i32::MAX);
    }
}
