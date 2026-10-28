// a screen the compositor shows windows on, handed to views made once per monitor
#[derive(Debug, Clone, PartialEq)]
pub struct Monitor {
    pub name: String,

    // dimensions in widget units, with the compositor and App scale factors already accounted for
    pub width: u32,
    pub height: u32,

    // logical desktop position reported by the compositor
    pub x: i32,
    pub y: i32,

    // names of the outputs currently connected to the compositor
    pub outputs: Vec<String>,

    // connected output at the logical desktop origin, when one is known
    pub origin_output: Option<String>,
}
