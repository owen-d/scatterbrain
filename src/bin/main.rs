//! Binary entrypoint for the scatterbrain tool

use scatterbrain::utils;

fn main() {
    println!("{}", scatterbrain::hello_library());
    println!("Calculation result: {}", utils::calculate_something(21));
    println!("Scatterbrain binary is running!");
}
