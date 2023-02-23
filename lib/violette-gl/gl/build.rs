use std::{env, io::BufWriter, path::PathBuf, fs::File};

use gl_generator::{Registry, DebugStructGenerator, StructGenerator};

fn main() {
    let dest = env::var("OUT_DIR").unwrap();
    let mut file = BufWriter::new(File::create(PathBuf::from(dest).join("bindings.rs")).unwrap());

    Registry::new(gl_generator::Api::Gl, (4, 5), gl_generator::Profile::Core, gl_generator::Fallbacks::All, ["KHR_debug", "ARB_debug_output", "GL_EXT_debug_label"])
        .write_bindings(StructGenerator, &mut file)
        .unwrap();
}