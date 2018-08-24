extern crate clap;
extern crate heck;
extern crate regex;

use clap::{Arg, App};
use std::fs as fs;
use std::path::{Path, PathBuf};
use heck::ShoutySnakeCase;
use heck::CamelCase;

fn append_if_non_empty(input: &str, tail: &str) -> String {
    if input.chars().count() > 0 {
        format!("{}{}", input, tail)
    } else {
        String::from("")
    }
}

fn prepend_if_non_empty(input: &str, head: &str) -> String {
    if input.chars().count() > 0 {
        format!("{}{}", head, input)
    } else {
        String::from("")
    }
}

fn  process_case_camel_to_snake(source_code: String, main_label: &str, module_signifier: &str) -> String {
    let module_signifier = append_if_non_empty(module_signifier, "::");

    let re_string = format!(r"(?x){}{}::([a-z A-Z 0-9]*)", module_signifier, main_label);
    let re = regex::Regex::new(&re_string).unwrap();

    println!("Fixing {}", main_label);

    re
        .replace_all(&source_code,
                     |caps: &regex::Captures| {
                         format!("{}{}::{}",
                                 module_signifier,
                                 main_label,
                                 &caps[1].to_shouty_snake_case())
                     })
        .into_owned()
}

fn process_case_bit_flags(source_code: String, main_label: &str, module_signifier: &str, extra_tail: &str, extra_head: &str) -> String {
    let module_signifier = append_if_non_empty(module_signifier, "::");
    let extra_head_with_underscore = append_if_non_empty(extra_head.clone(), "_");
    let extra_tail_with_underscore = prepend_if_non_empty(extra_tail.clone(), "_");

    let re_string = format!(r"(?x){}{}_([A-Z 0-9 _]*)_BIT{}\b", module_signifier, main_label, extra_tail_with_underscore);
    let re = regex::Regex::new(&re_string).unwrap();

    println!("Fixing {}", main_label);

    re
        .replace_all(&source_code,
                     |caps: &regex::Captures| {
                         format!("{}{}Flags{}::{}{}",
                                 module_signifier,
                                 main_label.to_camel_case(),
                                 extra_tail,
                                 extra_head_with_underscore,
                                 &caps[1])
                     })
        .into_owned()
}

fn fix_snake_case_oddities(source_code: String) -> String {
    let re = regex::Regex::new(r"TYPE(\d)D").unwrap();

    println!("Fixing snake case oddities");

    re
        .replace_all(&source_code, |caps: &regex::Captures| format!("TYPE_{}D", &caps[1]))
        .into_owned()
}

fn process_surface_extension_name_referrals(source_code: String) -> String {
    let re = regex::Regex::new("(?x)vk::VK_KHR_([A-Z _ 0-9]*)_EXTENSION_NAME").unwrap();

    println!("Fixing surface extension name referrals");
    re.replace_all(&source_code, |caps: &regex::Captures| format!("ext::{}::name().to_str().unwrap()", &caps[1].to_camel_case())).into_owned()
}

