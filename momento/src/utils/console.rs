/// Write an `info` level `&str` to the console.
///
/// Since we reserve detailed log messages when the cli is `verbose`,
/// we funnel any other console messages here (thus to `stderr`).
///
/// # Examples
/// ```
/// output_info("Hello, world!");
/// //stderr> Hello, world!
/// ```
pub fn output_info(msg: &str) {
    eprintln!("{msg}");
}

/// The console print macro for non-verbose output.
///
/// This macro will print to the console with a `format!`
/// based argument list.
///
/// # Examples
///
/// ```
/// console_info!("Hello, world!");
/// console_info!("My name is {}", "Momento");
/// ```
macro_rules! console_info {
    () => ($crate::utils::console::output_info(""));
    ($($a:tt)*) => {{
        $crate::utils::console::output_info(&format!($($a)*));
    }};
}

pub(crate) use console_info;

// Write cache data `&str` to the console.
//
// Since we reserve detailed log messages when the cli is `verbose`,
// we funnel any other console messages for cache data here (thus to `stdout`).
//
// # Examples
// ```
// output_data("{\"key\": \"value\"}");
// //> {"key": "value"}
// ```
pub fn output_data(msg: &str) {
    println!("{msg}");
}

/// The console print macro for cache response data.
///
/// This macro will print to the console with a `format!`
/// based argument list.
///
/// # Examples
///
/// ```
/// console_data!("{\"key\": \"my-key\", \"value\": \"my-value\"}");
/// ```
macro_rules! console_data {
    () => ($crate::utils::console::output_data(""));
    ($($a:tt)*) => {{
        $crate::utils::console::output_data(&format!($($a)*));
    }};
}

pub(crate) use console_data;
