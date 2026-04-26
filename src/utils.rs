//! shared utilities: spinners, etc

/// Classic Braille Spinner -- should work anywhere
pub const SPINNER_BRAILLE: &[&str] = &[
    "⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"
];

/// Dots Variant Spinner -- should work anywhere
pub const SPINNER_DOTS: &[&str] = &[
    "⣾", "⣽", "⣻", "⢿", "⡿", "⣟", "⣯", "⣷"
];

/// Claude Code Spinner(macOS)
pub const SPINNER_CC_MACOS: &[&str] = &[
    ".", "+", "*", "*", "*", "*"
];

/// ASCII Spinner -- for terminal that
pub const SPINNER_ASCII: &[&str] = &[
    "-", "\\", "|", "/"
];

pub fn spinner_frame(tick: usize) -> &'static str {
  let spinner = SPINNER_BRAILLE;
  spinner[tick % spinner.len()]
}
