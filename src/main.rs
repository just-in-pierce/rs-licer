use rs_licer::{slice, SlicerConfig};
use std::env;

mod gui_iced;

fn print_help() {
    println!("rs-licer - STL slicer for resin 3D printing");
    println!();
    println!("USAGE:");
    println!("    rs-licer --gui");
    println!("    rs-licer [OPTIONS] <INPUT_STL> <OUTPUT_DIR>");
    println!();
    println!("ARGS:");
    println!("    <INPUT_STL>     Path to input STL file");
    println!("    <OUTPUT_DIR>    Directory to output slice images");
    println!();
    println!("OPTIONS:");
    println!("    --gui                      Launch GUI mode");
    println!("    -h, --help                 Print help information");
    println!("    -p, --pixel-size <UM>      Pixel size in micrometers (default: 33.3333)");
    println!("    -l, --layer-height <UM>    Layer height in micrometers (default: 20.0)");
    println!("    --zero-slice-position      Position model at slice zero (default: false)");
    println!("    --keep-above-zero          Keep slices above zero (default: delete below zero)");
    println!("    --keep-output-dir          Don't delete existing output directory (default: delete)");
    println!("    --open-output-dir          Open output directory when done (default: false)");
    println!();
    println!("EXAMPLES:");
    println!("    rs-licer model.stl output/");
    println!("    rs-licer -p 50 -l 25 model.stl slices/");
    println!("    rs-licer --zero-slice-position model.stl output/");
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    
    // Check for help flag
    if args.len() > 1 && (args[1] == "-h" || args[1] == "--help") {
        print_help();
        return Ok(());
    }
    
    // Check for GUI flag
    if args.len() > 1 && args[1] == "--gui" {
        // Run GUI mode
        gui_iced::run_gui()?;
        return Ok(());
    }
    
    // Parse CLI arguments for headless mode
    if args.len() < 3 {
        eprintln!("Error: Missing required arguments");
        eprintln!();
        print_help();
        std::process::exit(1);
    }
    
    let mut input_path = String::new();
    let mut output_dir = String::new();
    let mut pixel_size_um = 33.3333;
    let mut layer_height_um = 20.0;
    let mut zero_slice_position = false;
    let mut delete_below_zero = true;
    let mut delete_output_dir = true;
    let mut open_output_dir = false;
    
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-p" | "--pixel-size" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("Error: --pixel-size requires a value");
                    std::process::exit(1);
                }
                pixel_size_um = args[i].parse().unwrap_or_else(|_| {
                    eprintln!("Error: Invalid pixel size value");
                    std::process::exit(1);
                });
            }
            "-l" | "--layer-height" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("Error: --layer-height requires a value");
                    std::process::exit(1);
                }
                layer_height_um = args[i].parse().unwrap_or_else(|_| {
                    eprintln!("Error: Invalid layer height value");
                    std::process::exit(1);
                });
            }
            "--zero-slice-position" => {
                zero_slice_position = true;
            }
            "--keep-above-zero" => {
                delete_below_zero = false;
            }
            "--keep-output-dir" => {
                delete_output_dir = false;
            }
            "--open-output-dir" => {
                open_output_dir = true;
            }
            arg if !arg.starts_with('-') => {
                if input_path.is_empty() {
                    input_path = arg.to_string();
                } else if output_dir.is_empty() {
                    output_dir = arg.to_string();
                } else {
                    eprintln!("Error: Unexpected argument: {}", arg);
                    std::process::exit(1);
                }
            }
            unknown => {
                eprintln!("Error: Unknown option: {}", unknown);
                eprintln!();
                print_help();
                std::process::exit(1);
            }
        }
        i += 1;
    }
    
    if input_path.is_empty() || output_dir.is_empty() {
        eprintln!("Error: Missing required arguments <INPUT_STL> and <OUTPUT_DIR>");
        eprintln!();
        print_help();
        std::process::exit(1);
    }
    
    let config = SlicerConfig {
        input_path,
        output_dir,
        pixel_size_um,
        layer_height_um,
        zero_slice_position,
        delete_below_zero,
        delete_output_dir,
        open_output_dir,
    };

    slice(config);
    Ok(())
}

// Example configuration for development/testing
#[allow(dead_code)]
fn example_config() -> SlicerConfig {
    SlicerConfig {
        input_path: "example.stl".to_string(),
        output_dir: "slices".to_string(),
        pixel_size_um: 33.3333,
        layer_height_um: 20.0,
        zero_slice_position: false,
        delete_below_zero: true,
        delete_output_dir: true,
        open_output_dir: false,
    }
}