fn process_src_file(file_path: &Path) {
    println!("-------------");
    println!("{}", file_path.to_string_lossy());

    let mut source_code = fs::read_to_string(file_path).unwrap();

    let case_camel_to_snake = vec!["SubpassContents", "StructureType", "PipelineBindPoint", "ComponentSwizzle",
             "IndexType", "ImageLayout", "AttachmentLoadOp", "SamplerAddressMode", "BorderColor",
             "PrimitiveTopology", "PolygonMode", "CompareOp", "FrontFace", "StencilOp", "BlendOp",
             "BlendFactor", "CommandBufferLevel", "ImageViewType", "VertexInputRate", "DynamicState", "SharingMode", "ImageLayout", "Result", "QueryType",
             "PhysicalDeviceType", "Format", "ImageType", "ColorSpaceKHR", "PresentModeKHR", "LogicOp", "AttachmentStoreOp",
    ];

    for main_label in case_camel_to_snake.into_iter() {
        source_code = process_case_camel_to_snake(source_code, main_label, "vk");
    }

    let case_bit_flags_no_tail =
        vec!["COMMAND_BUFFER_RESET", "IMAGE_ASPECT", "QUERY_CONTROL", "SHADER_STAGE",
             "COMMAND_POOL_CREATE", "PIPELINE_CREATE", "IMAGE_CREATE",
             "FENCE_CREATE", "SURFACE_TRANSFORM", "QUEUE", "MEMORY_PROPERTY", "CULL_MODE"];

    for main_label in case_bit_flags_no_tail.into_iter() {
        source_code = process_case_bit_flags(source_code, main_label, "", "", "");
    }

    source_code = process_surface_extension_name_referrals(source_code);

    // special cases
    println!("Fixing special cases");
    source_code = source_code.replace("vk::types", "vk");
    source_code = source_code.replace("VK_", "");
    source_code = source_code.replace("vk::SwapchainFn", "vk::KhrSwapchainFn");
    source_code = source_code.replace("depth: vk::ClearDepthStencilValue", "depth_stencil: vk::ClearDepthStencilValue");
    source_code = source_code.replace("depth: conv::", "depth_stencil: conv::");
    source_code = source_code.replace("typ: conv::", "ty: conv::");
    source_code = process_case_bit_flags(source_code, "COMPOSITE_ALPHA", "vk", "KHR", "");
    source_code = process_case_bit_flags(source_code, "SURFACE_TRANSFORM", "vk", "KHR", "");
    source_code = process_case_bit_flags(source_code, "SAMPLE_COUNT", "vk", "", "TYPE");
    source_code = process_case_bit_flags(source_code, "DEBUG_REPORT", "vk", "EXT", "");
    source_code = process_case_camel_to_snake(source_code, "Dt", "");

    let re = regex::Regex::new(r"vk::CULL_MODE_([A-Z]*)").unwrap();
    source_code = re.replace_all(&source_code, |caps: &regex::Captures| format!("vk::CullModeFlags::{}", &caps[1])).into_owned();

    source_code = source_code.replace("use ash::vk::Result::*;", "");
    let re = regex::Regex::new(r"(?x)Error([a-z A-Z]*)\s=>\s").unwrap();
    source_code = re.replace_all(&source_code, |caps: &regex::Captures| format!("vk::Result::ERROR_{} => ", &caps[1].to_shouty_snake_case())).into_owned();

    source_code = source_code.replace("use vk::ImageType::*;", "");
    let re = regex::Regex::new(r"(?x)\(([a-z A-Z 0-9]*),\s([a-z A-Z 0-9]*)\)([\s a-z A-Z _]*)=>\s").unwrap();
    source_code = re.replace_all(&source_code, |caps: &regex::Captures| format!("({}, {}){}=> ", &caps[1].to_shouty_snake_case(), &caps[2], &caps[3])).into_owned();

    // fix snake case oddities
    let re = regex::Regex::new(r"limits.max_image_dimension(\d)d").unwrap();
    source_code = re.replace_all(&source_code, |caps: &regex::Captures| format!("limits.max_image_dimension{}_d", &caps[1])).into_owned();

    source_code = source_code.replace("pso::Descriptor::Image(VIEW, layout)", "pso::Descriptor::Image(view, layout)");
    source_code = source_code.replace("com::AttachmentClear::Color(INDEX, cv)", "com::AttachmentClear::Color(index, cv)");
    source_code = fix_snake_case_oddities(source_code);

    fs::write(file_path, source_code.into_bytes()).expect("Error writing source code back into file!");
}

fn main() {
    let args = App::new("ash-integrator")
        .version("0.1")
        .author("Brian Merchant <bhmerchant^gm4il-dot-com>")
        .about("refactor gfx-rs/backend/vulkan to use the newest version of ash")
        .arg(Arg::with_name("GFX-RS-DIR")
            .help("gfx-rs directory.")
            .required(true)
            .index(1))
        .get_matches();

    let gfx_vulkan_dir: PathBuf =
        [args.value_of("GFX-RS-DIR").unwrap(), "src", "backend", "vulkan", "src"]
            .iter()
            .collect();

    let dir_contents = fs::read_dir(gfx_vulkan_dir.as_path())
        .expect("Could not read source files.")
        .into_iter()
        .flat_map(|entry| { entry.into_iter() })
        .collect::<Vec<fs::DirEntry>>();

    for entry in dir_contents.into_iter() {
        if entry.file_type().unwrap().is_file() {
            match entry.path().extension() {
                Some(ext) => {
                    if ext.to_string_lossy() == "rs" {
                        process_src_file(entry.path().as_path());
                    }
                },
                _ => {},
            }
        }
    }
}
